extern crate coinbase_pro_rs;
extern crate tokio;

mod common;

use coinbase_pro_rs::*;
use common::delay;
use tokio::prelude::Future;

#[test]
fn test_sync() {
    delay();
    let client: Public<Sync> = Public::new(SANDBOX_URL);
    let time = client.get_time().unwrap();
    let time_str = format!("{:?}", time);
    assert!(time_str.starts_with("Time {"));
    assert!(time_str.contains("iso:"));
    assert!(time_str.contains("epoch:"));
    assert!(time_str.ends_with("}"));
}

#[test]
fn test_async() {
    delay();
    let client: Public<ASync> = Public::new(SANDBOX_URL);
    let time = client.get_time().and_then(|time| {
        let time_str = format!("{:?}", time);
        assert!(time_str.starts_with("Time {"));
        assert!(time_str.contains("iso:"));
        assert!(time_str.contains("epoch:"));
        assert!(time_str.ends_with("}"));
        Ok(())
    });
    let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
    rt.block_on(time).ok();
}
