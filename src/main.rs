mod calc;
mod aggr;
mod req;


use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use clap::{arg, Command, value_parser};
use rand::rngs::OsRng;
use ed25519_dalek::{Signature, SigningKey};
use tokio::sync::Mutex;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use crate::aggr::handle_incoming_messages;
use crate::calc::calc_average_prices;
use crate::req::send_messages;

pub enum BTC {
    Price(String, Signature),
    Err
}

async fn cache_mode(secs: u16) {
    let url = "wss://ws-api.binance.com:443/ws-api/v3";

    println!("Connecting to - {}", url);
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Handshake successful");

    let (write, read) = ws_stream.split();

    let write = Arc::new(Mutex::new(write));
    let read = Arc::new(Mutex::new(read));

    let mut write_handles = vec![];
    let read_clone = Arc::clone(&read);

    let (message_sender,message_rec) = tokio::sync::mpsc::channel::<BTC>(20);
    let (alt_sender, alt_rec) = tokio::sync::mpsc::channel::<BTC>(20);
    let message_sender = Arc::new(message_sender);



    let mut csprng = OsRng;
    let sender_keypair: SigningKey = SigningKey::generate(&mut csprng);

    let mut public_key_file = File::create("pk.txt").expect("Failed to create public key");
    public_key_file.write_all(&sender_keypair.verifying_key().to_bytes()).expect("Failed to create public key");

    let sender_keypair = Arc::new(sender_keypair);
    let skp = Arc::clone(&sender_keypair);
    tokio::spawn(async move {
        handle_incoming_messages(read_clone, message_sender, skp).await;
    });
    let skp = Arc::clone(&sender_keypair);
    let average_handle = tokio::spawn(async move {
        calc_average_prices(alt_rec, skp).await;
    });

    let message_rec = Arc::new(Mutex::new(message_rec));
    let alt_sender = Arc::new(alt_sender);

    for _i in 0..5 {
        let write_stream = Arc::clone(&write);
        let message_rec = Arc::clone(&message_rec);
        let alt_sender = Arc::clone(&alt_sender);
        let skp = Arc::clone(&sender_keypair);
        write_handles.push(tokio::spawn(async move {send_messages(write_stream, message_rec, alt_sender, secs, skp).await;}));
    }

    for handle in write_handles {
        let _ = tokio::try_join!(handle);
    }

    if let Ok(_) = alt_sender.send(BTC::Err).await {
        let _ = tokio::try_join!(average_handle);
    } else {
        eprintln!("[ERROR]: couldn't get the average!");
    };

    let mut write_guard = write.lock().await;
    let _ = write_guard.close().await;
}


#[tokio::main]
async fn main() {
    let mode = Command::new("BTC READER")
        .version("1.0")
        .author("Dhrmk")
        .about("reads and caches btc prices")
        .arg(arg!(--mode <VALUE>)
            .value_parser(["read", "cache"])
            .required(true))
        .arg(arg!(--times <TIME> "time in seconds")
            .value_parser(
                value_parser!(u16).range(1..)
            ))
        .get_matches();

    match mode
          .get_one::<String>("mode")
          .expect("Required mode!")
          .as_str()
    {
        "cache" => {
            match mode
                .get_one::<u16>("times")
            {
                Some(x) => {
                    cache_mode(*x).await;
                },
                None => {
                    eprintln!("required time in seconds!");
                    return;
                }
            }
        },
        "read" => {
            let file = File::open("output.txt").expect("Failed to open file");
            let reader = BufReader::new(file);

            // Iterate through each line in the file and print it to the terminal
            for line in reader.lines() {
                println!("{}", line.unwrap());
            }
        },
        _ => unreachable!()
    }


}