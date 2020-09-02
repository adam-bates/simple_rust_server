use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::fs;
use std::thread;
use std::time::Duration;
use std::boxed::Box;
use std::env;
use simple_rust_server::{ThreadPool, HelloError};

const LOCALHOST: &str = "127.0.0.1";
const PORT_DEFAULT: &str = "7878";
const THREAD_POOL_SIZE_DEFAULT: usize = 4;
const REQUESTS_LIMIT_DEFAULT: Limit = Limit::None;

fn main() -> Result<(), Box<dyn HelloError>> {
    let config = parse_config(env::args().collect());
    println!("Using config: {:?}", config);

    let address = format!("{}:{}", config.ip, config.port);
    let listener = TcpListener::bind(address).unwrap();
    let pool = ThreadPool::new(config.thread_pool_size)?;
    
    let request_stream: Box<dyn Iterator<Item = std::io::Result<TcpStream>>>;

    request_stream = match config.requests_limit {
        Limit::None => Box::new(listener.incoming()),
        Limit::Of(n) => Box::new(listener.incoming().take(n)),
    };
    
    for stream in request_stream {
        let stream = stream.unwrap();
        pool.execute(|| handle_connection(stream));
    }

    println!("Shutting down.");
    Ok(())
}

#[derive(Debug)]
enum Limit {
    Of(usize),
    None,
}

#[derive(Debug)]
struct Config {
    ip: String,
    port: String,
    requests_limit: Limit,
    thread_pool_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ip: String::from(LOCALHOST),
            port: String::from(PORT_DEFAULT),
            requests_limit: REQUESTS_LIMIT_DEFAULT,
            thread_pool_size: THREAD_POOL_SIZE_DEFAULT,
        }
    }
}

fn parse_config(mut args: Vec<String>) -> Config {

    let mut config = Config { ..Default::default() };

    while args.len() > 1 {
        let arg = args.pop().unwrap();
        let arg: Vec<&str> = arg.split("=").collect();
        let query = arg[0];
        let value = arg[1];
        
        match query {
            "ip" => config.ip = String::from(value),
            "port" => config.port = String::from(value),
            "limit" => config.requests_limit = match value.parse::<usize>().unwrap() {
                0 => Limit::None,
                limit => Limit::Of(limit),
            },
            "pool" => config.thread_pool_size = value.parse::<usize>().unwrap(),
            _ => panic!("Expected 'limit=#' or 'pool=#'. Unknown query: {}", query),
        }
    }
    
    config
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else if buffer.starts_with(sleep) {
        thread::sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();
    let response = format!("{}{}", status_line, contents);

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
