use core::str;
use std::fs::read_to_string;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Opt {
    #[structopt(long, parse(from_os_str), default_value = "")]
    pub _directory: PathBuf,
}

const RES_404: &[u8] = b"HTTP/1.1 404 Not Found\r\n\r\n";
const RES_201: &[u8] = b"HTTP/1.1 201 Created\r\n\r\n";

fn main() {
    let opt = Opt::from_args();
    let current_directory = Box::leak(Box::new(opt._directory));

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                std::thread::spawn(|| {
                    handle_client(_stream, current_directory);
                });
                println!("accepted new connection");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum RequestMethod {
    GET,
    POST,
}

#[derive(Debug, Clone)]
struct RequestHeader {
    pub method: RequestMethod,
    pub host: String,
    pub user_agent: String,
    pub accept_encoding: String,
    pub accept: String,
    pub path: String,
}

impl Default for RequestHeader {
    fn default() -> Self {
        RequestHeader {
            method: RequestMethod::GET,
            host: String::default(),
            user_agent: String::default(),
            accept_encoding: String::default(),
            accept: String::default(),
            path: String::default(),
        }
    }
}

fn parse_request_header(lines: &[&str]) -> Option<RequestHeader> {
    let mut parsed_header = RequestHeader::default();

    // Request Header Method section
    // Parse the first line of the http header

    for line in lines.iter() {
        if let Some((key, value)) = line.split_once(' ') {
            match key {
                "POST" => {
                    parsed_header.method = RequestMethod::POST;
                    parsed_header.path = value.split(' ').next()?.to_string();
                }
                "GET" => {
                    parsed_header.method = RequestMethod::GET;
                    parsed_header.path = value.split(' ').next()?.to_string();
                }
                "Host:" => {
                    parsed_header.host = value.to_string();
                }

                "User-Agent:" => {
                    parsed_header.user_agent = value.to_string();
                }

                "Accept:" => {
                    parsed_header.accept = value.to_string();
                }
                // TODO: Implement multiple encoding protocol
                "Accept-Encoding:" => {
                    let encodings: Vec<&str> = value.split(", ").collect();
                    for encoding in encodings.iter() {
                        if *encoding == "gzip" {
                            parsed_header.accept_encoding = value.to_string();
                        }
                    }
                }

                _default => return Some(parsed_header),
            }
        }
    }

    Some(parsed_header)
}

fn handle_client(mut stream: TcpStream, current_directory: &Path) {
    let mut buf = [0; 256];
    let length = stream.read(&mut buf).unwrap();
    let input = String::from_utf8(buf[..length].to_vec()).unwrap();
    let lines: Vec<_> = input.split("\r\n").collect();

    // Parse the header buffer
    let parsed_header = parse_request_header(&lines).unwrap();

    match parsed_header.method {
        RequestMethod::POST => {
            if let Some(filename) = parsed_header.path.strip_prefix("/files/") {
                let file_path = current_directory.join(filename);
                if let Ok(mut file) = std::fs::File::create(file_path) {
                    file.write_all(lines.last().unwrap().as_bytes()).unwrap();
                    stream.write_all(RES_201).unwrap();
                } else {
                    stream.write_all(RES_404).unwrap();
                }
            }
        }
        RequestMethod::GET => {
            if parsed_header.path == "/" {
                stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
            } else if let Some(content) = parsed_header.path.strip_prefix("/echo/") {
                stream
                    .write_all(
                        generate_response(content, "text/plain", &parsed_header.accept_encoding)
                            .as_bytes(),
                    )
                    .unwrap();
            } else if parsed_header.path == "/user-agent" {
                stream
                    .write_all(
                        generate_response(
                            &parsed_header.user_agent,
                            "text/plain",
                            &parsed_header.accept_encoding,
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            } else if let Some(filename) = parsed_header.path.strip_prefix("/files/") {
                let file_path = current_directory.join(filename);
                if let Ok(content) = read_to_string(file_path) {
                    stream
                        .write_all(
                            generate_response(
                                &content,
                                "application/octet-stream",
                                &parsed_header.accept_encoding,
                            )
                            .as_bytes(),
                        )
                        .unwrap();
                } else {
                    stream.write_all(RES_404).unwrap();
                }
            } else {
                stream.write_all(RES_404).unwrap();
            }
        }
        _default => {}
    }
}

fn generate_response(text: &str, content_type: &str, content_encoding: &str) -> String {
    if content_encoding.is_empty() {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}\r\n",
            content_type,
            text.len(),
            text
        )
    } else {
        format!(
        "HTTP/1.1 200 OK\r\nContent-Encoding: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}\r\n",
        content_encoding,
        content_type,
        text.len(),
        text
    )
    }
}
