// -*- mode: rust; -*-

extern crate fancy_garbling;

// use criterion::{criterion_group, criterion_main, Criterion};
use fancy_garbling::{
    circuit::{Circuit, CircuitBuilder, CircuitRef},
    twopac::semihonest::Garbler,
    FancyInput, Modulus, photon_bin::PhotonFancyExt, Fancy, errors::CircuitBuilderError, classic::garble,
    Wire, 
};
use itertools::Itertools;
use ocelot::ot::{AlszSender as OtSender, Sender};
use scuttlebutt::{AesRng, Channel, AbstractChannel, Block};
use std::{
    io::{BufReader, BufWriter, Write},
    time::SystemTime, net::TcpStream, env, fs, path::Path,
};

// const EV_ADDR: &str = "10.2.33.45:9481";
const EV_ADDR: &str = "127.0.0.1:9481";

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
        .open("./helper_test_files/output_TCPnonstr_log.txt")
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
        "Garbler :: Building circuit: {} ms\nPer permutation: {} ms",
        timing,
        (timing as f64) / (pruns * sruns) as f64
    );
    write!(file, "Garbler :: Building circuit: {} ms\nPer permutation: {} ms\n",
        timing,
        (timing as f64) / (pruns * sruns) as f64
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

fn run_circuit(circ: &Circuit, mut sender: TcpStream, gb_inputs: &[u16], n_ev_inputs: usize, modulus: &Modulus, d: usize, n: usize, p_runs: usize, s_runs: usize) -> Vec<u16> {
    let n_gb_inputs = gb_inputs.len();
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
        "Garbler :: Garbling circuit: {} ms\nPer permutation: {} ms",
        timing, (timing as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Garbling circuit: {} ms\nPer permutation: {} ms\n",
        timing, (timing as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();

    let start = SystemTime::now();
    let gbc_ser = serde_json::to_string(&gbc).unwrap();
    println!("Size garbled circuit = {} bytes", gbc_ser.as_bytes().len());
    sender.write_all(&gbc_ser.as_bytes().len().to_le_bytes()).unwrap();
    sender.try_clone().unwrap().write_all(gbc_ser.as_bytes()).unwrap();
    sender.flush().unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Parsing & sending garbled circuit: {} ms\nPer permutation: {} ms\n",
        timing, (timing as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Parsing & sending garbled circuit: {} ms\nPer permutation: {} ms\n",
        timing, (timing as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();
    
    let mut ot = OtSender::init(&mut channel, &mut rng).unwrap();
    let start = SystemTime::now();
    let mut d_eff;
    if n_gb_inputs == 0 {
        d_eff = 0;
    } else {d_eff = d;}
    let gbs_4bit = encode_input_bin(gb_inputs.to_vec(), d_eff, n);
    let mut gbs = vec![0; p_runs*n_gb_inputs*n];
    (0..p_runs*n_gb_inputs*n).for_each(|i| gbs[i] = gbs_4bit[i % n_gb_inputs*n]);
    let encoded_gb = en.encode_garbler_inputs(&gbs);
    encoded_gb.iter().for_each(|wire| channel.write_block(&wire.as_block()).unwrap());

    let zero_ev = en.encode_evaluator_inputs(&vec![0; n_ev_inputs*p_runs*n]);

    let mut inputs = Vec::with_capacity(n*p_runs*n_ev_inputs as usize);
    let mut wire = Wire::default(); let mut delta = Wire::default();

    for run in 0..p_runs {
        inputs.clear();
        for i in 0..n_ev_inputs*n {    
            wire = zero_ev[i + run*n_ev_inputs*n].clone();
            delta = en.encode_evaluator_input(1, i + run*n_ev_inputs*n).negate().plus(&zero_ev[i + run*n_ev_inputs*n]);
            let input = (0..1)
                .map(|i| {
                    let zero = if i > 0{
                        Wire::rand(&mut rng, &Modulus::Zq { q: 2 })
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
        ot.send(&mut channel, &inputs, &mut rng).unwrap();
    }
    
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Encoding & sending inputs with OT: {} ms\nPer permutation: {} ms",
        timing, (timing as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Encoding & sending inputs with OT: {} ms\nPer permutation: {} ms\n",
        timing, (timing as f64) / ((s_runs*p_runs) as f64)
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
    let d; let input; let n;
    let out;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create_new(!Path::new("./helper_test_files/output_TCPnonstr_log.txt").exists())
        .open("./helper_test_files/output_TCPnonstr_log.txt")
        .unwrap();

    let total = SystemTime::now();

    write!(file, "--- GARBLER START: {} permutation(s) in series ---
                   {} permutation(s) in parallel
---           PHOTON{}                ---\n\n",
                s_runs, p_runs, perm_id).unwrap();
    match perm_id.as_ref() {
        "100" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 5; n = 4;
            input =   vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,4,1,4,1,0];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "144" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 6; n = 4;
            input = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,1,0,1,0];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_144, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_144, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "196" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 7; n = 4;
            input = vec![0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,2,8,2,4,2,4];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_196, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_196, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "256" => {
            modulus = Modulus::GF4 { p: 19 };
            d = 8; n = 4;
            input = vec!(0, 0 ,0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ,0, 0, 0, 0, 0, 3, 0, 0 ,0, 0, 0, 0, 0, 8, 0, 0 ,0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0);
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_256, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_256, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "288" => {
            modulus = Modulus::GF8 { p: 283 };
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
    match TcpStream::connect(EV_ADDR) {
        Ok(sender) => {
            println!("Successfully connected to evaluator on {}", EV_ADDR);
            if gb_ev == "ev" {
                out = run_circuit(&circ, sender, &[], d*d, &modulus, d, n, p_runs,s_runs);
            } else {
                out = run_circuit(&circ, sender, &input, 0, &modulus, d, n, p_runs, s_runs);
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

