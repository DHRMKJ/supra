use std::sync::Arc;
use std::time::{Duration, Instant};
use ed25519_dalek::{Signer, SigningKey, Verifier};
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use crate::BTC;


pub async fn send_messages(write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>, message_rec: Arc<Mutex<Receiver<BTC>>>, alt_sender: Arc<Sender<BTC>>, secs: u16, sender_keypair: Arc<SigningKey>) {
    let json_request = json!({
        "id": "fsdlljlksfadjkfssdffsdfsd",
        "method": "ticker.price",
        "params": {
            "symbol": "BTCUSDT"
        }
    });

    let str_request = serde_json::to_string(&json_request).expect("[ERROR]: invalid json");
    let start_time = Instant::now();
    let duration = Duration::from_secs(secs as u64);
    let mut interval = time::interval(Duration::from_secs(1));

    let mut average = 0.0f64;
    let mut cnt = 0;

    while Instant::now() - start_time < duration {
        let mut guard = write.lock().await;
        guard.send(Message::Text(str_request.clone())).await.expect("Failed to send message");

        let mut message_guard = message_rec.lock().await;
        if let Some(message) = message_guard.recv().await {
            match message {
                BTC::Price(x, signature) => {
                    assert!(sender_keypair.verifying_key().verify(x.to_string().as_bytes(), &signature).is_ok());
                    cnt += 1;
                    average += x.parse::<f64>().unwrap();
                }
                BTC::Err => {
                    println!("error receiving the price!")
                }
            }
        }
        interval.tick().await;
    }
    let price = (average / cnt as f64).to_string();
    let btc_price = BTC::Price(price.clone(), sender_keypair.sign(price.as_bytes()));
    let _ = alt_sender.send(btc_price).await.unwrap_or_else(|e| {
        eprintln!("[ERROR]: sending average price! {:?}", e);
    });
}
