[![Build Status](https://travis-ci.org/inv2004/coinbase-pro-rs.svg?branch=master)](https://travis-ci.org/inv2004/coinbase-pro-rs)
[![Crates.io](https://img.shields.io/crates/v/coinbase-pro-rs.svg)](https://crates.io/crates/coinbase-pro-rs)
[![Docs.rs](https://docs.rs/coinbase-pro-rs/badge.svg)](https://docs.rs/coinbase-pro-rs)

# HEAVILY Derived (err... copied...) from:

https://github.com/inv2004/coinbase-pro-rs

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

## Coinbase Pro functionality supported:
- [x] Async
- [x] Websocket-Feed

## API
- [x] Requests
- [ ] Pagination
- [x] Types
- [x] Private
  - [x] Authentication
  - [x] Accounts
  - [ ] Orders
  - [ ] Fills
  - [ ] Deposits
  - [ ] Withdrawals
  - [ ] Payment Methods
  - [ ] Coinbase Accounts
  - [ ] Reports
  - [x] User Account
- [x] Market Data
  - [x] Products
  - [x] Currencies
  - [x] Time
- [x] Websocket Feed
  - [x] heartbeat
  - [x] ticker
  - [x] level2
  - [x] user
  - [x] matches
  - [x] full

# FIX API
See https://github.com/inv2004/coinbase-pro-rs for FIX requests.

# OrderBook
See  https://github.com/inv2004/orderbook-rs (for `coinbase-pro-rs`) and/or `/examples` (for `coinbase-pro-one-rs`).

