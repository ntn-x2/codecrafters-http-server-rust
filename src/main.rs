use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    str::FromStr,
    thread,
};

enum AcceptType {
    Gzip,
}

impl FromStr for AcceptType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gzip" => Ok(Self::Gzip),
            _ => Err("Invalid value for Accept-Type header."),
        }
    }
}

fn echo_response(echo_payload: &str, accept_type: Option<&str>) -> Vec<u8> {
    let payload_size = echo_payload.len();
    let content_encoding_header = if let Some(accept_type) = accept_type {
        let mut types = accept_type.split(',').map(|s| s.trim());
        if let Some(supported_type) = types.find(|t: &&str| t.parse::<AcceptType>().is_ok()) {
            format!("\r\nContent-Encoding: {supported_type}")
        } else {
            "".to_owned()
        }
    } else {
        "".to_owned()
    };
    let response_body = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {payload_size}{}\r\n\r\n{echo_payload}", content_encoding_header);
    response_body.into_bytes()
}

fn user_agent_response(user_agent: &str) -> Vec<u8> {
    let payload_size = user_agent.len();
    let response_body = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {payload_size}\r\n\r\n{user_agent}");
    response_body.into_bytes()
}

fn get_file_response(file_path: &str) -> Vec<u8> {
    let Ok(mut file) = File::open(file_path) else {
        return b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec();
    };
    let file_content = {
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf.into_bytes()
    };

    {
        let file_size = file_content.len();
        let response_without_body = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {file_size}\r\n\r\n").into_bytes();
        response_without_body
            .into_iter()
            .chain(file_content)
            .collect::<Vec<_>>()
    }
}

fn post_file_response(file_path: &str, file_contents: Vec<u8>) -> Vec<u8> {
    let mut file = File::create(file_path).expect("Cannot create file at path {file_path}");
    file.write_all(file_contents.as_slice())
        .expect("Cannot write provided content into file.");

    b"HTTP/1.1 201 Created\r\n\r\n".to_vec()
}

struct HttpRequest {
    method: HttpMethod,
    path: String,
    #[allow(dead_code)]
    version: String,
    headers: HashMap<String, String>,
    #[allow(dead_code)]
    body: Option<Vec<u8>>,
}

impl HttpRequest {
    fn from_reader<R: Read>(r: R) -> Self {
        let mut buf_reader = BufReader::new(r);
        let (raw_method, path, version) = {
            let mut buffer = String::new();
            buf_reader
                .read_line(&mut buffer)
                .expect("Malformed HTTP request. No start line found.");
            let mut iter = buffer.trim().split_ascii_whitespace();
            (
                iter.next().expect("No HTTP method found.").to_string(),
                iter.next().expect("No path found.").to_string(),
                iter.next().expect("No HTTP version found.").to_string(),
            )
        };
        let method = raw_method.parse().expect("Unsupported HTTP method found.");
        let headers = {
            let mut buffer = String::new();
            let mut map = HashMap::<String, String>::default();
            loop {
                buf_reader
                    .read_line(&mut buffer)
                    .expect("Malformed HTTP request. No start line found.");
                let trimmed_string = buffer.trim();
                if trimmed_string.is_empty() {
                    break;
                }
                let (name, value) = trimmed_string
                    .split_once(": ")
                    .expect("Failed to read header.");
                map.insert(name.to_lowercase().to_string(), value.to_string());
                buffer.clear();
            }
            map
        };
        let body = {
            let content_length = {
                if let Some(raw_cl) = headers.get("content-length") {
                    raw_cl
                        .parse::<usize>()
                        .expect("Invalid value for header \"Content-Length\"")
                } else {
                    0
                }
            };
            if content_length > 0 {
                let mut buffer = vec![0; content_length];
                buf_reader.read_exact(buffer.as_mut_slice()).unwrap();
                Some(buffer)
            } else {
                None
            }
        };
        Self {
            method,
            path,
            version,
            headers,
            body,
        }
    }
}

enum HttpMethod {
    Get,
    Post,
}

impl FromStr for HttpMethod {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            _ => Err("Invalid HTTP method found."),
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    {
        let args = env::args().collect::<Vec<_>>();
        let (directory_flag, directory_path) = (args.get(1), args.get(2));
        match (directory_flag, directory_path) {
            (Some(flag), Some(path)) if flag == "--directory" => {
                File::open(path).expect("No directory found at the specified path.");
            }
            _ => {}
        }
    }

    for stream in listener.incoming() {
        thread::spawn(|| match stream {
            Ok(mut stream) => {
                let req = HttpRequest::from_reader(stream.try_clone().unwrap());
                let res = match req.path.as_str() {
                    "/" => b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
                    "/user-agent" => {
                        let user_agent_header = req
                            .headers
                            .get("user-agent")
                            .expect("No User-Agent header found.");
                        user_agent_response(user_agent_header.as_str())
                    }
                    s if s.starts_with("/echo/") => {
                        let accept_encoding_header =
                            req.headers.get("accept-encoding").map(|s| s.as_str());
                        echo_response(s.strip_prefix("/echo/").unwrap(), accept_encoding_header)
                    }
                    s if s.starts_with("/files/") => {
                        let args = env::args().collect::<Vec<_>>();
                        let directory_path = {
                            let (directory_flag, directory_path) = (args.get(1), args.get(2));
                            match (directory_flag, directory_path) {
                                (Some(flag), Some(path)) if flag == "--directory" => path,
                                _ => panic!("Invalid flag provided."),
                            }
                        };
                        let file_name = s.strip_prefix("/files/").unwrap();
                        match req.method {
                            HttpMethod::Get => {
                                get_file_response(format!("{directory_path}/{file_name}").as_str())
                            }
                            HttpMethod::Post => post_file_response(
                                format!("{directory_path}/{file_name}").as_str(),
                                req.body.unwrap_or_default(),
                            ),
                        }
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
        });
    }
}
