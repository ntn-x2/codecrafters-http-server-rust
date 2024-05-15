use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
};

fn return_echo_response(echo_payload: &str) -> Vec<u8> {
    let payload_size = echo_payload.len();
    let response_body = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {payload_size}\r\n\r\n{echo_payload}");
    response_body.into_bytes()
}

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
                    "/" => b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
                    s if s.starts_with("/echo/") => {
                        return_echo_response(s.split_at("/echo/".len()).1)
                    }
                    _ => b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec(),
                };
                stream
                    .write_all(res.as_slice())
                    .expect("Failed to send resp to client.");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
