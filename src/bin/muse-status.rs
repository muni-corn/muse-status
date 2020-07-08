use muse_status::daemon::{ClientMsg, DataOutput};
use muse_status::errors::MuseStatusError;
use muse_status::format::{Formatter, Mode};
use std::env;
use std::error::Error;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::net::TcpStream;

fn main() {
    let action = if let Some(first_arg) = env::args().nth(1) {
        let s = env::args().skip(1).collect::<Vec<String>>().join(" ");
        if let Some(first_char) = first_arg.chars().next() {
            if first_char == '-' {
                ClientMsg::Connect
            } else {
                ClientMsg::Command(s)
            }
        } else {
            ClientMsg::Connect
        }
    } else {
        ClientMsg::Connect
    };

    // start loop. muse-status will try listening for the daemon again if it is disconnected
    let mut stream = get_daemon_connection();
    stream
        .write_all(format!("{}\n", serde_json::to_string(&action).unwrap()).as_bytes())
        .unwrap();

    if let ClientMsg::Connect = action {
        // the client will connect to the daemon and output data updates.

        // get the formatter
        let formatter =
            formatter_from_flags(&env::args().skip(1).collect::<Vec<String>>()).unwrap(); // unwrap is dumb but ok

        // initialize output
        if formatter.get_format_mode() == &Mode::JsonProtocol {
            println!("{{\"version\":1}}");
            println!("[[]");
        }

        loop {
            start_listening(stream, &formatter);

            // if connection is lost, connect again
            stream = get_daemon_connection();
        }
    }
}

fn get_daemon_connection() -> TcpStream {
    loop {
        if let Ok(s) = TcpStream::connect("localhost:1612") {
            return s;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn start_listening(stream: TcpStream, formatter: &Formatter) {
    // create a buffered stream, which we'll read from line by line for status outputs
    let mut buf_stream = io::BufReader::new(stream);

    // listen for outputs from the daemon and print them
    loop {
        let mut s = String::new();
        match buf_stream.read_line(&mut s) {
            Ok(n) => {
                if n == 0 {
                    return;
                } else if let Err(e) = echo_output(&s, &formatter) {
                    echo_error(e, &formatter);
                }
            }
            Err(e) => echo_error(e, &formatter),
        }
    }
}

/// Parses flags passed to the `muse-status` command and returns a new Formatter. Results in an
/// Error if argument parsing failed.
fn formatter_from_flags(flags: &[String]) -> Result<Formatter, MuseStatusError> {
    let mut iter = flags.iter();

    let mut formatter: Formatter = Default::default();

    while let Some(flag) = iter.next() {
        if let Some(value) = iter.next() {
            match flag.as_str() {
                "-p" | "--primary-color" => {
                    formatter.set_primary_color(&value)?;
                }
                "-s" | "--secondary-color" => {
                    formatter.set_secondary_color(&value)?;
                }
                "-f" | "--font" => {
                    formatter.set_text_font(&value);
                }
                "-i" | "--icon-font" => {
                    formatter.set_icon_font(&value);
                }
                "-m" | "--mode" => match value.as_str() {
                    "i3" => {
                        formatter.set_format_mode(Mode::JsonProtocol);
                    }
                    "lemon" => {
                        formatter.set_format_mode(Mode::Lemonbar);
                    }
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }
    }

    Ok(formatter)
}

/// Prints formatted output.
fn echo_output(raw_data: &str, f: &Formatter) -> Result<(), MuseStatusError> {
    let data = serde_json::from_str::<DataOutput>(raw_data)?;
    println!("{}", f.format_data(data));
    Ok(())
}

/// Prints formatted error.
fn echo_error<E: Error>(e: E, f: &Formatter) {
    println!("{}", f.format_error(e));
}
