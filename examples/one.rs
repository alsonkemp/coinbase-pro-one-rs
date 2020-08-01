extern crate coinbase_pro_one_rs;

use std::time::Duration;

use async_std::{task};
use futures_util::{ StreamExt };

use coinbase_pro_one_rs::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dbg!("One: starting");
    task::block_on(async {
        let (mut conduit, mut receiver) = client::Conduit::new(SANDBOX_URL, WS_SANDBOX_URL, None).await;
        conduit.time().await;
        conduit.heartbeat().await;
        conduit.status().await;
        loop {
            let resp = receiver.next().await;
            dbg!("One: next: {:?}", resp);
        }
    });
    Ok(())
}

