extern crate coinbase_pro_one_rs;
#[macro_use]
extern crate  log;

use std::time::Duration;

use async_std::{task};
use futures_util::{ StreamExt };

use coinbase_pro_one_rs::*;
use coinbase_pro_one_rs::book::OrderBook;
use async_std::pin::Pin;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    debug!("One: starting");
    task::block_on(async {
        let mut order_book = OrderBook::new();
        let (mut conduit, receiver) = client::Conduit::new(SANDBOX_URL, WS_SANDBOX_URL, None).await;
        conduit.level2().await;
        conduit.ticker(vec!("BTC-USD".to_string())).await;
        conduit.time().await;
        conduit.heartbeat().await;
        conduit.status().await;
        conduit.time().await;
        // Need to box this outside of the loop to avoid recurrent ownership issues.
        let mut harvested =
            order_book.harvest(Pin::new(Box::new(receiver))).await;
        loop {
            let msg = &mut harvested.next().await;
            debug!("One: receiver.next: {:?}", msg);
        }
    });
    Ok(())
}

