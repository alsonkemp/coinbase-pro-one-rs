extern crate coinbase_pro_one_rs;
#[macro_use]
extern crate log;

use async_std::{task,
                sync::{Arc, Mutex}};
use serde::Deserialize;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use coinbase_pro_one_rs::*;
use coinbase_pro_one_rs::book::l2::OrderBook;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open()?;
    let reader = BufReader::new(file);
    let credentials = serde_json::from_reader(reader)?;

    // Sync
    env_logger::init();
    debug!("One: starting");

    // Async so that conduit methods aren't wait-y.
    task::block_on(async {
        let (mut conduit, mailbox) = conduit::Conduit::new(
            SANDBOX_URL, WS_SANDBOX_URL, credentials).await;
        let btc_order_book =
            Arc::new(Mutex::new(OrderBook::new("BTC-USD".to_string())));

        let product_ids = vec!("BTC-USD".to_string());
        conduit.interval(2500).await;
        conduit.level2().await;            // WS
        conduit.ticker(product_ids).await; // WS
        conduit.time().await;              // HTTP
        conduit.heartbeat().await;         // WS
        loop {
            btc_order_book.lock().await
                .harvest(&mailbox).await
                .map(|msg: structs::Message| {
                info!("One: receiver.next: {:?}\n", &msg);
            });
        }
    });
    println!("EXITING...");
    Ok(())
}

