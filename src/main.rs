// Uncomment this block to pass the first stage
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    //
    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                handle_client(_stream);
                println!("accepted new connection");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buf = [0; 256];
    let length = stream.read(&mut buf).unwrap();
    let input = String::from_utf8(buf[..length].to_vec()).unwrap();
    let lines: Vec<_> = input.split("\r\n").collect();
    let first_line: Vec<_> = lines.first().unwrap().split(" ").collect();
    if first_line.get(1).unwrap() == &"/" {
        stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
    } else {
        stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
    }
}
