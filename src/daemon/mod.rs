use crate::errors::*;
use crate::format;
use crate::format::blocks::output::BlockOutput;
use crate::format::blocks::Block;
use crate::format::Banner;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::BufRead;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

type BlockVec = Vec<Box<dyn Block>>;
type BlockSlice<'a> = &'a [Box<dyn Block>];
type BlockNames = (Vec<String>, Vec<String>, Vec<String>);
type BlockOutputs = HashMap<String, BlockOutput>;

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

/// A daemon for muse-status. The daemon handles the logic of blocks as a server. Any connected
/// clients are sent the formatted status output.
pub struct Daemon {
    addr: String,
    subscribers: Vec<Subscriber>,

    update_request_senders: Vec<UpdateRequestSender>,

    block_names: BlockNames,
    block_outputs: BlockOutputs,

    banners: VecDeque<Banner>,
}

type DaemonMutexArc = Arc<Mutex<Daemon>>;

impl Daemon {
    /// Creates a new Daemon that runs at the specified address.
    pub fn new(addr: &str) -> Self {
        Daemon {
            addr: addr.to_string(),
            subscribers: Vec::new(),

            update_request_senders: Vec::new(),

            block_names: Default::default(),
            block_outputs: Default::default(),

            banners: VecDeque::new(),
        }
    }

    /// Starts the Daemon with the given blocks by running many asynchronous threads. If starting
    /// is successful, this function will return a Vec of JoinHandles, which are to be used by
    /// the calling function.
    pub fn start(
        mut self,
        primary_blocks: BlockVec,
        secondary_blocks: BlockVec,
        tertiary_blocks: BlockVec,
    ) -> Result<Vec<JoinHandle<()>>, MuseStatusError> {
        // start listening on the daemon's address
        let listener = TcpListener::bind(&self.addr)?;

        // set block names from blocks
        self.set_block_names_from_blocks(&primary_blocks, &secondary_blocks, &tertiary_blocks);

        // get channels for block outputs and banners
        let (block_tx, block_rx) = mpsc::channel::<BlockOutput>();
        let (_banner_tx, banner_rx) = mpsc::channel::<format::Banner>();

        // vector for thread handles
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();

        // start status blocks
        println!("starting all blocks...");
        let (mut block_handles, update_request_senders) =
            self.start_all_blocks(block_tx, primary_blocks, secondary_blocks, tertiary_blocks);
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

    fn set_block_names_from_blocks(
        &mut self,
        primary_blocks: BlockSlice,
        secondary_blocks: BlockSlice,
        tertiary_blocks: BlockSlice,
    ) {
        for p in primary_blocks {
            self.block_names.0.push(p.name().to_string());
        }
        for s in secondary_blocks {
            self.block_names.1.push(s.name().to_string());
        }
        for t in tertiary_blocks {
            self.block_names.2.push(t.name().to_string());
        }
    }

    fn start_all_blocks(
        &self,
        sender: Sender<BlockOutput>,
        primary_blocks: BlockVec,
        mut secondary_blocks: BlockVec,
        mut tertiary_blocks: BlockVec,
    ) -> (Vec<JoinHandle<()>>, Vec<UpdateRequestSender>) {
        let mut handles = Vec::new();
        let mut senders = Vec::new();

        let mut all = primary_blocks;
        all.append(&mut secondary_blocks);
        all.append(&mut tertiary_blocks);

        while let Some(b) = all.pop() {
            let name = b.name().to_string();
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
        for result in listener.incoming() {
            match result {
                Ok(conn) => {
                    if let Err(e) = Self::handle_connection(daemon_arc.clone(), conn) {
                        eprintln!(
                            "there was a problem handling a connection to the daemon ({}), but the daemon will keep running",
                            e
                        );
                    }
                }
                Err(e) => panic!(e),
            }
        }
    }

    /// Should be run within a separate thread. `self` should NOT be a parameter, as a mutex would
    /// be locked for the entirety of this never-ending function.
    fn listen_to_blocks(daemon_arc: DaemonMutexArc, block_rx: Receiver<BlockOutput>) {
        while let Ok(output) = block_rx.recv() {
            let mut daemon = daemon_arc.lock().unwrap();
            daemon
                .block_outputs
                .insert(output.block_name.clone(), output.clone());

            if let Err(e) = daemon.send_output_update(output) {
                eprintln!("there was an error: {}", e)
            }
        }
    }

    /// Should be run within a separate thread. `self` should NOT be a parameter, as a mutex would
    /// be locked for the entirety of this never-ending function.
    fn listen_for_banners(_daemon_arc: DaemonMutexArc, _banner_rx: Receiver<format::Banner>) {
        // unimplemented!()
        // while let Ok(_) = banner_rx.recv() {
        //     let _ = daemon_arc.lock().unwrap().send_data_to_all();
        // }
    }

    fn subscribe_client(
        &mut self,
        conn: TcpStream,
        collection: Collection,
    ) -> Result<(), MuseStatusError> {
        // initialize the subscriber by sending all current data to it
        let mut sub = Subscriber(conn, collection);
        self.force_send_data(&mut sub);

        // register the subscriber
        self.subscribers.push(sub);

        // notify we've successfully connected with a new subscriber
        println!("new subscriber connected");

        Ok(())
    }

    fn handle_connection(
        daemon_arc: DaemonMutexArc,
        conn: TcpStream,
    ) -> Result<(), MuseStatusError> {
        let mut daemon = daemon_arc.lock().unwrap();

        let mut buf_reader = std::io::BufReader::new(conn.try_clone()?);
        let mut raw_action = String::new();

        // TODO XXX This is probably our issue! This call will block if the client doesn't send
        // anything. Move this and all following into a separate thread (maybe)?
        buf_reader.read_line(&mut raw_action)?;

        let action = serde_json::from_str(raw_action.as_str())?;
        match action {
            ClientMsg::Subscribe(collection) => {
                daemon.subscribe_client(conn, collection)?;
            }
            ClientMsg::Update(collection) => {
                println!("handling update request from client: {:?}", collection);
                daemon.update_collection(collection);
            }
            ClientMsg::Noop => (), // literally do nothing
        }

        Ok(())
    }

    /// Sends data updates to subscribers.
    fn send_output_update(&mut self, new_block_output: BlockOutput) -> Result<(), MuseStatusError> {
        let iter = self.subscribers.iter_mut();
        let serialized_output = serde_json::to_string(&new_block_output)?;

        for sub in iter {
            Self::send_serialized_data(sub, &serialized_output)?;
        }

        Ok(())
    }

    fn send_serialized_data(
        sub: &mut Subscriber,
        serialized_data: &str,
    ) -> Result<(), MuseStatusError> {
        sub.stream().write_all(serialized_data.as_bytes())
            .map_err(MuseStatusError::from)
    }

    /// Sends all data requested by the subscriber, usually to initialize it.
    fn force_send_data(&self, _sub: &mut Subscriber) {
        unimplemented!()
    }

    fn request_block_update(&mut self, who: &str) {
        self.update_request_senders
            .iter()
            .find(|tup| tup.0 == who)
            .map(|tup| tup.1.send(()));
    }

    fn is_block_name_in_collection(&self, block_name: &str, collection: &Collection) -> bool {
        match collection {
            Collection::All => true,
            Collection::Primary => self.block_names.0.iter().any(|n| n == block_name),
            Collection::Secondary => self.block_names.1.iter().any(|n| n == block_name),
            Collection::Tertiary => self.block_names.2.iter().any(|n| n == block_name),
            Collection::One(b) => b == block_name
        }
    }

    fn update_collection(&mut self, collection: Collection) {
        let (primary_names, secondary_names, tertiary_names) = &self.block_names;

        // get the iterator of requesters to use according to the collection
        let mut all_iter_mut = self.update_request_senders.iter_mut();
        let requesters: Vec<&mut UpdateRequestSender> = match collection {
            Collection::All => all_iter_mut.collect(),
            Collection::Primary => all_iter_mut
                .filter(|s| primary_names.iter().any(|n| s.0 == *n))
                .collect(),
            Collection::Secondary => all_iter_mut
                .filter(|s| secondary_names.iter().any(|n| s.0 == *n))
                .collect(),
            Collection::Tertiary => all_iter_mut
                .filter(|s| tertiary_names.iter().any(|n| s.0 == *n))
                .collect(),
            Collection::One(b) => all_iter_mut
                .find(|r| r.0 == b)
                .map_or(vec![], |r| vec![r])};

        for requester in requesters {
            if let Err(e) = requester.send() {
                eprintln!("updating error: {}", e)
            }
        }
    }
}

/// A payload sent from clients to the daemon.
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMsg {
    /// Connect to the daemon and receive updates from it.
    Subscribe(Collection),

    /// Update some part of the client.
    Update(Collection),

    /// Literally do nothing.
    Noop,
}

impl ClientMsg {
    /// Creates a ClientMsg from arguments passed through the command line.
    pub fn from_env() -> Self {
        // a temporary type to pick up the pieces passed through the command line. (it would be
        // difficult to get the message type and the collection argument at the same time)
        enum ClientMsgType {
            Subscribe,
            Update,
        }

        let env = std::env::args();

        let mut force_update = false;
        let mut collection = None;
        let mut msg_type = None;

        for arg in env {
            if arg == "--force" || arg == "-f" {
                force_update = true;
            } else if arg.starts_with('-') || arg.is_empty() {
                continue;
            }

            // check if arg starts with any of the following:
            //      "su" => subscribe
            //      "l"  => listen
            //      "u"  => update
            //      "n"  => notify
            //      "p"  => primary
            //      "se" => secondary
            //      "t"  => tertiary
            //      "a"  => all
            if arg.starts_with("su") || arg.starts_with('l') {
                msg_type = Some(ClientMsgType::Subscribe);
            } else if arg.starts_with('u') || arg.starts_with('n') {
                msg_type = Some(ClientMsgType::Update);
            } else if arg.starts_with('p') {
                collection = Some(Collection::Primary);
            } else if arg.starts_with("se") {
                collection = Some(Collection::Secondary);
            } else if arg.starts_with('t') {
                collection = Some(Collection::Tertiary);
            } else if arg.starts_with('a') {
                collection = Some(Collection::All);
            } else {
                assert!(!arg.starts_with('-'));
                collection = Some(Collection::One(arg));
            }
        }

        // assemble the msg
        if let Some(t) = msg_type {
            match collection {
                Some(c) => match t {
                    ClientMsgType::Subscribe => Self::Subscribe(c),
                    ClientMsgType::Update => Self::Update(c),
                },
                None => {
                    match t {
                        ClientMsgType::Subscribe => Self::Subscribe(Collection::All),
                        ClientMsgType::Update => {
                            if force_update {
                                Self::Update(Collection::All)
                            } else {
                                println!("what do you want to update? try `update all` or `update primary`");
                                Self::Noop
                            }
                        }
                    }
                }
            }
        } else {
            Self::default()
        }
    }
}

impl Default for ClientMsg {
    fn default() -> Self {
        Self::Subscribe(Collection::All)
    }
}

/// An enum for specifying a section of blocks. Used for subscriptions and other commands.
#[derive(Serialize, Deserialize, Debug)]
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
}

/// A payload sent to clients, containing data.
#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonMsg {
    /// New output to be sent to clients
    NewOutput(BlockOutput),
}

/// A collection of all outputs from blocks. Sent to clients as part of a DaemonMsg.
#[derive(Serialize, Deserialize, Debug)]
pub struct DataOutput {
    /// Output from primary blocks.
    pub primary: Vec<BlockOutput>,

    /// Output from secondary blocks.
    pub secondary: Vec<BlockOutput>,

    /// Output from tertiary blocks.
    pub tertiary: Vec<BlockOutput>,
}

impl DataOutput {
    fn from_block_names_and_outputs(
        block_names: &BlockNames,
        outputs: &HashMap<String, BlockOutput>,
    ) -> Self {
        let (mut primary, mut secondary, mut tertiary) = (Vec::new(), Vec::new(), Vec::new());

        for p in &block_names.0 {
            if let Some(o) = outputs.get(p) {
                primary.push((*o).clone());
            }
        }

        for s in &block_names.1 {
            if let Some(o) = outputs.get(s) {
                secondary.push((*o).clone());
            }
        }

        for t in &block_names.2 {
            if let Some(o) = outputs.get(t) {
                tertiary.push((*o).clone());
            }
        }

        Self {
            primary,
            secondary,
            tertiary,
        }
    }
}
