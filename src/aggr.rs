use std::sync::Arc;
use std::time::Instant;
use ed25519_dalek::{Signer, SigningKey};
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use crate::BTC;

pub async fn handle_incoming_messages(read: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>, message_sender: Arc<Sender<BTC>>, sender_keypair: Arc<SigningKey>) {
    let mut guard = read.lock().await;

    while let Some(message) = guard.next().await {
        match message {
            Ok(msg) => {
                match msg {
                    Message::Text(x) => {
                        let json_out: serde_json::Value = serde_json::from_str(&x).expect("[ERR]: error deserializing json");
                        if let Some(result) = json_out.get("result") {
                            if let Some(price) = result.get("price") {
                                if let Some(price_str) = price.as_str() {
                                    let btc_price: String = price_str.to_string();
                                    if let Ok(btc_prc) = btc_price.parse::<f64>() {
                                        let sign = sender_keypair.sign(btc_price.as_bytes());
                                        message_sender.send(BTC::Price(btc_price, sign)).await.unwrap_or_else(|e| {
                                            eprintln!("[ERROR]: error sending price! {:?}", e);
                                        });
                                    } else {
                                        eprintln!("failed price {btc_price}");
                                        message_sender.send(BTC::Err).await.unwrap_or_else(|e| {
                                            eprintln!("[ERROR]: error sending price! {:?}", e);
                                        });
                                    }
                                }
                            }
                        }

                    },
                    Message::Close(_x) => {
                        break;
                    }
                    _ => { }
                }
            },
            Err(e) => eprintln!("Error receiving message: {}", e),
        }
    }
    println!("End time receiver {:?}", Instant::now());
}