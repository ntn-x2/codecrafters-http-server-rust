use std::{io::Write, net::TcpListener};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let resp = b"HTTP/1.1 200 OK\r\n\r\n";
                stream
                    .write_all(resp)
                    .expect("Failed to send resp to client.");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
