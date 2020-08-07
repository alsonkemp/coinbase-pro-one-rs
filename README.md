# HEAVILY Derived (err... copied...) from:
https://github.com/inv2004/coinbase-pro-rs

# Objectives
See `Objectives [more]` below...

This repo/project has the following objective (which differs from those of the original):
* One mechanism to rule them all.
  * No (significant) exposure of REST sync versus async or websocket functionality.
  * All functionality is exposed in one async mechanism.
    * REST or websockets are used as appropriate.
  * All responses have a consistent return (whether delivered in response to a RESTful HTTP request or to a Websocket subscription).

# Coinbase Pro One client for Rust
* Supports async data only
* Public/private feeds are supported

# Usage
Cargo.toml:
```toml
[dependencies]
coinbase-pro-one-rs = "0.0.1"
```

# OrderBook
See `/src/book/mod.rs`.

The original author's @  https://github.com/inv2004/orderbook-rs (for `coinbase-pro-rs`).

# Examples

See `/examples`.

Run against the sandbox: `cargo run --example one` or `./run_one.sh`.

# Example
## Code
Note: `time()` is HTTPS; `heartbeat()` and `status()` are Websocket.

`/examples/one.rs`:

```
extern crate coinbase_pro_one_rs;

use std::time::Duration;

use async_std::{task};
use futures_util::{ StreamExt };

use coinbase_pro_one_rs::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dbg!("One: starting");
    task::block_on(async {
        let (mut conduit, mut receiver) = client::Conduit::new(SANDBOX_URL, WS_SANDBOX_URL, None).await;
        conduit.time().await;      // HTTP(S) REST call
        conduit.heartbeat().await; // Websocket call
        conduit.status().await;    // Websocket call
        loop {
            if receiver.is_empty() {
                task::sleep(Duration::from_millis(10)).await;
            } else {
                let resp = receiver.next().await;
                dbg!("One: next: {:?}", resp); // Print the HTTPS/WS response (Message)
            }
        }
    });
    Ok(())
}
```

## Output
(The logging output is constantly changing (yay, debug `printf`s) so it might not look exactly like this...)

```
[examples/one.rs:11] "One: starting" = "One: starting"
[src/client.rs:52] "Conduit.new: {:?} {:?} {:?}" = "Conduit.new: {:?} {:?} {:?}"
[src/client.rs:52] http_uri = "https://api-public.sandbox.pro.coinbase.com"
[src/client.rs:52] ws_uri = "wss://ws-feed-public.sandbox.pro.coinbase.com"
[src/client.rs:52] _creds = None
[src/client.rs:183] "Conduit: time sent..." = "Conduit: time sent..."
[src/client.rs:166] "Conduit: heartbeat..." = "Conduit: heartbeat..."
[src/client.rs:785] "Conduit: **not** calculating auth... " = "Conduit: **not** calculating auth... "
[src/client.rs:115] "Conduit: subscription: sending {:?}" = "Conduit: subscription: sending {:?}"
[src/client.rs:115] serde_json::to_string(&msg).unwrap() = "{\"type\":\"Subscribe\",\"type\":\"subscribe\",\"channels\":[{\"name\":\"heartbeat\",\"product_ids\":[\"BTC-USD\"]}]}"
[src/client.rs:177] "Conduit: status..." = "Conduit: status..."
[src/client.rs:785] "Conduit: **not** calculating auth... " = "Conduit: **not** calculating auth... "
[src/client.rs:115] "Conduit: subscription: sending {:?}" = "Conduit: subscription: sending {:?}"
[examples/one.rs:22] "One: next: {:?}" = "One: next: {:?}"
[examples/one.rs:22] resp = Some(
    Time(
        Time {
            iso: "2020-08-01T15:32:55.618Z",
            epoch: 1596295975.618,
        },
    ),
)
[src/client.rs:799] &msg = "{\"type\":\"subscribe\",\"channels\":[{\"name\":\"heartbeat\",\"product_ids\":[\"BTC-USD\"]}]}"
[src/client.rs:799] &msg = "{\"type\":\"Subscribe\",\"type\":\"subscribe\",\"channels\":[\"status\"]}"
[src/client.rs:808] "Conduit. _websocket_handler._handle_incoming: received {:?}" = "Conduit. _websocket_handler._handle_incoming: received {:?}"
[src/client.rs:808] &tmsg = Ok(
    Some(
        Text(
            "{\"type\":\"subscriptions\",\"channels\":[{\"name\":\"heartbeat\",\"product_ids\":[\"BTC-USD\"]}]}",
        ),
    ),
)
[src/client.rs:808] &tmsg = Ok(
    Some(
        Text(
            "{\"type\":\"subscriptions\",\"channels\":[{\"name\":\"heartbeat\",\"product_ids\":[\"BTC-USD\"]},{\"name\":\"status\",\"product_ids\":[]}]}",
        ),
    ),
)
[src/client.rs:808] &tmsg = Ok(
    Some(
        Text(
            "{\"type\":\"status\",\"currencies\":[{\"id\":\"BAT\",\"name\":\"Basic Attention Token\",\"min_size\":\"1.00000000\",\"status\":\"online\",\"funding_account_id\":\"bd60dff8-14c5-45e1-a000-766db4a01a2c\",\"status_message\":\"\",\"max_precision\":\"1.0000000000000000000000000000000000000000\",\"convertible_to\":[],\"details\":{\"type\":\"crypto\",\"symbol\":\"\",\"network_confirmations\":35,\"sort_order\":10,\"crypto_address_link\":\"https://etherscan.io/token/0x0d8775f648430679a709e98d2b0cb6250d2887ef?a={{address}}\",\"crypto_transaction_link\":\"https://etherscan.io/tx/0x{{txId}}\",\"push_payment_methods\":[\"crypto\"]}},{\"id\":\"LINK\",\"name\":\"Chainlink\",\"min_size\":\"1.00000000\",\"status\":\"online\",\"funding_account_id\":\"d74a25fa-bcf2-43b6-9528-6b30243d2baf\",\"status_message\":\"\",\"max_precision\":\"0.0000000100000000000000000000000000000000\",\"convertible_to\":[],\"details\":{\"type\":\"crypto\",\"symbol\":\"Ξ\",\"network_confirmations\":35,\"sort_order\":67,\"crypto_address_link\":\"https://etherscan.io/token/0x514910771af9ca656af840dff83e8264ecf986ca?a={{address}}\",\"crypto_transaction_link\":\"https://etherscan.io/tx/0x{{txId}}\",\"push_payment_methods\":[\"crypto\"]}},{\"id\":\"USD\",\"name\":\"United States Dollar\",\"min_size\":\"0.01000000\",\"status\":\"online\",\"funding_account_id\":\"a5fd5069-cd04-490d-a7c5-750efabc1b1e\",\"status_message\":\"\",\"max_precision\":\"0.0100000000000000000000000000000000000000\",\"convertible_to\":[\"USDC\"],\"details\":{\"type\":\"fiat\",\"symbol\":\"$\",\"network_confirmations\":0,\"sort_order\":0,\"crypto_address_link\":\"\",\"crypto_transaction_link\":\"\",\"push_payment_methods\":[\"bank_wire\",\"swift_bank_account\",\"intra_bank_account\"],\"group_types\":[\"fiat\",\"usd\"],\"display_name\":\"US Dollar\"}},{\"id\":\"BTC\",\"name\":\"Bitcoin\",\"min_size\":\"0.00000001\",\"status\":\"online\",\"funding_account_id\":\"e0c38e21-b711-4d5d-b046-c686259167ae\",\"status_message\":\"\",\"max_precision\":\"0.0000000100000000000000000000000000000000\",\"convertible_to\":[],\"details\":{\"type\":\"crypto\",\"symbol\":\"\",\"network_confirmations\":6,\"sort_order\":3,\"crypto_address_link\":\"https://live.blockcypher.com/btc/address/{{address}}\",\"crypto_transaction_link\":\"https://live.blockcypher.com/btc/tx/{{txId}}\",\"push_payment_methods\":[\"crypto\"],\"group_types\":[\"btc\",\"crypto\"]}},{\"id\":\"GBP\",\"name\":\"British Pound\",\"min_size\":\"0.01000000\",\"status\":\"online\",\"funding_account_id\":\"e845fc0c-4cd5-4d9f-8f43-06418bd5dd76\",\"status_message\":\"\",\"max_precision\":\"0.0100000000000000000000000000000000000000\",\"convertible_to\":[],\"details\":{\"type\":\"fiat\",\"symbol\":\"£\",\"network_confirmations\":0,\"sort_order\":2,\"crypto_address_link\":\"\",\"crypto_transaction_link\":\"\",\"push_payment_methods\":[\"uk_bank_account\",\"swift_lhv\"],\"group_types\":[\"fiat\",\"gbp\"]}},{\"id\":\"EUR\",\"name\":\"Euro\",\"min_size\":\"0.01000000\",\"status\":\"online\",\"funding_account_id\":\"07525075-9845-4e99-b68c-defec9334fd4\",\"status_message\":\"\",\"max_precision\":\"0.0100000000000000000000000000000000000000\",\"convertible_to\":[],\"details\":{\"type\":\"fiat\",\"symbol\":\"€\",\"network_confirmations\":0,\"sort_order\":1,\"crypto_address_link\":\"\",\"crypto_transaction_link\":\"\",\"push_payment_methods\":[\"sepa_bank_account\"],\"group_types\":[\"fiat\",\"eur\"]}},{\"id\":\"ETH\",\"name\":\"Ether\",\"min_size\":\"0.00000001\",\"status\":\"online\",\"funding_account_id\":\"bdd1f924-bb79-493f-bffd-2633ce013a89\",\"status_message\":\"\",\"max_precision\":\"0.0000000100000000000000000000000000000000\",\"convertible_to\":[],\"details\":{\"type\":\"crypto\",\"symbol\":\"\",\"network_confirmations\":35,\"sort_order\":7,\"crypto_address_link\":\"https://etherscan.io/address/{{address}}\",\"crypto_transaction_link\":\"https://etherscan.io/tx/0x{{txId}}\",\"push_payment_methods\":[\"crypto\"],\"group_types\":[\"eth\",\"crypto\"]}},{\"id\":\"USDC\",\"name\":\"USD Coin\",\"min_size\":\"0.00000100\",\"status\":\"online\",\"funding_account_id\":\"224b8e2d-45d3-4376-ad89-28eae8ecb4bb\",\"status_message\":\"\",\"max_precision\":\"0.0000010000000000000000000000000000000000\",\"convertible_to\":[\"USD\"],\"details\":{\"type\":\"crypto\",\"symbol\":\"$\",\"network_confirmations\":35,\"sort_order\":9,\"crypto_address_link\":\"https://etherscan.io/token/0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48?a={{address}}\",\"crypto_transaction_link\":\"https://etherscan.io/tx/0x{{txId}}\",\"push_payment_methods\":[\"crypto\"],\"group_types\":[\"stablecoin\",\"usdc\",\"crypto\"]}}],\"products\":[{\"id\":\"LINK-USDC\",\"base_currency\":\"LINK\",\"quote_currency\":\"USDC\",\"base_min_size\":\"1\",\"base_max_size\":\"800000\",\"base_increment\":\"1\",\"quote_increment\":\"0.000001\",\"display_name\":\"LINK/USDC\",\"status\":\"online\",\"margin_enabled\":false,\"status_message\":\"\",\"min_market_funds\":\"10\",\"max_market_funds\":\"100000\",\"post_only\":false,\"limit_only\":false,\"cancel_only\":false,\"type\":\"spot\"},{\"id\":\"BTC-EUR\",\"base_currency\":\"BTC\",\"quote_currency\":\"EUR\",\"base_min_size\":\"0.001\",\"base_max_size\":\"10000\",\"base_increment\":\"0.00000001\",\"quote_increment\":\"0.01\",\"display_name\":\"BTC/EUR\",\"status\":\"online\",\"margin_enabled\":false,\"status_message\":\"\",\"min_market_funds\":\"10\",\"max_market_funds\":\"600000\",\"post_only\":false,\"limit_only\":false,\"cancel_only\":false,\"type\":\"spot\"},{\"id\":\"BAT-USDC\",\"base_currency\":\"BAT\",\"quote_currency\":\"USDC\",\"base_min_size\":\"1\",\"base_max_size\":\"300000\",\"base_increment\":\"1\",\"quote_increment\":\"0.000001\",\"display_name\":\"BAT/USDC\",\"status\":\"online\",\"margin_enabled\":false,\"status_message\":\"\",\"min_market_funds\":\"1\",\"max_market_funds\":\"100000\",\"post_only\":false,\"limit_only\":false,\"cancel_only\":false,\"type\":\"spot\"},{\"id\":\"ETH-BTC\",\"base_currency\":\"ETH\",\"quote_currency\":\"BTC\",\"base_min_size\":\"0.01\",\"base_max_size\":\"1000000\",\"base_increment\":\"0.00000001\",\"quote_increment\":\"0.00001\",\"display_name\":\"ETH/BTC\",\"status\":\"online\",\"margin_enabled\":false,\"status_message\":\"\",\"min_market_funds\":\"0.001\",\"max_market_funds\":\"80\",\"post_only\":false,\"limit_only\":false,\"cancel_only\":false,\"type\":\"spot\"},{\"id\":\"BTC-USD\",\"base_currency\":\"BTC\",\"quote_currency\":\"USD\",\"base_min_size\":\"0.001\",\"base_max_size\":\"10000\",\"base_increment\":\"0.00000001\",\"quote_increment\":\"0.01\",\"display_name\":\"BTC/USD\",\"status\":\"online\",\"margin_enabled\":true,\"status_message\":\"\",\"min_market_funds\":\"10\",\"max_market_funds\":\"1000000\",\"post_only\":false,\"limit_only\":false,\"cancel_only\":false,\"type\":\"spot\"},{\"id\":\"BTC-GBP\",\"base_currency\":\"BTC\",\"quote_currency\":\"GBP\",\"base_min_size\":\"0.001\",\"base_max_size\":\"10000\",\"base_increment\":\"0.00000001\",\"quote_increment\":\"0.01\",\"display_name\":\"BTC/GBP\",\"status\":\"online\",\"margin_enabled\":false,\"status_message\":\"\",\"min_market_funds\":\"10\",\"max_market_funds\":\"200000\",\"post_only\":false,\"limit_only\":false,\"cancel_only\":false,\"type\":\"spot\"}]}",
        ),
    ),
)
[src/client.rs:808] &tmsg = Ok(
    Some(
        Text(
            "{\"type\":\"heartbeat\",\"last_trade_id\":0,\"product_id\":\"BTC-USD\",\"sequence\":187217268,\"time\":\"2020-08-01T15:32:56.183755Z\"}",
        ),
    ),
)
[src/client.rs:808] &tmsg = Ok(
    Some(
        Text(
            "{\"type\":\"heartbeat\",\"last_trade_id\":0,\"product_id\":\"BTC-USD\",\"sequence\":187217268,\"time\":\"2020-08-01T15:32:57.183778Z\"}",
        ),
    ),
)
```
# FIX API
See the original author @ https://github.com/inv2004/coinbase-pro-rs for FIX requests.

# Thanks

JetBrains for a complimentary Open Source License for:
![CLion](/extra/logo_CLion.png)
