use core::str;
use std::fs::read_to_string;
use std::path::Path;
// Uncomment this block to pass the first stage
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
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

fn handle_client(mut stream: TcpStream, current_directory: &Path) {
    let mut buf = [0; 256];
    let length = stream.read(&mut buf).unwrap();
    let input = String::from_utf8(buf[..length].to_vec()).unwrap();
    let lines: Vec<_> = input.split("\r\n").collect();
    let first_line: Vec<_> = lines.first().unwrap().split(' ').collect();
    let operator = first_line.first().unwrap();
    let path = first_line.get(1).unwrap();
    match *operator {
        "POST" => {
            if let Some(filename) = path.strip_prefix("/files/") {
                let file_path = current_directory.join(filename);
                if let Ok(mut file) = std::fs::File::create(file_path) {
                    file.write_all(lines.last().unwrap().as_bytes()).unwrap();
                    stream.write_all(RES_201).unwrap();
                } else {
                    stream.write_all(RES_404).unwrap();
                }
            }
        }
        "GET" => {
            if *path == "/" {
                stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
            } else if let Some(content) = path.strip_prefix("/echo/") {
                stream
                    .write_all(generate_response(content, "text/plain").as_bytes())
                    .unwrap();
            } else if *path == "/user-agent" {
                if let Some(line) = lines.get(2) {
                    if let Some(content) = line.strip_prefix("User-Agent: ") {
                        stream
                            .write_all(generate_response(content, "text/plain").as_bytes())
                            .unwrap();
                    } else {
                        stream.write_all(RES_404).unwrap();
                    }
                } else {
                    stream.write_all(RES_404).unwrap();
                };
            } else if let Some(filename) = path.strip_prefix("/files/") {
                let file_path = current_directory.join(filename);
                if let Ok(content) = read_to_string(file_path) {
                    stream
                        .write_all(
                            generate_response(&content, "application/octet-stream").as_bytes(),
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

fn generate_response(text: &str, content_type: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}\r\n",
        content_type,
        text.len(),
        text
    )
}
