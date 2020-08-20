use crate::daemon::{Collection, DaemonMsg, DataPayload};
use crate::errors::MuseStatusError;
use crate::format::blocks::BlockOutput;
use crate::format::Formatter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

/// A Client that connects to the Daemon and receives data.
pub struct Client {
    args: ClientArgs,
    data: HashMap<String, BlockOutput>,
}

impl Client {
    /// Returns a new Client with options parsed from command line arguments
    pub fn new() -> Result<Self, MuseStatusError> {
        Ok(Self {
            args: ClientArgs::from_env()?,
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
        if let ClientMsg::Noop = &self.args.client_msg {
            #[cfg(debug_assertions)]
            println!("doing nothing; exiting");

            // girl bye
            Ok(())
        } else {
            #[cfg(debug_assertions)]
            println!("sending action to daemon: {:?}", self.args.client_msg);

            // for anything else, we'll need a connection to the daemon.
            let mut stream = get_daemon_connection();
            stream.write_all(format!("{}\n", serde_json::to_string(&self.args.client_msg)?).as_bytes())?;

            // if Subscribe, handle the subscription. if Update, send request and quit.
            // self.args.client_msg is cloned in the case that we handle a subscription and `self` must be
            // moved
            if let ClientMsg::Subscribe(c) = self.args.client_msg.clone() {
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
                #[allow(clippy::single_match)]
                match buf_stream.read_line(&mut s) {
                    Ok(n) => {
                        if n == 0 {
                            break 'inner;
                        } else {
                            // `s` should be a DaemonMsg
                            let msg = match serde_json::from_str::<DaemonMsg>(&s) {
                                Ok(m) => m,
                                Err(e) => {
                                    eprintln!("{}", e);
                                    break 'inner;
                                }
                            };

                            // only matching one thing for now lol
                            match msg {
                                DaemonMsg::NewOutput(o) => {
                                    self.data.insert(o.block_name.clone(), o);
                                    self.echo_output(collection, &formatter);
                                }
                                DaemonMsg::AllData(a) => {
                                    for output in a {
                                        self.data.insert(output.block_name.clone(), output);
                                    }
                                    self.echo_output(collection, &formatter);
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("{}", e),
                }
            }

            // if the connection to the daemon is lost, restore it
            daemon_conn = get_daemon_connection();
        }
    }

    /// Prints formatted output.
    fn echo_output(&self, collection: &Collection, f: &Formatter) {
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
    }

    // TODO
    // /// Prints formatted error.
    // fn echo_error<E: Error>(&self, e: E, f: &Formatter) {
    //     println!("{}", f.format_error(e));
    // }
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

impl Default for ClientMsg {
    fn default() -> Self {
        Self::Subscribe(Collection::All)
    }
}

#[derive(Default)]
struct ClientArgs {
    client_msg: ClientMsg,
    force: bool,
    formatter: Formatter,
}

impl ClientArgs {
    pub fn from_env() -> Result<Self, MuseStatusError> {
        let mut result = Self::default();

        // a temporary type to pick up the pieces passed through the command line. (it would be
        // difficult to get the message type and the collection argument at the same time, so we
        // have separate variables for `msg_type` and `collection`)
        enum ClientMsgType {
            Subscribe,
            Update,
        }

        // default values
        let mut msg_type = ClientMsgType::Subscribe;
        let mut collection = Collection::All;

        let mut args = std::env::args();

        while let Some(arg) = args.next() {
            let mut extract_next_value = || args.next().ok_or_else(|| MuseStatusError::from(format!("`{}` requires a value", arg)));

            match arg.as_str() {
                "sub" | "subscribe" => msg_type = ClientMsgType::Subscribe,
                "u" | "update" | "n" | "notify" => msg_type = ClientMsgType::Update,
                "p" | "primary" => collection = Collection::Primary,
                "s" | "secondary" => collection = Collection::Secondary,
                "t" | "tertiary" => collection = Collection::Tertiary,
                "a" | "all" => collection = Collection::All,

                "-p" | "--primary-color" => result.formatter.set_primary_color(&extract_next_value()?)?,
                "-s" | "--secondary-color" => result.formatter.set_secondary_color(&extract_next_value()?)?,
                "-t" | "--font" | "--text-font" => result.formatter.set_text_font(&extract_next_value()?),
                "-i" | "--icon-font" => result.formatter.set_icon_font(&extract_next_value()?),
                "-m" | "--mode" => result.formatter.set_format_mode(extract_next_value()?.parse()?),
                "-f" | "--force" => result.force = true,
                _ => {
                    if arg.starts_with('-') {
                        eprintln!("heads up: `{}` is not a flag muse-status recognizes, but we'll go on anyways", arg)
                    } else {
                        match collection {
                            Collection::One(o) => collection = Collection::Many(vec![o, arg]),
                            Collection::Many(ref mut m) => m.push(arg),
                            _ => collection = Collection::One(arg)
                        }
                    }
                },
            }
        }

        result.client_msg = match msg_type {
            ClientMsgType::Subscribe => ClientMsg::Subscribe(collection),
            ClientMsgType::Update => ClientMsg::Update(collection),
        };

        Ok(result)
    }
}
