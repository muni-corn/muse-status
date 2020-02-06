use crate::errors::*;
use crate::format;
use crate::format::blocks::Block;
use crate::format::blocks::output::BlockOutput;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::io::Write;
use std::sync::mpsc;

/// A daemon for muse-status. The daemon handles the logic of blocks as a server. Any connected
/// clients are sent the formatted status output.
pub struct Daemon {
    addr: String,
    connections: Vec<TcpStream>,
    formatter: format::Formatter,

    notify_senders: Vec<Sender<String>>,
}

type DaemonArc = Arc<Mutex<Daemon>>;

impl Daemon {
    /// Creates a new Daemon that runs at the specified address.
    pub fn new(
        addr: &str,
    ) -> Self {
        Daemon {
            addr: addr.to_string(),
            connections: Vec::new(),
            formatter: Default::default(),

            notify_senders: Vec::new(),
        }
    }

    /// Starts the Daemon with the given blocks by running many asynchronous threads. If starting
    /// is successful, this function will return a Vec of JoinHandles, which are to be used by
    /// the calling function.
    pub fn start(mut self, 
        formatter: format::Formatter,
        primary_blocks: Vec<Box<dyn Block>>,
        secondary_blocks: Vec<Box<dyn Block>>,
        ternary_blocks: Vec<Box<dyn Block>>,
    ) -> Result<Vec<JoinHandle<()>>, MuseStatusError> {
        self.formatter = formatter;
        let listener = TcpListener::bind(&self.addr)?;

        let (block_tx, block_rx) = mpsc::channel::<BlockOutput>();
        let (_banner_tx, banner_rx) = mpsc::channel::<format::Banner>();
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();

        let daemon_arc_mutex = Arc::new(Mutex::new(self));

        // accept connections and handle them, asynchronously
        let data_clone = daemon_arc_mutex.clone();
        thread_handles.push(thread::spawn(move || Self::accept_connections(data_clone, &listener)));

        // listen for block outputs
        let data_clone = daemon_arc_mutex.clone();
        thread_handles.push(thread::spawn(move || Self::listen_to_blocks(data_clone, block_rx)));

        // listen for banners
        let data_clone = daemon_arc_mutex.clone();
        thread_handles.push(thread::spawn(move || Self::listen_for_banners(data_clone, banner_rx)));

        // thread::spawn(self.listen_for_xorg_changes());

        // start status blocks
        let (mut block_handles, update_request_senders) = Self::start_all_blocks(block_tx, primary_blocks, secondary_blocks, ternary_blocks);
        daemon_arc_mutex.lock().unwrap().notify_senders = update_request_senders;
        thread_handles.append(&mut block_handles);

        Ok(thread_handles)
    }

    fn start_all_blocks(
        sender: Sender<BlockOutput>,
        mut primary_blocks: Vec<Box<dyn Block + 'static>>,
        mut secondary_blocks: Vec<Box<dyn Block>>,
        mut ternary_blocks: Vec<Box<dyn Block + 'static>>,
    ) -> (Vec<JoinHandle<()>>, Vec<Sender<String>>) {
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let mut senders: Vec<Sender<String>> = Vec::new();

        // combines all blocks into one Vec
        secondary_blocks.append(&mut ternary_blocks);
        primary_blocks.append(&mut secondary_blocks);
        let mut all = primary_blocks;

        while let Some(mut b) = all.pop() {
            let (mut handle_vec, sender) = b.run(sender.clone());
            handles.append(&mut handle_vec);
            senders.push(sender);
        }

        (handles, senders)
    }

    /// Shound be run within a separate thread.
    fn accept_connections(daemon_arc: DaemonArc, listener: &TcpListener) {
        for result in listener.incoming() {
            match result {
                Ok(conn) => { 
                    let _ = daemon_arc.lock().unwrap().handle_connection(conn); 
                },
                Err(e) => panic!(e),
            }
        }
    }

    /// Shound be run within a separate thread.
    fn listen_to_blocks(daemon_arc: DaemonArc, block_rx: Receiver<BlockOutput>) {
        loop {
            if let Ok(b) = block_rx.recv() { daemon_arc.lock().unwrap().formatter.update(b.block_name, b.body) }
        }
    }

    /// Shound be run within a separate thread.
    fn listen_for_banners(daemon_arc: DaemonArc, banner_rx: Receiver<format::Banner>) {
        loop {
            if let Ok(b) = banner_rx.recv() { daemon_arc.lock().unwrap().formatter.banner(b) }
        }
    }

    fn handle_command(&mut self, cmd: &str) -> Result<(), BasicError> {
        let mut split = cmd.split_whitespace();

        if let Some(subcommand) = split.next() {
            let value = split.collect::<String>();

            match subcommand {
                "update" => self.notify(&value),
                _ => {
                    return Err(BasicError {
                        message: format!(
                                     "muse-status doesn't understand this command: {}",
                                     subcommand
                                 ),
                    })
                }
            }
        }

        Ok(())
    }

    fn handle_connection(&mut self, conn: TcpStream) -> Result<(), MuseStatusError> {
        self.connections.push(conn);
        Ok(())
    }

    fn echo(&mut self, s: &str) -> Result<(), MuseStatusError> {
        for mut conn in &self.connections {
            let f = format!("{}\n", s);
            conn.write_all(f.as_bytes())?;
        }

        Ok(())
    }

    fn echo_new_status(&self) -> Result<(), MuseStatusError> {
        // let status = self.make_status_string(false);
        // self.echo(&status)
        Ok(())
    }

    fn notify(&mut self, who: &str) {
        for sender in &self.notify_senders {
            let _ = sender.send(who.to_owned());
        }
        let _ = self.echo_new_status();
    }
}
