use clap::Parser;
use rand::Rng;
use std::io::prelude::*;
use std::net::{TcpListener, UdpSocket};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 17)]
    port: u16,
    #[arg(short, long)]
    quotes: String,
}

#[derive(Clone)]
struct Quote {
    text: String,
    author: String
}

#[derive(Clone)]
struct CurrentQuote {
    quote: Quote,
    time: std::time::SystemTime,
    quotes: Vec<Quote>,
}

impl CurrentQuote {
    fn get_string(&mut self) -> String {
        let now = std::time::SystemTime::now();
        if now.duration_since(self.time).unwrap().as_secs() > 86400 {
            let mut rng = rand::thread_rng();
            let selected_quote = self.quotes[rng.gen_range(0..self.quotes.len())].clone();
            self.quote = selected_quote;
            self.time = std::time::SystemTime::now();
        }
        self.to_string()
    }
}

impl ToString for Quote {
    fn to_string(&self) -> String {
        format!("'{}' -{}", self.text, self.author)
    }
}

impl ToString for CurrentQuote {
    fn to_string(&self) -> String {
        self.quote.to_string()
    }
}

fn main() {
    let cli = Cli::parse();
    let quotes = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(&cli.quotes[..]).expect("Unable to read quote file given")[..],
    )
    .expect("Unable to parse JSON");
    let arr = quotes["quotes"].as_array().expect("Unable to read JSON array 'quotes'");

    let mut quote_vec: Vec<Quote> = vec!();
    for q in arr.iter()
        .map(|obj| obj.as_object().expect("Unable to read one of 'quotes' JSON object"))
        .map(|obj| Quote{
            text: String::from(obj["quote"].as_str().expect("'quote' object of JSON is not a string or does not exist")), 
            author: String::from(obj["author"].as_str().expect("'author' object of JSON is not a string or does not exist"))}
        ) {
        quote_vec.push(q.clone());
    }
    let selected_quote = quote_vec[0].clone();

    let quote = Arc::new(std::sync::Mutex::new(CurrentQuote {
        quote: selected_quote,
        time: std::time::SystemTime::now(),
        quotes: quote_vec,
    }));

    let tcpquote = quote.clone();
    let tcpthread = std::thread::spawn(move || {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", cli.port)).expect(&format!("Cannot bind tcp 0.0.0.0:{}", cli.port)[..]);
        println!("Listening on tcp port {}", cli.port);
        for stream in listener.incoming() {
            let quote_to_send = match tcpquote.lock() {
                Ok(q) => q.clone().get_string(),
                Err(_) => {continue}
            };
            match stream {
                Err(_) => {}
                Ok(mut client) => {
                    client
                        .write(format!("{}", quote_to_send).as_bytes())
                        .unwrap();
                }
            }
        }
    });

    let udpquote = quote.clone();
    let udpthread = std::thread::spawn(move || {
        let listener = UdpSocket::bind(format!("0.0.0.0:{}", cli.port)).expect(&format!("Cannot bind udp 0.0.0.0:{}", cli.port)[..]);
        println!("Listening on udp port {}", cli.port);
        let mut buf = [0; 10];
        loop {
            let src_addr = match listener.recv_from(&mut buf) {
                Err(_) => {continue},
                Ok((_, addr)) => addr
            };
            let quote_to_send = match udpquote.lock() {
                Ok(q) => q.clone().get_string(),
                Err(_) => {continue}
            };
            match listener
                .send_to(
                    format!("{}", quote_to_send).as_bytes(),
                    src_addr,
                ) {
                Err(_) => {},
                Ok(_) => {}
            }
                
        }
    });

    tcpthread.join().unwrap();
    udpthread.join().unwrap();
}
