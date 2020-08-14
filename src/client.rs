use crate::daemon::{Collection, DaemonMsg, DataPayload};
use crate::errors::{BasicError, MuseStatusError};
use crate::format::blocks::BlockOutput;
use crate::format::Formatter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

/// A Client that connects to the Daemon and receives data.
pub struct Client {
    action: ClientMsg,
    data: HashMap<String, BlockOutput>,
}

impl Client {
    /// Returns a new Client with action and formatter parsed from the command line arguments
    pub fn new() -> Result<Self, MuseStatusError> {
        Ok(Self {
            action: ClientMsg::from_env(),
            data: HashMap::new(),
        })
    }

    /// Have the Client send its message to the daemon. This functions consumes the Client.
    ///
    /// If the client should subscribe to the daemon, it will receive updates (first requesting
    /// all data) and then output formatted data to stdout.
    ///
    /// If the client should request the daemon to update, it will send its request and then quit.
    ///
    /// If the client should do nothing, it summons a unicorn. But you can't see it. You'll never
    /// know it was summoned. You'll just think that nothing happened, because that's exactly what
    /// Noop does.
    pub fn act(self) -> Result<(), MuseStatusError> {
        if let ClientMsg::Noop = &self.action {
            // girl bye
            Ok(())
        } else {
            // for anything else, we'll need a connection to the daemon.
            let mut stream = get_daemon_connection();
            stream.write_all(format!("{}\n", serde_json::to_string(&self.action)?).as_bytes())?;

            // if Subscribe, handle the subscription. if Update, send request and quit.
            // self.action is cloned in the case that we handle a subscription and `self` must be
            // moved
            if let ClientMsg::Subscribe(c) = self.action.clone() {
                self.handle_subscription(stream, &c);
            } else {
                // if Update or somehow Noop, the client does not need to maintain its connection
                // to the daemon, so we just return
                Ok(())
            }
        }
    }

    /// If the client should subscribe and output data, handle that. Because this function never
    /// returns, it will take ownership of `self`.
    pub fn handle_subscription(mut self, mut daemon_conn: TcpStream, collection: &Collection) -> ! {
        let formatter = Formatter::from_env().unwrap();

        if let crate::format::Mode::JsonProtocol = formatter.get_format_mode() {
            println!("{{\"version\":1}}");
            println!("[[]");
        }

        loop {
            // create a buffered stream, which we'll read from line by line for status outputs
            let mut buf_stream = BufReader::new(daemon_conn);

            // listen for outputs from the daemon and print them
            'inner: loop {
                let mut s = String::new();
                match buf_stream.read_line(&mut s) {
                    Ok(n) => {
                        if n == 0 {
                            break 'inner;
                        } else {
                            // `s` should be a DaemonMsg
                            let msg = match serde_json::from_str::<DaemonMsg>(&s) {
                                Ok(m) => m,
                                Err(e) => {
                                    self.echo_error(e, &formatter);
                                    break 'inner;
                                }
                            };

                            // only matching one thing for now lol
                            match msg {
                                DaemonMsg::NewOutput(o) => {
                                    self.data.insert(o.block_name.clone(), o);
                                    self.echo_output(collection, &formatter);
                                }
                            }
                        }
                    }
                    Err(e) => self.echo_error(e, &formatter),
                }
            }

            // if the connection to the daemon is lost, restore it
            daemon_conn = get_daemon_connection();
        }
    }

    /// Prints formatted output.
    fn echo_output(&self, collection: &Collection, f: &Formatter) -> Result<(), MuseStatusError> {
        let data = match collection {
            Collection::All => DataPayload::ranked(&self.data),
            Collection::Primary => DataPayload::only_primary(&self.data),
            Collection::Secondary => DataPayload::only_secondary(&self.data),
            Collection::Tertiary => DataPayload::only_tertiary(&self.data),
            Collection::One(b) => DataPayload::from_one(b, &self.data),
            Collection::Many(n) => DataPayload::from_many(
                &n.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
                &self.data,
            ),
        };

        println!("{}", f.format_data(data));
        Ok(())
    }

    /// Prints formatted error.
    fn echo_error<E: Error>(&self, e: E, f: &Formatter) {
        println!("{}", f.format_error(e));
    }
}

/// Polls for a connection to the daemon.
fn get_daemon_connection() -> TcpStream {
    loop {
        if let Ok(s) = TcpStream::connect("localhost:1612") {
            return s;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// A payload sent from clients to the daemon.
#[derive(Serialize, Deserialize, Clone, Debug)]
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
                continue;
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
