use std::io;
use std::io::BufRead;
use std::net;

fn main() {
    let stream = match net::TcpStream::connect(":1612") {
        Ok(s) => s,
        Err(e) => {
            println!("couldn't connect to the daemon: {}", e);
            return;
        }
    };

    let mut buf_stream = io::BufReader::new(stream);

    let e = loop {
        let mut s = String::new();
        match buf_stream.read_line(&mut s) {
            Ok(l) => {
                println!("{}", l);
            }
            Err(e) => break e,
        }
    };

    println!("an error has stopped muse-status: {}", e)
}
