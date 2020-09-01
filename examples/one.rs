extern crate coinbase_pro_one_rs;

#[macro_use]
extern crate log;

use async_std::{task};
use std::fs::File;
use std::io::BufReader;

use coinbase_pro_one_rs::*;
use coinbase_pro_one_rs::book::MsgHarvester;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open("./credentials.json")?;
    let reader = BufReader::new(file);
    let credentials: Option<structs::Credentials> =
        serde_json::from_reader(reader)?;

    let (url, ws_url) = if credentials.is_some() {
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
            url, ws_url, credentials).await;

        let mut btc_order_book: Box<dyn MsgHarvester> = Box::new(book::l2::OrderBook::new("BTC-USD".to_string()));
        let mut ticker: Box<dyn MsgHarvester> = Box::new(book::ticker::Ticker::new("BTC-USD".to_string()));
        let product_ids = vec!("BTC-USD".to_string());

        conduit.interval(2500);                 // NONE
        conduit.level(structs::Level::Level2).await;  // WS
        conduit.ticker(product_ids).await;            // WS
        conduit.time().await;                         // HTTP
        conduit.heartbeat().await;                    // WS
        while let msg = mailbox.recv().await {
            match msg {
                Err(e) => debug!("Match _msg err: {:?}", e),
                Ok(m) => {
                    // Thread the `m` through the MsgHarvesters.
                    let msg = book::harvest(m, vec!(
                        &mut btc_order_book,
                        &mut ticker,
                    ));
                    match msg {
                        Some(structs::Message::Interval(..)) |
                        Some(structs::Message::WSHeartbeat { .. }) => {
                            /* Don't output these.  Noisy.
                                These messages can be used to kick off async processing. */
                            debug!("Ignored...{:?}", &msg);
                        },
                        None => {},
                        _ => debug!("{:?}\n", &msg)
                    };

                }
            }
        };
    });
    println!("EXITING...");
    Ok(())
}

