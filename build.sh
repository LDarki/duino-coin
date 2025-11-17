#!/bin/bash
cargo +nightly build --release -p masterserver-ebpf --target bpfel-unknown-none -Z build-std=core
mv target/bpfel-unknown-none/release/ebpf-filter masterserver/src/network/
cargo build --release -p masterserver