extern crate coinbase_pro_one_rs;
#[macro_use]
extern crate log;

use async_std::{task,
                sync::{Arc, Mutex}};

use coinbase_pro_one_rs::*;
use coinbase_pro_one_rs::book::l2::OrderBook;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Sync
    env_logger::init();
    debug!("One: starting");

    // Async so that conduit methods aren't wait-y.
    task::block_on(async {
        let (mut conduit, mailbox) = client::Conduit::new(
            SANDBOX_URL, WS_SANDBOX_URL, None).await;
        let btc_order_book =
            Arc::new(Mutex::new(OrderBook::new("BTC-USD".to_string())));

        let product_ids = vec!("BTC-USD".to_string());
        conduit.level2_ws().await;         // WS
        conduit.ticker(product_ids).await; // WS
        conduit.time().await;              // HTTP
        conduit.heartbeat().await;         //WS
        conduit.interval(2500);
        loop {
            btc_order_book.lock().await
                .harvest(&mailbox).await
                .map(|msg: structs::Message| {
                debug!("One: receiver.next: {:?}\n", &msg);
            });
        }
    });
    println!("EXITING...");
    Ok(())
}

