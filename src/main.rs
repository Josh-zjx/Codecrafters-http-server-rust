use bytes::Bytes;
use flate2::write::DeflateEncoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use nom::AsBytes;
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
                #[cfg(debug_assertions)]
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
struct RequestHeader<'a> {
    pub method: RequestMethod,
    pub host: &'a str,
    pub user_agent: &'a str,
    pub accept_encoding: &'a str,
    pub accept: &'a str,
    pub path: &'a str,
}

impl Default for RequestHeader<'_> {
    fn default() -> Self {
        RequestHeader {
            method: RequestMethod::GET,
            host: "",
            user_agent: "",
            accept_encoding: "",
            accept: "",
            path: "",
        }
    }
}

/// Parse the request header
fn parse_request_header<'a>(lines: &'a [&str]) -> Option<RequestHeader<'a>> {
    let mut parsed_header = RequestHeader::default();

    for line in lines.iter() {
        if let Some((key, value)) = line.split_once(' ') {
            match key {
                "POST" => {
                    parsed_header.method = RequestMethod::POST;
                    parsed_header.path = value.split(' ').next()?;
                }
                "GET" => {
                    parsed_header.method = RequestMethod::GET;
                    parsed_header.path = value.split(' ').next()?;
                }
                "Host:" => {
                    parsed_header.host = value;
                }

                "User-Agent:" => {
                    parsed_header.user_agent = value;
                }

                "Accept:" => {
                    parsed_header.accept = value;
                }
                // TODO: Implement multiple encoding protocol
                "Accept-Encoding:" => {
                    let encodings: Vec<&str> = value.split(", ").collect();
                    for encoding in encodings.iter() {
                        if *encoding == "gzip" {
                            parsed_header.accept_encoding = "gzip";
                        } else if *encoding == "deflate" {
                            parsed_header.accept_encoding = "deflate";
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
                    .write_all(&generate_response(
                        content,
                        "text/plain",
                        parsed_header.accept_encoding,
                    ))
                    .unwrap();
            } else if parsed_header.path == "/user-agent" {
                stream
                    .write_all(&generate_response(
                        parsed_header.user_agent,
                        "text/plain",
                        parsed_header.accept_encoding,
                    ))
                    .unwrap();
            } else if let Some(filename) = parsed_header.path.strip_prefix("/files/") {
                let file_path = current_directory.join(filename);
                if let Ok(content) = read_to_string(file_path) {
                    stream
                        .write_all(&generate_response(
                            &content,
                            "application/octet-stream",
                            parsed_header.accept_encoding,
                        ))
                        .unwrap();
                } else {
                    stream.write_all(RES_404).unwrap();
                }
            } else {
                stream.write_all(RES_404).unwrap();
            }
        } //_default => {}
    }
}

fn generate_response(text: &str, content_type: &str, content_encoding: &str) -> Bytes {
    if content_encoding == "gzip" {
        // Using Gzip compression if acceptable

        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        let _ = e.write_all(text.as_bytes());
        let compressed_text = e.finish().unwrap();
        let response_header = Bytes::from(format!(
        "HTTP/1.1 200 OK\r\nContent-Encoding: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
        content_encoding,
        content_type,
        compressed_text.len(),
    ));
        [
            response_header,
            Bytes::from(compressed_text),
            Bytes::from("\r\n"),
        ]
        .concat()
        .into()
    } else if content_encoding == "deflate" {
        // using deflate is acceptable
        //
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        let _ = e.write_all(text.as_bytes());
        let compressed_text = e.finish().unwrap();
        let response_header = Bytes::from(format!(
        "HTTP/1.1 200 OK\r\nContent-Encoding: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
        content_encoding,
        content_type,
        compressed_text.len(),
    ));
        [
            response_header,
            Bytes::from(compressed_text),
            Bytes::from("\r\n"),
        ]
        .concat()
        .into()
    } else {
        // fallback to non-compression if no compression supported
        Bytes::from(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}\r\n",
            content_type,
            text.len(),
            text
        ))
    }
}
