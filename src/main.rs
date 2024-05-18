use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
};

fn echo_response(echo_payload: &str) -> Vec<u8> {
    let payload_size = echo_payload.len();
    let response_body = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {payload_size}\r\n\r\n{echo_payload}");
    response_body.into_bytes()
}

fn user_agent_response(user_agent: &str) -> Vec<u8> {
    let payload_size = user_agent.len();
    let response_body = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {payload_size}\r\n\r\n{user_agent}");
    response_body.into_bytes()
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let (request, mut headers) = {
                    let mut buf_reader = BufReader::new(stream.try_clone().unwrap());
                    let mut lines = Vec::new();
                    loop {
                        let mut tmp_buffer = String::new();
                        buf_reader.read_line(&mut tmp_buffer).unwrap();
                        let trimmed_tmp_buffer = tmp_buffer.trim();
                        if trimmed_tmp_buffer.is_empty() {
                            break;
                        }
                        lines.push(trimmed_tmp_buffer.to_string());
                    }
                    #[allow(clippy::unnecessary_to_owned)]
                    (lines[0].clone(), lines[1..].to_owned().into_iter())
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
                    "/user-agent" => {
                        let user_agent_header = headers
                            .find_map(|h| h.strip_prefix("User-Agent: ").map(|s| s.to_string()))
                            .unwrap();
                        user_agent_response(&user_agent_header)
                    }
                    s if s.starts_with("/echo/") => echo_response(s.split_at("/echo/".len()).1),
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
