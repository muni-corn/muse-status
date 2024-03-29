use crate::{
    client::ClientMsg,
    config::Config,
    errors::*,
    format::{
        self,
        blocks::{output::BlockOutput, Block, BlockOutputMsg},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::thread::JoinHandle;
use std::{
    io::BufRead,
    io::Write,
    net::{TcpListener, TcpStream},
};

type BlockVec = Vec<Box<dyn Block>>;
type BlockOutputs = HashMap<String, BlockOutput>;

/// A daemon for muse-status. The daemon handles the logic of blocks as a server. Any connected
/// clients are sent the formatted status output.
pub struct Daemon {
    config: Config,
    subscribers: Vec<Subscriber>,
    update_request_senders: Vec<UpdateRequestSender>,
    block_outputs: BlockOutputs,
}

type DaemonMutexArc = Arc<Mutex<Daemon>>;

impl Daemon {
    /// Creates a new Daemon that runs at the specified address.
    pub fn new(config: Config) -> Self {
        Daemon {
            config,
            subscribers: Vec::new(),
            update_request_senders: Vec::new(),
            block_outputs: Default::default(),
        }
    }

    /// Starts the Daemon with the given blocks by running many asynchronous threads. If starting
    /// is successful, this function will return a Vec of JoinHandles, which are to be used by
    /// the calling function.
    pub fn start(mut self, blocks: BlockVec) -> Result<Vec<JoinHandle<()>>, MuseStatusError> {
        #[cfg(debug_assertions)]
        println!("the daemon has been started");

        // start listening on the daemon's address
        let listener = TcpListener::bind(&self.config.daemon_addr)?;

        // get channels for block outputs and banners
        let (block_tx, block_rx) = mpsc::channel::<BlockOutputMsg>();
        let (_banner_tx, banner_rx) = mpsc::channel::<format::Banner>();

        // vector for thread handles
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();

        // start status blocks
        println!("starting all blocks...");
        let (mut block_handles, update_request_senders) = self.start_all_blocks(block_tx, blocks);
        self.update_request_senders = update_request_senders;
        thread_handles.append(&mut block_handles);

        let daemon_arc_mutex = Arc::new(Mutex::new(self));

        // accept connections and handle them, asynchronously
        let data_clone = daemon_arc_mutex.clone();
        thread_handles.push(
            thread::Builder::new()
                .name(String::from("client listener"))
                .spawn(move || {
                    Self::accept_connections(data_clone, &listener);
                })
                .unwrap(),
        );

        // listen for block outputs
        let blocks_thread_daemon_mutex = daemon_arc_mutex.clone();
        thread_handles.push(
            thread::Builder::new()
                .name(String::from("block listener"))
                .spawn(move || {
                    Self::listen_to_blocks(blocks_thread_daemon_mutex, block_rx);
                })
                .unwrap(),
        );

        // listen for banners
        let banners_thread_daemon_mutex = daemon_arc_mutex;
        thread_handles.push(
            thread::Builder::new()
                .name(String::from("banner listener"))
                .spawn(move || {
                    Self::listen_for_banners(banners_thread_daemon_mutex, banner_rx);
                })
                .unwrap(),
        );

        Ok(thread_handles)
    }

    fn start_all_blocks(
        &self,
        sender: Sender<BlockOutputMsg>,
        mut blocks: BlockVec,
    ) -> (Vec<JoinHandle<()>>, Vec<UpdateRequestSender>) {
        let mut handles = Vec::new();
        let mut senders = Vec::new();

        while let Some(b) = blocks.pop() {
            let name = b.name().to_string();

            #[cfg(debug_assertions)]
            println!("==> starting '{}'...", name);

            let (mut handle_vec, sender) = b.run(sender.clone());

            handles.append(&mut handle_vec);
            senders.push(UpdateRequestSender(name, sender));
        }

        (handles, senders)
    }

    /// Should be run within a separate thread. `self` should NOT be a parameter, as a mutex would
    /// be locked for the entirety of this never-ending function.
    fn accept_connections(daemon_arc: DaemonMutexArc, listener: &TcpListener) {
        #[cfg(debug_assertions)]
        println!("listening for connections");

        for result in listener.incoming() {
            match result {
                Ok(conn) => {
                    if let Err(e) = Self::handle_connection(daemon_arc.clone(), conn) {
                        eprintln!(
                            "there was a problem handling a new connection ({}), but the daemon will keep running",
                            e
                        );
                    }
                }
                Err(e) => std::panic::panic_any(e),
            }
        }
    }

    /// Should be run within a separate thread. `self` should NOT be a parameter, as a mutex would
    /// be locked for the entirety of this never-ending function.
    fn listen_to_blocks(daemon_arc: DaemonMutexArc, block_rx: Receiver<BlockOutputMsg>) {
        #[cfg(debug_assertions)]
        println!("listening for block updates");

        while let Ok(msg) = block_rx.recv() {
            #[cfg(debug_assertions)]
            println!(
                "received block update from {}: {:?}",
                msg.name(),
                msg.data()
            );

            let mut daemon = daemon_arc.lock().unwrap();
            if let Some(output) = msg.data() {
                daemon.block_outputs.insert(msg.name(), output);
            } else {
                daemon.block_outputs.remove(&msg.name());
            }

            if let Err(e) = daemon.send_output_update_to_all(msg) {
                eprintln!("there was an error: {}", e)
            }
        }
    }

    /// Should be run within a separate thread. `self` should NOT be a parameter, as a mutex would
    /// be locked for the entirety of this never-ending function.
    fn listen_for_banners(_daemon_arc: DaemonMutexArc, _banner_rx: Receiver<format::Banner>) {
        todo!()
    }

    fn subscribe_client(
        &mut self,
        conn: TcpStream,
        collection: Collection,
    ) -> Result<(), MuseStatusError> {
        #[cfg(debug_assertions)]
        println!("a new subscriber requested to connect");

        // initialize the subscriber by sending all current data to it
        let mut sub = Subscriber(conn, collection);
        self.force_send_data(&mut sub)?;

        // register the subscriber
        self.subscribers.push(sub);

        println!("new subscriber successfully connected");

        Ok(())
    }

    fn handle_connection(
        daemon_arc: DaemonMutexArc,
        conn: TcpStream,
    ) -> Result<(), MuseStatusError> {
        #[cfg(debug_assertions)]
        println!("handling a new connection");

        let mut buf_reader = std::io::BufReader::new(conn.try_clone()?);
        let mut raw_action = String::new();

        thread::Builder::new()
            .name("single client handler".to_string())
            .spawn(move || {
                buf_reader.read_line(&mut raw_action).unwrap();

                let action = serde_json::from_str(raw_action.as_str()).unwrap();

                #[cfg(debug_assertions)]
                println!("handling message from new client: {:?}", action);

                let mut daemon = daemon_arc.lock().unwrap();

                match action {
                    ClientMsg::Subscribe(collection) => {
                        daemon.subscribe_client(conn, collection).unwrap();
                    }
                    ClientMsg::Update(collection) => {
                        #[cfg(debug_assertions)]
                        println!("handling update request from client: {:?}", collection);

                        daemon.update_collection(&collection);
                    }
                    ClientMsg::Noop => (), // literally do nothing
                }
            })
            .unwrap();

        Ok(())
    }

    /// Sends data updates to subscribers.
    fn send_output_update_to_all(
        &mut self,
        new_block_output: BlockOutputMsg,
    ) -> Result<(), MuseStatusError> {
        #[cfg(debug_assertions)]
        println!("sending output to all subscribers: {:?}", new_block_output);

        let block_name = new_block_output.name();
        let serialized_output = serde_json::to_string(&DaemonMsg::NewOutput(new_block_output))?;
        let config = &self.config;

        // send updates, only retaining subscribers that were successfully sent updates
        self.subscribers.retain_mut(|sub| {
            if is_block_name_in_collection(config, &block_name, sub.collection()) {
                if let Err(e) = send_serialized_data(sub, &serialized_output) {
                    eprintln!(
                        "there was an error ({}). the subscriber will be ignored from now on",
                        e
                    );
                    false
                } else {
                    true
                }
            } else {
                #[cfg(debug_assertions)]
                println!(
                    "subscriber skipped when sending update: collection is {:?}",
                    sub.collection()
                );
                true
            }
        });

        Ok(())
    }

    /// Sends all data requested by the subscriber, usually to initialize it.
    fn force_send_data(&self, sub: &mut Subscriber) -> Result<(), MuseStatusError> {
        let all_outputs = self
            .block_outputs
            .values()
            .map(|v| v.to_owned())
            .collect::<Vec<BlockOutput>>();
        let msg = DaemonMsg::AllData(all_outputs);
        send_serialized_data(sub, &serde_json::to_string(&msg)?)
    }

    fn update_collection(&mut self, collection: &Collection) {
        // get the iterator of requesters to use according to the collection
        let all_requesters = self.update_request_senders.iter_mut();
        let config = &self.config;
        let requesters: Vec<&mut UpdateRequestSender> = all_requesters
            .filter(|r| is_block_name_in_collection(config, &r.0, collection))
            .collect();

        for requester in requesters {
            if let Err(e) = requester.send() {
                eprintln!("updating error: {}", e)
            }
        }
    }
}

/// A struct containing a TcpStream to send data to. The collection defines what data the
/// subscriber receives.
struct Subscriber(TcpStream, Collection);

impl Subscriber {
    /// Convenience function to get the Subscriber's TcpStream.
    fn stream(&self) -> &TcpStream {
        &self.0
    }

    /// Convenience function to get the Subscriber's requested Collection.
    fn collection(&self) -> &Collection {
        &self.1
    }
}

/// A struct/tuple for a block update request sender.
struct UpdateRequestSender(String, Sender<()>);

impl UpdateRequestSender {
    /// Convenience function for sending update requests.
    fn send(&mut self) -> Result<(), mpsc::SendError<()>> {
        self.1.send(())
    }
}

/// An enum for specifying a section of blocks. Used for subscriptions and other commands.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Collection {
    /// Primary-level blocks.
    Primary,

    /// Secondary-level blocks.
    Secondary,

    /// Tertiary-level blocks.
    Tertiary,

    /// All blocks.
    All,

    /// One specific block.
    One(String),

    /// Many custom-picked blocks.
    Many(Vec<String>),
}

/// A payload sent to clients, containing data.
#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonMsg {
    /// New output to be sent to clients
    NewOutput(BlockOutputMsg),

    /// A Vec of BlockOutputs for all data currently known by the daemon.
    AllData(Vec<BlockOutput>),
}

/// A collection of outputs from blocks to be formatted
#[derive(Serialize, Deserialize, Debug)]
pub enum DataPayload {
    /// All blocks ranked by primary, secondary, and tertiary levels
    Ranked {
        /// Primary-ranked blocks.
        primary: Vec<BlockOutput>,

        /// Secondary-ranked blocks.
        secondary: Vec<BlockOutput>,

        /// Tertiary-ranked blocks.
        tertiary: Vec<BlockOutput>,
    },

    /// A custom ordering of blocks.
    Unranked(Vec<BlockOutput>),
}

impl DataPayload {
    /// Creates a Ranked DataPayload out of the outputs provided.
    pub fn ranked(config: &Config, outputs: &BlockOutputs) -> Self {
        let (primary, secondary, tertiary) = (
            Self::make_vec(&config.primary_order, outputs),
            Self::make_vec(&config.secondary_order, outputs),
            Self::make_vec(&config.tertiary_order, outputs),
        );

        Self::Ranked {
            primary,
            secondary,
            tertiary,
        }
    }

    /// Creates a Ranked DataPayload out of the outputs provided, but only with primary blocks.
    pub fn only_primary(config: &Config, outputs: &BlockOutputs) -> Self {
        let v = Self::make_vec(&config.primary_order, outputs);

        Self::Ranked {
            primary: v,
            secondary: Default::default(),
            tertiary: Default::default(),
        }
    }

    /// Creates a Ranked DataPayload out of the outputs provided, but only with secondary blocks.
    pub fn only_secondary(config: &Config, outputs: &BlockOutputs) -> Self {
        let v = Self::make_vec(&config.secondary_order, outputs);

        Self::Ranked {
            primary: Default::default(),
            secondary: v,
            tertiary: Default::default(),
        }
    }

    /// Creates a Ranked DataPayload out of the outputs provided, but only with tertiary blocks.
    pub fn only_tertiary(config: &Config, outputs: &BlockOutputs) -> Self {
        let v = Self::make_vec(&config.tertiary_order, outputs);

        Self::Ranked {
            primary: Default::default(),
            secondary: Default::default(),
            tertiary: v,
        }
    }

    /// Creates an Unranked DataPayload out any arbitrary combination of blocks.
    pub fn from_many(names: &[String], outputs: &BlockOutputs) -> Self {
        let v = Self::make_vec(names, outputs);
        Self::Unranked(v)
    }

    /// Creates an Unranked DataPayload out of exactly one arbitrary block.
    pub fn from_one(name: &str, outputs: &BlockOutputs) -> Self {
        Self::Unranked(Self::make_vec(&[name.to_string()], outputs))
    }

    fn make_vec(names: &[String], outputs: &BlockOutputs) -> Vec<BlockOutput> {
        names
            .iter()
            .filter_map(|name| outputs.get(name).cloned())
            .collect()
    }
}

fn is_block_name_in_collection(config: &Config, block_name: &str, collection: &Collection) -> bool {
    match collection {
        Collection::All => true,
        Collection::Primary => config.primary_order.iter().any(|n| n == block_name),
        Collection::Secondary => config.secondary_order.iter().any(|n| n == block_name),
        Collection::Tertiary => config.tertiary_order.iter().any(|n| n == block_name),
        Collection::One(b) => b == block_name,
        Collection::Many(v) => v.iter().any(|n| n == block_name),
    }
}

fn send_serialized_data(
    sub: &mut Subscriber,
    serialized_data: &str,
) -> Result<(), MuseStatusError> {
    // add a new line to the end of the data so that clients can parse correctly
    let out = format!("{}\n", serialized_data);
    sub.stream()
        .write_all(out.as_bytes())
        .map_err(MuseStatusError::from)
}
