// -*- mode: rust; -*-

extern crate fancy_garbling;

// use criterion::{criterion_group, criterion_main, Criterion};
use fancy_garbling::{
    circuit::{Circuit, CircuitBuilder, CircuitRef},
    Modulus, photon::PhotonGadgets, Fancy, errors::CircuitBuilderError, classic::garble,
    Wire, 
};
use itertools::Itertools;
use ocelot::ot::{AlszSender as OtSender, Sender};
use scuttlebutt::{AesRng, Channel, AbstractChannel, Block};
use std::{
    io::{BufReader, BufWriter, Write},
    time::SystemTime, net::TcpStream, env, fs, path::Path,
};

const EV_ADDR: &str = "10.2.33.45:9481";
// const EV_ADDR: &str = "127.0.0.1:9481";

type Reader = BufReader<TcpStream>;
type Writer = BufWriter<TcpStream>;
type MyChannel = Channel<Reader, Writer>;

fn build_photon_circuit_gb<FPERM>(poly: &Modulus, mut perm: FPERM, d: usize, sruns: usize, pruns: usize) -> Circuit  where 
    FPERM: FnMut(&mut CircuitBuilder, &Vec<CircuitRef>) -> Result<Vec<CircuitRef>, CircuitBuilderError>, 
    {
    let start = SystemTime::now();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("./helper_test_files/output_TCPnonstr_log.txt")
        .unwrap();
    let mut b = CircuitBuilder::new();
    let xs = (0..pruns).map(|_| b.garbler_inputs(&vec![*poly; d*d])).collect_vec();
    for x in xs.into_iter() {
        let mut z = x;
        for _ in 0..sruns {
            z = perm(&mut b, &z).unwrap();
        }
        b.outputs(&z).unwrap();
    }
    let out = b.finish();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Building circuit: {} ms\nPer permutation: {} us",
        timing,
        ((timing*1000) as f64) / (pruns * sruns) as f64
    );
    write!(file, "Garbler :: Building circuit: {} ms\nPer permutation: {} us\n",
        timing,
        ((timing*1000) as f64) / (pruns * sruns) as f64
    ).unwrap();
    out
    
}

fn build_photon_circuit_ev<FPERM> (poly: &Modulus, mut perm: FPERM, d: usize, sruns: usize, pruns: usize) -> Circuit  where 
    FPERM: FnMut(&mut CircuitBuilder, &Vec<CircuitRef>) -> Result<Vec<CircuitRef>, CircuitBuilderError>, 
    {
    let start = SystemTime::now();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("./helper_test_files/output_TCPnonstr_log.txt")
        .unwrap();
    let mut b = CircuitBuilder::new();
    let xs = (0..pruns).map(|_| b.evaluator_inputs(&vec![*poly; d*d])).collect_vec();
    for x in xs.into_iter() {
        let mut z = x;
        for _ in 0..sruns {
            z = perm(&mut b, &z).unwrap();
        }
        b.outputs(&z).unwrap();
    }
    let out = b.finish();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Building circuit: {} ms\nPer permutation: {} us\n",
        timing,
        ((timing*1000) as f64) / (pruns * sruns) as f64
    );
    write!(file, "Garbler :: Building circuit: {} ms\nPer permutation: {} us\n",
        timing,
        ((timing*1000) as f64) / (pruns * sruns) as f64
    ).unwrap();
    out
}

fn run_circuit(circ: &Circuit, mut sender: TcpStream, gb_inputs: &[u16], n_ev_inputs: usize, modulus: &Modulus, p_runs: usize, s_runs: usize) -> Vec<u16> {
    // let n_gb_inputs = gb_inputs.len();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("./helper_test_files/output_TCPnonstr_log.txt")
        .unwrap();
    
    let mut rng = AesRng::new();
    let reader = BufReader::new(sender.try_clone().unwrap());
    let writer = BufWriter::new(sender.try_clone().unwrap());
    let mut channel = MyChannel::new(reader, writer);
    
    let start = SystemTime::now();
    let (en,gbc) = garble(&circ).unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Garbling circuit: {} ms\nPer permutation: {} us",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Garbling circuit: {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();

    let start = SystemTime::now();
    let gbc_ser = serde_json::to_string(&gbc).unwrap();
    println!("Size garbled circuit = {} bytes", gbc_ser.as_bytes().len());
    sender.write_all(&gbc_ser.as_bytes().len().to_le_bytes()).unwrap();
    sender.try_clone().unwrap().write_all(gbc_ser.as_bytes()).unwrap();
    sender.flush().unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Parsing & sending garbled circuit: {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Parsing & sending garbled circuit: {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();
    
    let mut ot = OtSender::init(&mut channel, &mut rng).unwrap();
    let start = SystemTime::now();
    let encoded_gb = en.encode_garbler_inputs(&gb_inputs);
    encoded_gb.iter().for_each(|wire| channel.write_block(&wire.as_block()).unwrap());

    let zero_ev = en.encode_evaluator_inputs(&vec![0; n_ev_inputs*p_runs]);

    let mut inputs = Vec::with_capacity(p_runs*n_ev_inputs*(modulus.size() as f32).log2() as usize);
    let mut wire: Wire; let mut delta: Wire;

    for run in 0..p_runs {
        for i in 0..n_ev_inputs {    
            wire = zero_ev[i + run*n_ev_inputs].clone();
            delta = en.encode_evaluator_input(1, i + run*n_ev_inputs).negate().plus(&zero_ev[i + run*n_ev_inputs]);
            let input = (0..(modulus.size() as f32).log2() as usize)
                .map(|i| {
                    let zero = if i > 0{
                        Wire::rand(&mut rng, modulus)
                    } else {wire.clone()};
                    let one = zero.plus(&delta);
                    wire = wire.plus(&zero.cmul(1 << i));   // see 7.1 in paper for binary representation labels
                    (zero.as_block(), one.as_block())
                }).rev()
                .collect::<Vec<(Block, Block)>>();
            for i in input.into_iter().rev() {
                inputs.push(i);
            }
        }
    }
    println!("send {}", inputs.len());
    ot.send(&mut channel, &inputs, &mut rng).unwrap();
    
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Encoding & sending inputs with OT: {} ms\nPer permutation: {} us",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Encoding & sending inputs with OT: {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();

    let out = (0..circ.noutputs()).map(|_| {
        channel.flush().unwrap();
        let val = channel.read_u16().unwrap();
        val
    }).collect_vec();
    out

}

fn main() {
    let args: Vec<String> = env::args().collect();
    let perm_id = &args[1];
    let gb_ev = &args[2];
    let s_runs: usize = args[3].parse().unwrap();
    let p_runs: usize = args[4].parse().unwrap();
    let modulus; let circ;
    let d; let input;
    let out;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create_new(!Path::new("./helper_test_files/output_TCPnonstr_log.txt").exists())
        .open("./helper_test_files/output_TCPnonstr_log.txt")
        .unwrap();


    write!(file, "--- GARBLER START: {} permutation(s) in series ---
                   {} permutation(s) in parallel
                   {} has all inputs
---           PHOTON{}                ---\n\n",
                s_runs, p_runs, gb_ev, perm_id).unwrap();
    match perm_id.as_ref() {
        "100" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 5;
            if gb_ev == "ev" {
                circ = build_photon_circuit_ev(&modulus, 
                        move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_100(f, &x), d, s_runs, p_runs);
            } else {
                circ = build_photon_circuit_gb(&modulus, 
                    move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_100(f, &x), d, s_runs, p_runs);
                }
            input =   vec![0, 0 ,0, 0, 4,
                            0, 0, 0, 0, 1,
                            0, 0 ,0, 0, 4,
                            0, 0 ,0, 0, 1,
                            0, 0 ,0, 1, 0];
        },
        "144" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 6;
            if gb_ev == "ev" {
                circ = build_photon_circuit_ev(&modulus, 
                        move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_144(f, &x), d, s_runs, p_runs);
            } else {
                circ = build_photon_circuit_gb(&modulus, 
                    move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_144(f, &x), d, s_runs, p_runs);
                }
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
            if gb_ev == "ev" {
                circ = build_photon_circuit_ev(&modulus, 
                        move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_196(f, &x), d, s_runs, p_runs);
            } else {
                circ = build_photon_circuit_gb(&modulus, 
                    move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_196(f, &x), d, s_runs, p_runs);
                }
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
            if gb_ev == "ev" {
                circ = build_photon_circuit_ev(&modulus, 
                        move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_256(f, &x), d, s_runs, p_runs);
            } else {
                circ = build_photon_circuit_gb(&modulus, 
                    move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_256(f, &x), d, s_runs, p_runs);
                }
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
            if gb_ev == "ev" {
                circ = build_photon_circuit_ev(&modulus, 
                        move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_288(f, &x), d, s_runs, p_runs);
            } else {
                circ = build_photon_circuit_gb(&modulus, 
                    move |f: &mut CircuitBuilder, x| PhotonGadgets::photon_288(f, &x), d, s_runs, p_runs);
                }
            input = vec![0, 0 ,0, 0, 0, 0,
                            0, 0, 0, 0, 0, 0,
                            0, 0 ,0, 0, 0, 0,
                            0, 0 ,0, 0, 0, 0x40,
                            0, 0 ,0, 0, 0, 0x20,
                            0, 0, 0, 0, 0, 0x20];
        },
        &_ => panic!("Command line argument is not a right permutation ID!")
    }
    match TcpStream::connect(EV_ADDR) {
        Ok(sender) => {
            let total = SystemTime::now();
            println!("Successfully connected to evaluator on {}", EV_ADDR);
            if gb_ev == "ev" {
                out = run_circuit(&circ, sender, &[], d*d, &modulus, p_runs,s_runs);
            } else {
                let mut gbs = vec![0; p_runs*input.len()];
                (0..p_runs*input.len()).for_each(|i| gbs[i] = input[i % input.len()]);
                out = run_circuit(&circ, sender, &gbs, 0, &modulus, p_runs, s_runs);
            }
            println!("output: {:?}", out);
            let tot = total.elapsed().unwrap().as_millis();
            println!("Total: {} ms", tot);
            println!("Average computing time / permutation: {} ms", (tot as f64)/((s_runs * p_runs) as f64));
            write!(file, "Garbler :: Total: {} ms\n 
                          Average computing time / permutation: {} ms\n
--------------------------------------\n\n", tot, (tot as f64)/((s_runs * p_runs) as f64)).unwrap();

        }
        Err(e) => println!("Failed to connect to evaluator: {}", e)
    }


}

