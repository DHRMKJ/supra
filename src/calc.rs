use std::fs::File;
use std::io;
use std::io::Write;
use std::sync::Arc;
use ed25519_dalek::{SigningKey, Verifier};
use tokio::sync::mpsc::Receiver;
use crate::BTC;

fn create_output(averages: Vec<f64>) -> io::Result<()> {
    let mut file = File::create("output.txt")?;
    for (index, &value) in averages.iter().enumerate() {
        let line = format!("{}. {}\n", index, value);
        file.write_all(line.as_bytes())?;
    }
    let line  = format!("Average: {}", averages.iter().sum::<f64>() / averages.len() as f64 );
    file.write_all(line.as_bytes())?;
    Ok(())
}
pub async fn calc_average_prices(mut receiver: Receiver<BTC>, sender_keypair: Arc<SigningKey>) {
    let mut average: Vec<f64> = vec![];

    loop {
        if let Some(message) = receiver.recv().await {
            match message {
                BTC::Price(x, signature) => {
                    let vk = sender_keypair.verifying_key();
                    assert!(vk.verify(x.to_string().as_bytes(), &signature).is_ok());
                    average.push(x.parse::<f64>().unwrap());
                },
                BTC::Err => {
                    break;
                }
            }
        };
    }
    if let Ok(_) = create_output(average.clone()) {
        println!("Cache complete. The average USD price of BTC is: {}", average.iter().sum::<f64>() / average.len() as f64);
    }else {
        eprintln!("Failed to cache.");
    }
}
