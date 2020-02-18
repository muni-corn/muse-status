use crate::errors::*;
use crate::format;
use crate::format::blocks::output::BlockOutput;
use crate::format::blocks::Block;
use std::io::BufRead;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

/// A daemon for muse-status. The daemon handles the logic of blocks as a server. Any connected
/// clients are sent the formatted status output.
pub struct Daemon {
    addr: String,
    connections: Vec<TcpStream>,
    formatter: format::Formatter,

    notify_senders: Vec<Sender<String>>,
    last_output: Option<String>,
}

type DaemonArc = Arc<Mutex<Daemon>>;

impl Daemon {
    /// Creates a new Daemon that runs at the specified address.
    pub fn new(addr: &str) -> Self {
        Daemon {
            addr: addr.to_string(),
            connections: Vec::new(),
            formatter: Default::default(),

            notify_senders: Vec::new(),

            last_output: None,
        }
    }

    /// Starts the Daemon with the given blocks by running many asynchronous threads. If starting
    /// is successful, this function will return a Vec of JoinHandles, which are to be used by
    /// the calling function.
    pub fn start(
        mut self,
        primary_blocks: Vec<Box<dyn Block>>,
        secondary_blocks: Vec<Box<dyn Block>>,
        ternary_blocks: Vec<Box<dyn Block>>,
    ) -> Result<Vec<JoinHandle<()>>, MuseStatusError> {
        // set formatter's block names
        self.formatter.set_block_names_from_blocks(
            &primary_blocks,
            &secondary_blocks,
            &ternary_blocks,
        );

        // start listening on the daemon's address
        let listener = TcpListener::bind(&self.addr)?;

        // get output channel from formatter
        let formatter_output_rx = self.formatter.new_output_channel();

        // get channels for block outputs and banners
        let (block_tx, block_rx) = mpsc::channel::<BlockOutput>();
        let (_banner_tx, banner_rx) = mpsc::channel::<format::Banner>();

        // vector for thread handles
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();

        let daemon_arc_mutex = Arc::new(Mutex::new(self));

        // accept connections and handle them, asynchronously
        let data_clone = daemon_arc_mutex.clone();
        thread_handles.push(thread::Builder::new().name(String::from("client listener")).spawn(move || {
            Self::accept_connections(data_clone, &listener);
        }).unwrap());

        // listen for block outputs
        let blocks_thread_daemon_mutex = daemon_arc_mutex.clone();
        thread_handles.push(thread::Builder::new().name(String::from("block listener")).spawn(move || {
            Self::listen_to_blocks(blocks_thread_daemon_mutex, block_rx);
        }).unwrap());

        // listen for banners
        let banners_thread_daemon_mutex = daemon_arc_mutex.clone();
        thread_handles.push(thread::Builder::new().name(String::from("banner listener")).spawn(move || {
            Self::listen_for_banners(banners_thread_daemon_mutex, banner_rx);
        }).unwrap());

        // listen to formatter
        let formatter_listener_daemon_mutex = daemon_arc_mutex.clone();
        thread_handles.push(thread::Builder::new().name(String::from("output listener")).spawn(move || {
            Self::listen_for_formatter_updates(formatter_listener_daemon_mutex, formatter_output_rx)
        }).unwrap());

        // thread::spawn(self.listen_for_xorg_changes());

        // start status blocks
        let (mut block_handles, update_request_senders) =
            Self::start_all_blocks(block_tx, primary_blocks, secondary_blocks, ternary_blocks);
        daemon_arc_mutex.lock().unwrap().notify_senders = update_request_senders;
        thread_handles.append(&mut block_handles);

        Ok(thread_handles)
    }

    fn start_all_blocks(
        sender: Sender<BlockOutput>,
        mut primary_blocks: Vec<Box<dyn Block>>,
        mut secondary_blocks: Vec<Box<dyn Block>>,
        mut ternary_blocks: Vec<Box<dyn Block>>,
    ) -> (Vec<JoinHandle<()>>, Vec<Sender<String>>) {
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let mut senders: Vec<Sender<String>> = Vec::new();

        // combines all blocks into one Vec
        secondary_blocks.append(&mut ternary_blocks);
        primary_blocks.append(&mut secondary_blocks);
        let mut all = primary_blocks;

        while let Some(b) = all.pop() {
            let (mut handle_vec, sender) = b.run(sender.clone());
            handles.append(&mut handle_vec);
            senders.push(sender);
        }

        (handles, senders)
    }

    /// Shound be run within a separate thread. `self` should NOT a parameter, as a mutex would be
    /// locked for the entirety of this never-ending function.
    fn accept_connections(daemon_arc: DaemonArc, listener: &TcpListener) {
        for result in listener.incoming() {
            match result {
                Ok(conn) => {
                    println!("new client connected");
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
    fn listen_to_blocks(daemon_arc: DaemonArc, block_rx: Receiver<BlockOutput>) {
        while let Ok(b) = block_rx.recv() {
            daemon_arc.lock().unwrap().formatter.update(b)
        }
    }

    /// Shound be run within a separate thread. `self` should NOT a parameter, as a mutex would be
    /// locked for the entirety of this never-ending function.
    fn listen_for_banners(daemon_arc: DaemonArc, banner_rx: Receiver<format::Banner>) {
        while let Ok(b) = banner_rx.recv() {
            daemon_arc.lock().unwrap().formatter.banner(b)
        }
    }

    /// Shound be run within a separate thread. `self` should NOT a parameter, as a mutex would be
    /// locked for the entirety of this never-ending function.
    fn listen_for_formatter_updates(daemon_arc: DaemonArc, formatter_output_rx: Receiver<String>) {
        while let Ok(o) = formatter_output_rx.recv() {
            println!("echoing output from formatter: {}", o);
            let mut daemon = daemon_arc.lock().unwrap();
            let _ = daemon.echo(&o);
            daemon.last_output = Some(o);
        }
    }

    /// Parses arguments passed to a `muse-status-daemon` or `muse-status` command. Results in an
    /// Error if argument parsing failed.
    pub fn handle_args(&mut self, args: &[String]) -> Result<(), MuseStatusError> {
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if let Some(next) = iter.next() {
                match arg.as_str() {
                    "-p" | "--primary-color" => {
                        self.formatter.set_primary_color(&next)?;
                    }
                    "-s" | "--secondary-color" => {
                        self.formatter.set_secondary_color(&next)?;
                    }
                    "-f" | "--font" => {
                        self.formatter.set_text_font(&next);
                    }
                    "-i" | "--icon-font" => {
                        self.formatter.set_icon_font(&next);
                    }
                    "-m" | "--mode" => match next.as_str() {
                        "i3" => {
                            self.formatter.set_format_mode(format::Mode::JsonProtocol);
                        }
                        "lemon" => {
                            self.formatter.set_format_mode(format::Mode::Lemonbar);
                        }
                        _ => unimplemented!(),
                    },
                    _ => unimplemented!(),
                }
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, cmd: &str) -> Result<(), MuseStatusError> {
        if cmd.starts_with('-') {
            let mut args: Vec<String> = Vec::new();
            for s in cmd.split_whitespace() {
                args.push(s.to_string());
            }
            self.handle_args(&args)?;
        } else {
            let mut split = cmd.split_whitespace();

            if let Some(subcommand) = split.next() {
                match subcommand {
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
                                subcommand
                            ),
                        }));
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_connection(
        daemon_arc: DaemonArc,
        mut conn: TcpStream,
    ) -> Result<(), MuseStatusError> {
        let mut daemon = daemon_arc.lock().unwrap();

        let mut buf_reader = std::io::BufReader::new(conn.try_clone()?);
        let mutex_clone = daemon_arc.clone();
        thread::Builder::new()
            .name("connection command handler thread".to_string())
            .spawn(move || {
                let mut command = String::new();
                buf_reader.read_line(&mut command).unwrap();
                mutex_clone
                    .lock()
                    .unwrap()
                    .handle_command(&command)
                    .unwrap();
            })?;

        if daemon.formatter.get_format_mode() == &format::Mode::JsonProtocol {
            conn.write_all(b"{\"version\":1}\n")?;
            conn.write_all(b"[[]\n")?;
        }

        if let Some(o) = &daemon.last_output {
            conn.write_all(format!(",{}\n", o).as_bytes())?;
        }

        daemon.connections.push(conn);

        Ok(())
    }

    fn echo(&mut self, s: &str) -> Result<(), MuseStatusError> {
        for mut conn in &self.connections {
            let f = format!(",{}\n", s);
            conn.write_all(f.as_bytes())?;
        }

        Ok(())
    }

    fn notify(&mut self, who: &str) {
        for sender in &self.notify_senders {
            let _ = sender.send(who.to_owned());
        }
    }
}
