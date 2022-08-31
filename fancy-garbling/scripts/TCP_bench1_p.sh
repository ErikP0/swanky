#!/bin/bash

for P_RUNS in {10..500..10}
do
    cargo run --bin photon_gb_nonstr --features="serde1" --release 100 gb 1 $P_RUNS
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 100 gb 1 $P_RUNS

    cargo run --bin photon_gb_nonstr --features="serde1" --release 144 gb 1 $P_RUNS
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 144 gb 1 $P_RUNS

    cargo run --bin photon_gb_nonstr --features="serde1" --release 196 gb 1 $P_RUNS
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 196 gb 1 $P_RUNS

    cargo run --bin photon_gb_nonstr --features="serde1" --release 256 gb 1 $P_RUNS
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 256 gb 1 $P_RUNS

    cargo run --bin photon_gb_nonstr --features="serde1" --release 288 gb 1 $P_RUNS
    cargo run --bin photonbin_gb_nonstr --features="serde1" --release 288 gb 1 $P_RUNS
done