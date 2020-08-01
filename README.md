NOTE: needs a bit commit from my other computer...

# HEAVILY Derived (err... copied...) from:

https://github.com/inv2004/coinbase-pro-rs

# Objectives

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

# Examples

See `/examples`.

Run against the sandbox: `cargo run --example one`

# FIX API
See the original author @ https://github.com/inv2004/coinbase-pro-rs for FIX requests.

# OrderBook
See the original author's @  https://github.com/inv2004/orderbook-rs (for `coinbase-pro-rs`) and/or `/examples` (for `coinbase-pro-one-rs`).

