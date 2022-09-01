#!/bin/bash

cargo run --bin photon_gb_nonstr --features="serde1" --release 100 gb 1 1
cargo run --bin photonbin_gb_nonstr --features="serde1" --release 100 gb 1 1

for S_RUNS in {10..500..10}
do
    cargo run --bin photon_gb_nonstr --features="serde1" --release 100 gb $S_RUNS 1
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 100 gb $S_RUNS 1

    cargo run --bin photon_gb_nonstr --features="serde1" --release 144 gb $S_RUNS 1
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 144 gb $S_RUNS 1

    cargo run --bin photon_gb_nonstr --features="serde1" --release 196 gb $S_RUNS 1
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 196 gb $S_RUNS 1

    cargo run --bin photon_gb_nonstr --features="serde1" --release 256 gb $S_RUNS 1
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 256 gb $S_RUNS 1

    cargo run --bin photon_gb_nonstr --features="serde1" --release 288 gb $S_RUNS 1
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 288 gb $S_RUNS 1
done