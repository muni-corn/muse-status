use crate::errors::*;
use crate::format;
use crate::format::blocks::output::BlockOutput;
use crate::format::blocks::Block;
use crate::format::Banner;
use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashMap};
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

/// A daemon for muse-status. The daemon handles the logic of blocks as a server. Any connected
/// clients are sent the formatted status output.
pub struct Daemon {
    addr: String,
    connections: Vec<TcpStream>,

    notify_senders: Vec<Sender<String>>,

    block_names: BlockNames,
    block_outputs: BlockOutputs,

    banners: VecDeque<Banner>,
}

type DaemonMutexArc = Arc<Mutex<Daemon>>;

impl Daemon {
    /// Creates a new Daemon that runs at the specified address.
    pub fn new(
        addr: &str,
    ) -> Self {
        Daemon {
            addr: addr.to_string(),
            connections: Vec::new(),

            notify_senders: Vec::new(),

            block_names: Default::default(),
            block_outputs: Default::default(),

            banners: VecDeque::new(),
        }
    }

    /// Starts the Daemon with the given blocks by running many asynchronous threads. If starting
    /// is successful, this function will return a Vec of JoinHandles, which are to be used by
    /// the calling function.
    pub fn start(mut self,
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
        let (mut block_handles, update_request_senders) = self.start_all_blocks(block_tx, primary_blocks, secondary_blocks, tertiary_blocks);
        self.notify_senders = update_request_senders;
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

    fn set_block_names_from_blocks(&mut self, primary_blocks: &BlockVec, secondary_blocks: &BlockVec, tertiary_blocks: &BlockVec) {
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
    ) -> (Vec<JoinHandle<()>>, Vec<Sender<String>>) {
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let mut senders: Vec<Sender<String>> = Vec::new();

        let mut all = primary_blocks;
        all.append(&mut secondary_blocks);
        all.append(&mut tertiary_blocks);

        while let Some(b) = all.pop() {
            let (mut handle_vec, sender) = b.run(sender.clone());

            handles.append(&mut handle_vec);
            senders.push(sender);
        }

        (handles, senders)
    }

    /// Shound be run within a separate thread. `self` should NOT a parameter, as a mutex would be
    /// locked for the entirety of this never-ending function.
    fn accept_connections(daemon_arc: DaemonMutexArc, listener: &TcpListener) {
        for result in listener.incoming() {
            match result {
                Ok(conn) => {
                    if let Err(e) = Self::handle_connection(daemon_arc.clone(), conn) {
                        eprintln!(
                            "there was a problem handling a connection to the daemon: {}",
                            e
                        );
                    }
                }
                Err(e) => panic!(e),
            }
        }
    }

    /// Shound be run within a separate thread. `self` should NOT a parameter, as a mutex would be
    /// locked for the entirety of this never-ending function.
    fn listen_to_blocks(daemon_arc: DaemonMutexArc, block_rx: Receiver<BlockOutput>) {
        while let Ok(output) = block_rx.recv() {
            let mut daemon = daemon_arc.lock().unwrap();
            daemon.block_outputs.insert(output.block_name.clone(), output.clone());
            let _ = daemon.send_data_to_all(); // TODO
        }
    }

    /// Shound be run within a separate thread. `self` should NOT a parameter, as a mutex would be
    /// locked for the entirety of this never-ending function.
    fn listen_for_banners(daemon_arc: DaemonMutexArc, banner_rx: Receiver<format::Banner>) {
        while let Ok(_) = banner_rx.recv() {
            let _ = daemon_arc.lock().unwrap().send_data_to_all(); // TODO
        }
    }

    fn handle_client_command(&mut self, cmd: &str) -> Result<(), MuseStatusError> {
        // handle command
        let mut split = cmd.split_whitespace();

        if let Some(command) = split.next() {
            match command {
                "update" | "notify" => {
                    for value in split {
                        self.notify(&value);
                    }
                }
                _ => {
                    // unknown subcommand
                    return Err(MuseStatusError::from(BasicError {
                        message: format!(
                                     "muse-status doesn't understand this command: {}",
                                     command
                                 ),
                    }));
                }
            }
        }

        Ok(())
    }

    fn handle_connection(
        daemon_arc: DaemonMutexArc,
        mut conn: TcpStream,
    ) -> Result<(), MuseStatusError> {
        let mut daemon = daemon_arc.lock().unwrap();

        let mut buf_reader = std::io::BufReader::new(conn.try_clone()?);
        let mut raw_action = String::new();
        buf_reader.read_line(&mut raw_action)?;

        let action: ClientMsg = serde_json::from_str(raw_action.as_str()).unwrap();

        if let ClientMsg::Command(c) = &action {
            println!("handling command from client: {}", c);
            daemon.handle_client_command(c)?;
        } else {
            println!("new listener connected");

            let serialized = daemon.get_serialized_data()?;
            send_serialized_data(&mut conn, &serialized)?;
            daemon.connections.push(conn);
        }

        Ok(())
    }

    fn send_data_to_all(&mut self) -> Result<(), MuseStatusError> {
        // we're doing this so that get_serialized_data doesn't have to be computed for each
        // connection
        let serialized = self.get_serialized_data()?;

        let iter = self.connections.iter_mut();
        for conn in iter {
            send_serialized_data(conn, &serialized)?;
        }

        Ok(())
    }

    fn get_serialized_data(&self) -> Result<String, MuseStatusError> {
        let output = DataOutput::from_block_names_and_outputs(&self.block_names, &self.block_outputs);
        Ok(format!("{}\n", serde_json::to_string(&output)?))
    }

    fn notify(&mut self, who: &str) {
        for sender in &self.notify_senders {
            let _ = sender.send(who.to_owned());
        }
    }
}

/// A payload sent from clients to the daemon.
#[derive(Serialize, Deserialize)]
pub enum ClientMsg {
    /// Send a command to the daemon, then disconnect.
    Command(String),

    /// Only connect to the daemon and print outputs.
    Connect,
}

/// A payload sent to clients, containing data.
#[derive(Serialize, Deserialize)]
pub enum DaemonMsg {
    /// New output to be sent to clients
    DataOutput(DataOutput),
}

/// A collection of all outputs from blocks. Sent to clients as part of a DaemonMsg.
#[derive(Serialize, Deserialize)]
pub struct DataOutput {
    /// Output from primary blocks.
    pub primary: Vec<BlockOutput>,

    /// Output from secondary blocks.
    pub secondary: Vec<BlockOutput>,

    /// Output from tertiary blocks.
    pub tertiary: Vec<BlockOutput>,
}

impl DataOutput {
    fn from_block_names_and_outputs(block_names: &BlockNames, outputs: &HashMap<String, BlockOutput>) -> Self {
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
            primary, secondary, tertiary
        }
    }
}

fn send_serialized_data(conn: &mut TcpStream, data: &str) -> Result<(), MuseStatusError> {
    conn.write_all(data.as_bytes()).map_err(MuseStatusError::from)
}
