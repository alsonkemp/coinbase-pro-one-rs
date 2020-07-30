extern crate coinbase_pro_one_rs;

use async_std::{task};
use futures;
use futures_util::{ StreamExt };
use std::fmt;
use std::time::Duration;

use coinbase_pro_one_rs::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dbg!("One: starting");
    task::block_on(async {
        let (mut conduit, mut receiver) = client::Conduit::new(SANDBOX_URL, WS_SANDBOX_URL, None).await;
        conduit.heartbeat().await;
        conduit.time().await;
        loop {
            if receiver.is_empty() {
                task::sleep(Duration::from_millis(10)).await;
            } else {
                let resp = receiver.next().await;
                dbg!("One: next: {:?}", resp);
            }
        }
    });
    Ok(())
}

