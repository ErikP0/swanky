// -*- mode: rust; -*-

extern crate fancy_garbling;

// use criterion::{criterion_group, criterion_main, Criterion};
use fancy_garbling::{
    circuit::{Circuit, CircuitBuilder, CircuitRef},
    twopac::semihonest::Evaluator,
    FancyInput, Modulus, photon::PhotonGadgets, Fancy, errors::CircuitBuilderError, classic::GarbledCircuit,
};
use itertools::Itertools;
use ocelot::ot::AlszReceiver as OtReceiver;
use scuttlebutt::{AesRng, Channel, AbstractChannel};
// use core::slice::SlicePattern;
use std::{
    io::{BufReader, BufWriter, Write, Read},
    net::{TcpStream, TcpListener},
    time::SystemTime, env, fs, path::Path,
};

const EV_ADDR: &str = "0.0.0.0:9481";

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
        "Evaluator :: Building circuit: {} ms\nPer permutation: {} us",
        timing,
        ((timing * 1000) as f64) / (pruns * sruns) as f64
    );
    write!(file, "Evaluator :: Building circuit: {} ms\nPer permutation: {} us\n",
        timing,
        ((timing * 1000) as f64) / (pruns * sruns) as f64
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
        "Evaluator :: Building circuit: {} ms\nPer permutation: {} us",
        timing,
        ((timing * 1000) as f64) / (pruns * sruns) as f64
    );
    write!(file, "Evaluator :: Building circuit: {} ms\nPer permutation: {} us\n",
        timing,
        ((timing * 1000) as f64) / (pruns * sruns) as f64
    ).unwrap();
    out
}

fn run_circuit(circ: &Circuit, mut receiver: TcpStream, ev_inputs: &[u16], n_gb_inputs: usize, modulus: &Modulus, p_runs: usize, s_runs: usize) 
                -> Vec<u16> {
    let n_ev_inputs = ev_inputs.len();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("./helper_test_files/output_TCPnonstr_log.txt")
        .unwrap();

    let rng = AesRng::new();
    let reader = BufReader::new(receiver.try_clone().unwrap());
    let writer = BufWriter::new(receiver.try_clone().unwrap());
    let channel = Channel::new(reader, writer);
    
    let start = SystemTime::now();
    let mut sz_b = [0 as u8; 8];
    receiver.read_exact(&mut sz_b).unwrap();
    let sz = u64::from_le_bytes(sz_b);
    let mut gbc_b = vec![0 as u8; sz as usize]; let gbc_s;
    receiver.read_exact(&mut gbc_b).unwrap();
    gbc_s = std::str::from_utf8(&gbc_b).unwrap();

    let gbc: GarbledCircuit = serde_json::from_str(gbc_s).unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Evaluator :: Receiving & parsing garbled circuit: {} ms\nPer permutation: {} us",
        timing, ((timing*1000) as f64) / (p_runs * s_runs) as f64
    );
    write!(file, "Evaluator :: Receiving & parsing garbled circuit: {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / (p_runs * s_runs) as f64
    ).unwrap();

    let start = SystemTime::now();
    let mut ev_ext = Evaluator::<MyChannel, AesRng, OtReceiver>::new(channel, rng).unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Evaluator :: Initialization ext: {} ms",
        timing
    );
    write!(file,
        "Evaluator :: Initialization ext: {} ms\n",
        timing
    ).unwrap();

    let start = SystemTime::now();
    let mut xs = Vec::new(); 
    let mut ys = Vec::new();
    for _ in 0..p_runs {
        ev_ext.receive_many(&vec![*modulus; n_gb_inputs]).unwrap().into_iter().for_each(|w| xs.push(w));
    }
    ev_ext.encode_many(&ev_inputs, &vec![*modulus; n_ev_inputs]).unwrap().into_iter().for_each(|w| ys.push(w));
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Evaluator :: Encoding inputs (with OT): {} ms\nPer permutation: {} us",
        timing, ((timing*1000) as f64) / (p_runs * s_runs) as f64
    );
    write!(file,
        "Evaluator :: Encoding inputs (with OT): {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / (p_runs * s_runs) as f64
    ).unwrap();

    let start = SystemTime::now();
    let output = gbc.eval(circ, &xs, &ys).unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Evaluator :: Circuit evaluation: {} ms\nPer permutation: {} us",
        timing, ((timing * 1000) as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file,
        "Evaluator :: Circuit evaluation: {} ms\nPer permutation: {} us\n",
        timing, ((timing * 1000) as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();

    let out = output.into_iter().map(|o| {
        ev_ext.get_channel().write_u16(o).unwrap();
        ev_ext.get_channel().flush().unwrap();
        o
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
    let mut output;
    let pre = SystemTime::now();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(!Path::new("./helper_test_files/output_TCPnonstr_log.txt").exists())
        .append(true)
        .open("./helper_test_files/output_TCPnonstr_log.txt")
        .unwrap();

    write!(file, "--- EVALUATOR START: {} permutation(s) in series ---
                   {} permutation(s) in parallel
---           PHOTON{}                ---\n\n",
                s_runs, p_runs, perm_id).unwrap();

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

    let listener = TcpListener::bind(EV_ADDR).unwrap();
    println!("Evaluator listening on {}", EV_ADDR);

    let pre_tot = pre.elapsed().unwrap().as_millis();
    loop {
        match listener.accept() {
            Ok((receiver, addr)) => {
                let total = SystemTime::now();
                println!("Garbler connected on {}", addr);
                
                if gb_ev == "ev" {
                    let mut evs = vec![0; p_runs*input.len()];
                    (0..p_runs*input.len()).for_each(|i| evs[i] = input[i % input.len()]);
                    output = run_circuit(&circ, receiver, &evs, 0, &modulus, p_runs, s_runs);
                } else {
                    output = run_circuit(&circ, receiver, &[], d*d, &modulus, p_runs, s_runs);
                }
    
                println!("done: {:?}", output);
                let tot = total.elapsed().unwrap().as_millis();
                println!("Total: {} ms", tot + pre_tot);
                println!("Average computing time / permutation: {} ms", ((tot + pre_tot) as f64)/((s_runs * p_runs) as f64));
                write!(file, "Evaluator :: Total: {} ms\n 
                              Average computing time / permutation: {} ms\n\n\n", tot + pre_tot, ((tot + pre_tot) as f64)/((s_runs * p_runs) as f64)).unwrap();
            }
            Err(e) => println!("Connection failed: {}", e),
        }
    }


}