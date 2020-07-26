extern crate coinbase_pro_one_rs;

use async_std::println;
use async_std::{task};
use futures_util::{ StreamExt };

use coinbase_pro_one_rs::*;


fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    task::block_on(async {
        println!("One: starting").await;
        let (conduit, receiver) =
            client::Conduit::new(SANDBOX_URL, WS_SANDBOX_URL, None).await;
        println!("One: for_each").await;
        println!("One: heartbeat").await;
        conduit.clone().heartbeat().await;
        println!("One: time").await;
        conduit.clone().time().await;
        println!("One: time").await;
        conduit.clone().time().await;
        conduit.listen();
        receiver.for_each(|m| async move {
            println!("One message: {:?}", m).await;
        }).await;
        Ok(())
    })
}
