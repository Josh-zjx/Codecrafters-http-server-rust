// Uncomment this block to pass the first stage
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;

//const NOT_FOUND: String = String::from("HTTP/1.1 404 Not Found\r\n");

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
                std::thread::spawn(|| {
                    handle_client(_stream);
                });
                //handle_client(_stream);
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
    let first_line: Vec<_> = lines.first().unwrap().split(' ').collect();
    let path = first_line.get(1).unwrap();
    if path == &"/" {
        stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
    } else if let Some(content) = path.strip_prefix("/echo/") {
        stream
            .write_all(generate_response(200, content).as_bytes())
            .unwrap();
    } else if path == &"/user-agent" {
        let (status, content) = if let Some(line) = lines.get(2) {
            if let Some(content) = line.strip_prefix("User-Agent: ") {
                (200, content)
            } else {
                (404, "")
            }
        } else {
            (404, "")
        };
        stream
            .write_all(generate_response(status, content).as_bytes())
            .unwrap();
    } else {
        stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
    }
}

fn generate_response(status: usize, text: &str) -> String {
    if status == 404 {
        "HTTP/1.1 404 Not Found\r\n\r\n".to_string()
    } else {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n",
            text.len(),
            text
        )
    }
}
