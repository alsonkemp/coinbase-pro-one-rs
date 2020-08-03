#!/bin/bash

reset
RUST_LOG=error,one=debug,coinbase_pro_one_rs=debug cargo run --example one --release
