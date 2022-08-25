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
    io::{BufReader, BufWriter, Write, Read},
    time::Duration, net::{TcpListener, TcpStream}, str::from_utf8,
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

fn run_circuit(circ: &Circuit, sender: TcpStream, gb_inputs: &[u16], n_ev_inputs: usize, modulus: &Modulus) -> Vec<u16>{
    let n_gb_inputs = gb_inputs.len();

    let rng = AesRng::new();
    let reader = BufReader::new(sender.try_clone().unwrap());
    let writer = BufWriter::new(sender);
    let channel = Channel::new(reader, writer);
    let mut gb = Garbler::<MyChannel, AesRng, OtSender>::new(channel, rng).unwrap();
    let xs = gb.encode_many(&gb_inputs, &vec![*modulus; n_gb_inputs]).unwrap();
    let ys = gb.receive_many(&vec![*modulus; n_ev_inputs]).unwrap();
    circ.eval(&mut gb, &xs, &ys).unwrap().unwrap()
    
}

fn main() {
    let x4_x_1 = Modulus::GF4 { p: 19 };
    let input =   &[0, 0 ,0, 0, 4,
                                0, 0, 0, 0, 1,
                                0, 0 ,0, 0, 4,
                                0, 0 ,0, 0, 1,
                                0, 0 ,0, 1, 0];

    match TcpStream::connect(EV_ADDR) {
        Ok(sender) => {
            println!("Successfully connected to evaluator on {}", EV_ADDR);

            let circ = build_photon_circuit_ev(&x4_x_1);
            // run_circuit(&circ, sender, input, 0, &x4_x_1);
            let out = run_circuit(&circ, sender, &[], 25, &x4_x_1);
            println!("gb out: {:?}", out);

        }
        Err(e) => println!("Failed to connect to evaluator: {}", e)
    }


}

