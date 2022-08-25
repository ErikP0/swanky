// -*- mode: rust; -*-

extern crate fancy_garbling;

// use criterion::{criterion_group, criterion_main, Criterion};
use fancy_garbling::{
    circuit::{Circuit, CircuitBuilder},
    twopac::semihonest::{Evaluator, Garbler},
    FancyInput, Modulus, photon::PhotonGadgets, Fancy,
};
use ocelot::ot::{AlszReceiver as OtReceiver, AlszSender as OtSender};
use scuttlebutt::{AesRng, Channel, SyncChannel};
use std::{
    io::{BufReader, BufWriter, Read, Write},
    os::unix::net::UnixStream,
    net::{TcpStream, TcpListener, Shutdown},
    time::Duration,
};

const EV_ADDR: &str = "127.0.0.1:9481";

type Reader = BufReader<TcpStream>;
type Writer = BufWriter<TcpStream>;
type MyChannel = Channel<Reader, Writer>;

fn build_photon_circuit_gb(poly: &Modulus) -> Circuit {
    let mut b = CircuitBuilder::new();
    let x = b.garbler_inputs(&vec![*poly; 25]); 
    let z = b.photon_100(&x).unwrap();
    b.outputs(&z).unwrap();
    b.finish()
}

fn build_photon_circuit_ev(poly: &Modulus) -> Circuit {
    let mut b = CircuitBuilder::new();
    let x = b.evaluator_inputs(&vec![*poly; 25]); 
    let z = b.photon_100(&x).unwrap();
    b.outputs(&z).unwrap();
    b.finish()
}

fn run_circuit(circ: &Circuit, receiver: TcpStream, ev_inputs: &[u16], n_gb_inputs: usize, modulus: &Modulus) 
                -> Vec<u16> {
    let n_ev_inputs = ev_inputs.len();

    let rng = AesRng::new();
    let reader = BufReader::new(receiver.try_clone().unwrap());
    let writer = BufWriter::new(receiver);
    let channel = Channel::new(reader, writer);
    let mut ev = Evaluator::<MyChannel, AesRng, OtReceiver>::new(channel, rng).unwrap();
    let xs = ev.receive_many(&vec![*modulus; n_gb_inputs]).unwrap();
    let ys = ev.encode_many(&ev_inputs, &vec![*modulus; n_ev_inputs]).unwrap();
    let output = circ.eval(&mut ev, &xs, &ys).unwrap();
    output.unwrap()
}


fn main() {
    let input =   &[0, 0 ,0, 0, 4,
                                0, 0, 0, 0, 1,
                                0, 0 ,0, 0, 4,
                                0, 0 ,0, 0, 1,
                                0, 0 ,0, 1, 0];
    let x4_x_1 = Modulus::GF4 { p: 19 };

    let listener = TcpListener::bind(EV_ADDR).unwrap();
    println!("Evaluator listening on {}", EV_ADDR);

    loop {
        match listener.accept() {
            Ok((mut receiver, addr)) => {
                println!("Garbler connected on {}", addr);

                let circ = build_photon_circuit_ev(&x4_x_1);
                // let output = run_circuit(&circ, receiver, &[], 25, &x4_x_1);
                let output = run_circuit(&circ, receiver, input, 0, &x4_x_1);
                println!("done: {:?}", output);
            }
            Err(e) => println!("Connection failed: {}", e),
        }
    }


}
