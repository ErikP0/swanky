// -*- mode: rust; -*-
//
// This file is part of `twopac`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

use fancy_garbling::{
    circuit::Circuit,
    twopac::semihonest::{Evaluator, Garbler},
    FancyInput,
};
use fancy_garbling::Modulus;
use ocelot::ot::{AlszReceiver as OtReceiver, AlszSender as OtSender};
use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};
use std::time::SystemTime;

fn circuit(fname: &str) -> Circuit {
    println!("* Circuit: {}", fname);
    Circuit::parse(fname).unwrap()
}

fn run_circuit(circ: &mut Circuit, gb_inputs: Vec<u16>, ev_inputs: Vec<u16>) {
    let circ_ = circ.clone();
    let (sender, receiver) = unix_channel_pair();
    let n_gb_inputs = gb_inputs.len();
    let n_ev_inputs = ev_inputs.len();
    let total = SystemTime::now();
    let handle = std::thread::spawn(move || {
        let rng = AesRng::new();
        let start = SystemTime::now();
        let mut gb = Garbler::<UnixChannel, AesRng, OtSender>::new(sender, rng).unwrap();
        println!(
            "Garbler :: Initialization: {} ms",
            start.elapsed().unwrap().as_millis()
        );
        let start = SystemTime::now();
        let xs = gb.encode_many(&gb_inputs, &vec![Modulus::Zq { q: (2) }; n_gb_inputs]).unwrap();    // encoded garbler inputs - only W^0
        let ys = gb.receive_many(&vec![Modulus::Zq { q: (2) }; n_ev_inputs]).unwrap();               // encoded evaluator inputs - only W^0
        println!(
            "Garbler :: Encoding inputs: {} ms",
            start.elapsed().unwrap().as_millis()
        );
        let start = SystemTime::now();
        circ_.eval(&mut gb, &xs, &ys).unwrap();
        println!(
            "Garbler :: Circuit garbling: {} ms",
            start.elapsed().unwrap().as_millis()
        );
    });
    let rng = AesRng::new();
    let start = SystemTime::now();
    let mut ev = Evaluator::<UnixChannel, AesRng, OtReceiver>::new(receiver, rng).unwrap();
    println!(
        "Evaluator :: Initialization: {} ms",
        start.elapsed().unwrap().as_millis()
    );
    let start = SystemTime::now();
    let xs = ev.receive_many(&vec![Modulus::Zq { q: (2) }; n_gb_inputs]).unwrap();               // receive inputs in same order! moduli array ev == moduli array gb
    let ys = ev.encode_many(&ev_inputs, &vec![Modulus::Zq { q: (2) }; n_ev_inputs]).unwrap();
    println!(
        "Evaluator :: Encoding inputs: {} ms",
        start.elapsed().unwrap().as_millis()
    );
    let start = SystemTime::now();
    circ.eval(&mut ev, &xs, &ys).unwrap();
    println!(
        "Evaluator :: Circuit evaluation: {} ms",
        start.elapsed().unwrap().as_millis()
    );
    handle.join().unwrap();
    println!("Total: {} ms", total.elapsed().unwrap().as_millis());
}

fn main() {
    let mut circ = circuit("circuits/AES-non-expanded.txt");
    run_circuit(&mut circ, vec![0; 128], vec![0; 128]);
    let mut circ = circuit("circuits/sha-1.txt");
    run_circuit(&mut circ, vec![0; 512], vec![]);
    let mut circ = circuit("circuits/sha-256.txt");
    run_circuit(&mut circ, vec![0; 512], vec![]);
}
