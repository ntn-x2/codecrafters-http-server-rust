use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let string = {
                    let mut buf = BufReader::new(&mut stream);
                    let mut s = String::new();
                    buf.read_line(&mut s).unwrap();
                    s
                };
                let (request, _headers) = {
                    let mut parts = string.split("\r\n");
                    (
                        parts.next().unwrap().to_owned(),
                        parts.next().unwrap().split("\r\n"),
                    )
                };
                let (_method, path, _http_version) = {
                    let mut components = request.split_ascii_whitespace();
                    (
                        components.next().unwrap(),
                        components.next().unwrap(),
                        components.next().unwrap(),
                    )
                };
                let res = match path {
                    "/" => b"HTTP/1.1 200 OK\r\n\r\n".as_slice(),
                    _ => b"HTTP/1.1 404 Not Found\r\n\r\n".as_slice(),
                };
                stream
                    .write_all(res)
                    .expect("Failed to send resp to client.");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
