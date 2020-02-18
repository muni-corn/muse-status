use std::io::Write;
use std::io;
use std::io::BufRead;
use std::net;
use std::process;
use std::env;

fn main() {
    let mut stream = match net::TcpStream::connect("localhost:1612") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("couldn't connect to the daemon: {}", e);
            return;
        }
    };

    // send a command to the daemon
    let mut command = env::args().skip(1).collect::<Vec<String>>().join(" ");
    command.push('\n'); // end command in newline

    stream.write_all(command.as_bytes()).unwrap();

    if command.trim().is_empty() || command.starts_with('-') {
        // create a buffered stream, which we'll read from line by line for status outputs
        let mut buf_stream = io::BufReader::new(stream);

        // listen for outputs from the daemon and print them
        let e = loop {
            let mut s = String::new();
            match buf_stream.read_line(&mut s) {
                Ok(n) => {
                    if n == 0 {
                        eprintln!("muse-status client read 0 bytes from daemon");
                        process::exit(1);
                    }
                    print!("{}", s);
                }
                Err(e) => break e,
            }
        };

        eprintln!("an error has stopped muse-status: {}", e)
    }
}
