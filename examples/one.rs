extern crate coinbase_pro_one_rs;

#[macro_use]
extern crate log;

use async_std::sync::Arc;
use async_std::{task};
use futures::{StreamExt, TryFutureExt};
use std::fs::File;
use std::io::BufReader;

use coinbase_pro_one_rs::*;
use std::sync::Mutex;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open("./credentials.json")?;
    let reader = BufReader::new(file);
    let credentials: Option<structs::Credentials> =
        serde_json::from_reader(reader)?;

    let (_URL, _WS_URL) = if credentials.is_some() {
        (MAIN_URL, WS_URL)
    } else {
        (SANDBOX_URL, WS_SANDBOX_URL)
    };
    // Sync
    env_logger::init();
    debug!("One: starting");

    // Async so that conduit methods aren't wait-y.
    task::block_on(async {
        let (mut conduit, mailbox) = conduit::Conduit::new(
            _URL, _WS_URL, credentials).await;
        let btc_order_book =
            Arc::new(Mutex::new(book::l2::OrderBook::new("BTC-USD".to_string())));
        let ticker =
            Arc::new(Mutex::new(book::ticker::Ticker::new("BTC-USD".to_string())));
        let product_ids = vec!("BTC-USD".to_string());
        conduit.interval(2500);            // NONE
        conduit.level2().await;            // WS
        conduit.ticker(product_ids).await; // WS
        conduit.time().await;              // HTTP
        conduit.heartbeat().await;         // WS
        while let _msg = mailbox.recv() {
            let _ = _msg.and_then(|msg| async move { btc_order_book.lock().await.harvest(Some(msg)) })
                .and_then(|msg| async move { ticker.lock().await.harvest(msg) })
                .and_then(|msg| {
                    match msg {
                        structs::Message::Interval(..) |
                        structs::Message::WSHeartbeat { .. }
                        => {
                            /* Don't output these.  Noisy.
                                These messages can be used to kick off async processing. */
                        },
                        _ => info!("{:?}\n", &msg)
                    };
                    Ok(())
                });
        }
    });
    println!("EXITING...");
    Ok(())
}

