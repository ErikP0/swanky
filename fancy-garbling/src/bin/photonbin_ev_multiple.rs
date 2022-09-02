// -*- mode: rust; -*-

extern crate fancy_garbling;

// use criterion::{criterion_group, criterion_main, Criterion};
use fancy_garbling::{
    circuit::{Circuit, CircuitBuilder, CircuitRef},
    twopac::semihonest::Evaluator,
    FancyInput, Modulus, photon_bin::PhotonFancyExt, Fancy, classic::GarbledCircuit,
};
use itertools::Itertools;
use ocelot::ot::AlszReceiver as OtReceiver;
use scuttlebutt::{AesRng, Channel, AbstractChannel};
use std::{
    io::{BufReader, BufWriter, Write, Read},
    net::{TcpStream, TcpListener},
    time::SystemTime, env, fs, path::Path,
};

const EV_ADDR: &str = "0.0.0.0:9481";

type Reader = BufReader<TcpStream>;
type Writer = BufWriter<TcpStream>;
type MyChannel = Channel<Reader, Writer>;

fn build_photon_circuit_bin<P,F>(photon: &mut P, fcn_input: &mut F, input: &[u16], d: usize, n: usize, sruns: usize, pruns: usize) -> Circuit  
where  P: FnMut(&mut CircuitBuilder, Vec<Vec<Vec<CircuitRef>>>) -> Result<Vec<Vec<Vec<CircuitRef>>>, <CircuitBuilder as Fancy>::Error>,
       F: FnMut(&mut CircuitBuilder, u16, &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> 
    {
    let start = SystemTime::now();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("./helper_test_files/output_TCP_log.txt")
        .unwrap();
    let mut b = CircuitBuilder::new();
    let input_wires: Vec<Vec<Vec<Vec<CircuitRef>>>> = (0..pruns).map(|_| fill_nbit::<_,_>(input, &mut |i| fcn_input(&mut b, i as u16, &Modulus::Zq { q: 2 }).unwrap(), d, n)).collect();
    for x in input_wires.into_iter() {
        let mut z = x;
        for _ in 0..sruns {
            z = photon(&mut b, z).unwrap();
        }
        b.outputs(&z.into_iter().flatten().flatten().collect::<Vec<_>>()).unwrap();
    }
    let out = b.finish();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Building circuit: {} ms\nPer permutation: {} us",
        timing,
        ((timing * 1000) as f64) / (pruns * sruns) as f64
    );
    write!(file, "Garbler :: Building circuit: {} ms\nPer permutation: {} us\n",
        timing,
        ((timing * 1000) as f64) / (pruns * sruns) as f64
    ).unwrap();
    out
    
}

// Helper functions
fn garbler_input(b: &mut CircuitBuilder, _val: u16, modulus: &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    Ok(b.garbler_input(modulus))
}

fn evaluator_input(b: &mut CircuitBuilder, _val: u16, modulus: &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    Ok(b.evaluator_input(modulus))
}

fn fill_nbit<F, T>(bytes: &[u16], f: &mut F, d: usize, n :usize) -> Vec<Vec<Vec<T>>>
    where F: FnMut(u16) -> T {
        assert_eq!(bytes.len(), d*d);
        let mut v = Vec::with_capacity(d);
        let mut cnt = 0;
        for _ in 0..d {
            let mut row = Vec::with_capacity(d);
            for _ in 0..d {
                let x = bytes[cnt];
                cnt += 1;
                let cell: Vec<_> = (0..n).map(|i| f((x >> i) & 0x1))
                    .collect();
                row.push(cell);
            }
            v.push(row);
        }
        return v;
    }

fn encode_input_bin(input: Vec<u16>, d: usize, n: usize) -> Vec<u16>{
    fill_nbit::<_, _>(&input, &mut |i| i, d, n).into_iter().flatten().flatten().collect()
}

fn run_circuit(circ: &Circuit, receiver: TcpStream, ev_inputs: &[u16], n_gb_inputs: usize, d: usize, n: usize, p_runs: usize, s_runs: usize) 
                -> Vec<u16> {
    let n_ev_inputs = ev_inputs.len();
    let d_eff;
    if n_ev_inputs == 0 {
        d_eff = 0;
    } else {d_eff = d;}
    let mut evs_4bit = Vec::with_capacity(p_runs*d_eff*d_eff);
    for i in 0..p_runs {
        evs_4bit.extend(encode_input_bin(ev_inputs[i*d_eff*d_eff..(i*d_eff*d_eff+d_eff*d_eff)].to_vec(), d_eff, n));
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("./helper_test_files/output_TCP_log.txt")
        .unwrap();

    let rng = AesRng::new();
    let reader = BufReader::new(receiver.try_clone().unwrap());
    let writer = BufWriter::new(receiver.try_clone().unwrap());
    let channel = Channel::new(reader, writer);
    let d_eff;
    if n_ev_inputs == 0 {
        d_eff = 0;
    } else {d_eff = d;}
    let mut evs_4bit = Vec::with_capacity(p_runs*d_eff*d_eff);
    for i in 0..p_runs {
        evs_4bit.extend(encode_input_bin(ev_inputs[i*d_eff*d_eff..(i*d_eff*d_eff+d_eff*d_eff)].to_vec(), d_eff, n));
    }
    let start = SystemTime::now();
    let mut ev = Evaluator::<MyChannel, AesRng, OtReceiver>::new(channel, rng).unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Evaluator :: Initialization: {} ms",
        timing
    );
    write!(file,
        "Evaluator :: Initialization: {} ms\n",
        timing
    ).unwrap();

    let start = SystemTime::now();
    let xs = ev.receive_many(&vec![Modulus::Zq { q: 2 }; n_gb_inputs*p_runs*n]).unwrap();
    let ys = ev.encode_many(&evs_4bit, &vec![Modulus::Zq { q: 2 }; n_ev_inputs*n]).unwrap();
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
    let output = circ.eval(&mut ev, &xs, &ys).unwrap().unwrap();
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
        ev.get_channel().write_u16(o).unwrap();
        ev.get_channel().flush().unwrap();
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
    let circ;
    let d; let input; let n;
    let output;
    // let pre = SystemTime::now();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(!Path::new("./helper_test_files/output_TCP_log.txt").exists())
        .append(true)
        .open("./helper_test_files/output_TCP_log.txt")
        .unwrap();

    write!(file, "--- BIN EVALUATOR START: {} permutation(s) in series ---
                   {} permutation(s) in parallel
                   {} has all inputs
---           PHOTON{}                ---\n\n",
                s_runs, p_runs, gb_ev, perm_id).unwrap();

    match perm_id.as_ref() {
        "100" => {
            d = 5; n = 4;
            input =   vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,4,1,4,1,0];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "144" => {
            d = 6; n = 4;
            input = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,1,0,1,0];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_144, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_144, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "196" => {
            d = 7; n = 4;
            input = vec![0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,2,8,2,4,2,4];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_196, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_196, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "256" => {
            d = 8; n = 4;
            input = vec!(0, 0 ,0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ,0, 0, 0, 0, 0, 3, 0, 0 ,0, 0, 0, 0, 0, 8, 0, 0 ,0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0);
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_256, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_256, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "288" => {
            d = 6; n = 8;
            input = vec![00, 00, 00, 00, 00, 00, 
                        00, 00, 00, 00, 00, 00, 
                        00, 00, 00, 00, 00, 00, 
                        00, 00, 00, 00, 00, 00, 
                        00, 00, 00, 00, 00, 00, 
                        00, 00, 00, 40, 20, 20 ];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_288, &mut evaluator_input, &input, d, 8, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_288, &mut garbler_input, &input, d, 8, s_runs, p_runs)
            }
        },
        &_ => panic!("Command line argument is not a right permutation ID!")
    }

    let listener = TcpListener::bind(EV_ADDR).unwrap();
    println!("Evaluator listening on {}", EV_ADDR);

    // let pre_tot = pre.elapsed().unwrap().as_millis();
    loop {
        match listener.accept() {
            Ok((receiver, addr)) => {
                let total = SystemTime::now();
                println!("Garbler connected on {}", addr);
                
                if gb_ev == "ev" {
                    let mut evs = vec![0; p_runs*input.len()];
                    (0..p_runs*input.len()).for_each(|i| evs[i] = input[i % input.len()]);
                    output = run_circuit(&circ, receiver, &evs, 0, d, n, p_runs, s_runs);
                } else {
                    output = run_circuit(&circ, receiver, &[], d*d, d, n, p_runs, s_runs);
                }
    
                println!("done: {:?}", output);
                let tot = total.elapsed().unwrap().as_millis();
                println!("Total: {} ms", tot);
                println!("Average computing time / permutation: {} ms", ((tot) as f64)/((s_runs * p_runs) as f64));
                write!(file, "Evaluator :: Total: {} ms\n 
                              Average computing time / permutation: {} ms\n\n\n", tot, ((tot) as f64)/((s_runs * p_runs) as f64)).unwrap();
            }
            Err(e) => println!("Connection failed: {}", e),
        }
        break;
    }


}