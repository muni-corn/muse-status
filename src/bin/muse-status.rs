use std::io::Write;
use std::io;
use std::io::BufRead;
use std::net::TcpStream;
use std::process;
use std::env;
use muse_status::daemon::Action;
use serde::Serialize;

fn main() {
    let action = if let Some(first_arg) = env::args().nth(1) {
        let s = env::args().skip(1).collect::<Vec<String>>().join(" ");
        if let Some(first_char) = first_arg.chars().next() {
            if first_char == '-' {
                Action::Flags(Some(s))
            } else {
                Action::Command(s)
            }
        } else {
            Action::Flags(None)
        }
    } else {
        Action::Flags(None)
    };

    // start
    let mut stream = loop {
        if let Ok(s) = TcpStream::connect("localhost:1612") {
            break s
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    };

    stream.write_all(format!("{}\n", serde_json::to_string(&action).unwrap()).as_bytes()).unwrap();

    if let Action::Flags(_) = &action {
        start_listening(stream)
    } else {
        return
    }

    println!(r#"[{{name: "error", long_text: "disconnected", short_text: "disconnected"}}]"#)
}

fn start_listening(stream: TcpStream) {
    // create a buffered stream, which we'll read from line by line for status outputs
    let mut buf_stream = io::BufReader::new(stream);

    // listen for outputs from the daemon and print them
    loop {
        let mut s = String::new();
        match buf_stream.read_line(&mut s) {
            Ok(n) => {
                if n == 0 {
                    return
                }
                print!("{}", s);
            }
            Err(_) => return,
        }
    };
}
