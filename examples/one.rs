extern crate coinbase_pro_one_rs;
extern crate tokio_tungstenite;

use futures_util::{ StreamExt };
use coinbase_pro_one_rs::*;
use coinbase_pro_one_rs::{SANDBOX_URL, WS_SANDBOX_URL};
use std::borrow::Borrow;


async fn main() {
    println!("Coinbase-pro-one-rs starting");
    let conduit = client::Conduit::new(SANDBOX_URL, WS_SANDBOX_URL, None).await;
    conduit.receiver.for_each( |m| async move {
       println!("Message: {:?}", m);
    });
    conduit.time().await;
}
