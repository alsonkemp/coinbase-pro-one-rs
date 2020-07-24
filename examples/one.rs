extern crate coinbase_pro_one_rs;

use async_std::{task};
use futures_util::{ StreamExt };

use coinbase_pro_one_rs::*;


fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    task::block_on(async {
        println!("Coinbase-pro-one-rs starting");
        let conduit = client::Conduit::new(SANDBOX_URL, WS_SANDBOX_URL, None).await;
        conduit.receiver.lock().await.for_each(|m| async move {
            println!("Message: {:?}", m);
        });
        conduit.time().await;
        Ok(())
    })
}
