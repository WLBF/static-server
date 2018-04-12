extern crate httparse;

mod thread_pool;

use thread_pool::ThreadPool;

use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::fs;
use std::path::PathBuf;
use std::env;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8910").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let res = req.parse(&buffer).unwrap();

    if res.is_complete() {
        match req.path {
            Some(ref path) => {
                let uri = PathBuf::from(path);
                let dir = env::current_dir().unwrap();
                let mut path = dir.join(uri.strip_prefix("/").unwrap());

                println!("{:?}", path);

                match (path.exists(), path.is_file(), path.is_dir()) {
                    (true, true, false) => handle_file_request(path, &mut stream),
                    (true, false, true) => handle_dir_request(path, &mut stream),
                    (_, _, _) => handle_not_found(&mut stream),
                };
                stream.flush().unwrap();
            }
            None => {
                // TODO: read more and parse again
            }
        }
    }
}

fn handle_file_request(path: PathBuf, stream: &mut TcpStream) {
    let mut file = fs::File::open(path).unwrap();
    let metadata = file.metadata().unwrap();
    let length = metadata.len();
    let mut sent: u64 = 0;
    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", length);
    stream.write(response.as_bytes()).unwrap();

    let mut content = [0; 1024];
    
    while sent < length {
        let chunk = file.read(&mut content).unwrap();
        stream.write(&content[..chunk]).unwrap();
        sent += chunk as u64;
    }
}

fn handle_dir_request(path: PathBuf, stream: &mut TcpStream) {
    let paths = fs::read_dir(path).unwrap();

    let list = paths.map(|entry| {
        let entry = entry.unwrap();
        let mut name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().unwrap().is_dir() {
            name.push('/');
        }
        format!("<li><a href=\"{name}\">{name}</a>", name=name)
    }).collect::<Vec<_>>();

    let response = format!("HTTP/1.1 200 OK\r\n\r\n
        <html>
        <title>Directory listing for /</title>
        <body>
        <h2>Directory listing for /</h2>
        <hr>
        <ul>
        {}
        </ul>
        <hr>
        </body>
        </html>", list.join(""));
    stream.write(response.as_bytes()).unwrap();
}

fn handle_not_found(stream: &mut TcpStream) {
    stream.write("HTTP/1.1 404 NOT FOUND\r\n\r\n".as_bytes()).unwrap();
}
