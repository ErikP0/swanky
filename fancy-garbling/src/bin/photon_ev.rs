// -*- mode: rust; -*-

extern crate fancy_garbling;

// use criterion::{criterion_group, criterion_main, Criterion};
use fancy_garbling::{
    circuit::{Circuit, CircuitBuilder, CircuitRef},
    twopac::semihonest::{Evaluator, Garbler},
    FancyInput, Modulus, photon::PhotonGadgets, Fancy, errors::CircuitBuilderError,
};
use ocelot::ot::{AlszReceiver as OtReceiver, AlszSender as OtSender};
use scuttlebutt::{AesRng, Channel, SyncChannel};
use std::{
    io::{BufReader, BufWriter, Read, Write},
    os::unix::net::UnixStream,
    net::{TcpStream, TcpListener, Shutdown},
    time::Duration, env,
};

const EV_ADDR: &str = "127.0.0.1:9481";

type Reader = BufReader<TcpStream>;
type Writer = BufWriter<TcpStream>;
type MyChannel = Channel<Reader, Writer>;

fn build_photon_circuit_gb<FPERM>(poly: &Modulus, mut perm: FPERM, d: usize) -> Circuit  where 
    FPERM: FnMut(&mut CircuitBuilder, &Vec<CircuitRef>) -> Result<Vec<CircuitRef>, CircuitBuilderError>, 
    {
    let mut b = CircuitBuilder::new();
    let x = b.garbler_inputs(&vec![*poly; d*d]); 
    // let z = b.photon_100(&x).unwrap();
    let z = perm(&mut b, &x).unwrap();
    b.outputs(&z).unwrap();
    b.finish()
}

fn build_photon_circuit_ev<FPERM> (poly: &Modulus, mut perm: FPERM, d: usize) -> Circuit  where 
    FPERM: FnMut(&mut CircuitBuilder, &Vec<CircuitRef>) -> Result<Vec<CircuitRef>, CircuitBuilderError>, 
    {
    let mut b = CircuitBuilder::new();
    let x = b.evaluator_inputs(&vec![*poly; d*d]); 
    let z = perm(&mut b, &x).unwrap();
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
    let args: Vec<String> = env::args().collect();
    let perm_id = &args[1];
    let modulus; let circ;
    let d; let input;

    match perm_id.as_ref() {
        "100" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 5;
            circ = build_photon_circuit_gb(&modulus, 
                    move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_100(f, &x), d);
            input =   vec![0, 0 ,0, 0, 4,
                            0, 0, 0, 0, 1,
                            0, 0 ,0, 0, 4,
                            0, 0 ,0, 0, 1,
                            0, 0 ,0, 1, 0];
        },
        "144" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 6;
            circ = build_photon_circuit_gb(&modulus, 
                move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_144(f, &x), d);
            input = vec![0, 0 ,0, 0, 0, 2,
                          0, 0, 0, 0, 0, 0,
                          0, 0 ,0, 0, 0, 1,
                          0, 0 ,0, 0, 0, 0,
                          0, 0 ,0, 0, 0, 1,
                          0, 0, 0, 0, 0, 0];
        },
        "196" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 7;
            circ = build_photon_circuit_gb(&modulus, 
                move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_196(f, &x), d);
            input = vec![0, 0 ,0, 0, 0, 0, 0,
                          0, 0, 0, 0, 0, 0, 2,
                          0, 0 ,0, 0, 0, 0, 8,
                          0, 0 ,0, 0, 0, 0, 2,
                          0, 0 ,0, 0, 0, 0, 4,
                          0, 0, 0, 0, 0, 0, 2,
                          0, 0, 0, 0, 0, 0, 4];
        },
        "256" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 8;
            circ = build_photon_circuit_gb(&modulus, 
                move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_256(f, &x), d);
            input = vec![0, 0 ,0, 0, 0, 0, 0, 0,
                            0, 0, 0, 0, 0, 0, 0, 0,
                            0, 0 ,0, 0, 0, 0, 0, 3,
                            0, 0 ,0, 0, 0, 0, 0, 8,
                            0, 0 ,0, 0, 0, 0, 0, 2,
                            0, 0, 0, 0, 0, 0, 0, 0,
                            0, 0, 0, 0, 0, 0, 0, 2,
                            0, 0, 0, 0, 0, 0, 0, 0];
        },
        "288" => {
            modulus = Modulus::GF8 { p: 283 };
            d = 6;
            circ = build_photon_circuit_gb(&modulus, 
                move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_288(f, &x), d);
            input = vec![0, 0 ,0, 0, 0, 0,
                            0, 0, 0, 0, 0, 0,
                            0, 0 ,0, 0, 0, 0,
                            0, 0 ,0, 0, 0, 0x40,
                            0, 0 ,0, 0, 0, 0x20,
                            0, 0, 0, 0, 0, 0x20];
        },
        &_ => panic!("Command line argument is not a right permutation ID!")
    }

    let listener = TcpListener::bind(EV_ADDR).unwrap();
    println!("Evaluator listening on {}", EV_ADDR);

    loop {
        match listener.accept() {
            Ok((receiver, addr)) => {
                println!("Garbler connected on {}", addr);

                // let output = run_circuit(&circ, receiver, &[], d*d, &modulus);
                let output = run_circuit(&circ, receiver, &[], 25, &modulus);
                println!("done: {:?}", output);
            }
            Err(e) => println!("Connection failed: {}", e),
        }
    }


}