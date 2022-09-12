// -*- mode: rust; -*-

extern crate fancy_garbling;

// use criterion::{criterion_group, criterion_main, Criterion};
use fancy_garbling::{
    circuit::{Circuit, CircuitBuilder, CircuitRef},
    Modulus, photon_bin::PhotonFancyExt, Fancy,
    twopac::semihonest::Garbler, FancyInput, 
};
use itertools::Itertools;
use ocelot::ot::AlszSender as OtSender;
use scuttlebutt::{AesRng, Channel, AbstractChannel};
use std::{
    io::{BufReader, BufWriter, Write},
    time::SystemTime, net::TcpStream, env, fs, path::Path,
};

const EV_ADDR: &str = "10.2.33.45:9481";
// const EV_ADDR: &str = "127.0.0.1:9481";

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
        ((timing*1000) as f64) / (pruns * sruns) as f64
    );
    write!(file, "Garbler :: Building circuit: {} ms\nPer permutation: {} us\n",
        timing,
        ((timing*1000) as f64) / (pruns * sruns) as f64
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

fn run_circuit(circ: &Circuit, sender: TcpStream, gb_inputs: &[u16], n_ev_inputs: usize, d: usize, n: usize, p_runs: usize, s_runs: usize) -> Vec<u16> {
    let n_gb_inputs = gb_inputs.len();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("./helper_test_files/output_TCP_log.txt")
        .unwrap();
    
    let rng = AesRng::new();
    let reader = BufReader::new(sender.try_clone().unwrap());
    let writer = BufWriter::new(sender.try_clone().unwrap());
    let channel = MyChannel::new(reader, writer);

    let mut gb = Garbler::<MyChannel, AesRng, OtSender>::new(channel, rng).unwrap();
    
    let start = SystemTime::now();
    let d_eff;
    if n_gb_inputs == 0 {
        d_eff = 0;
    } else {d_eff = d;}
    let mut gbs_4bit = Vec::with_capacity(p_runs*d_eff*d_eff*n);
    for i in 0..p_runs {
        gbs_4bit.extend(encode_input_bin(gb_inputs[i*d_eff*d_eff..(i*d_eff*d_eff+d_eff*d_eff)].to_vec(), d_eff, n));
    }
    let xs = gb.encode_many(&gbs_4bit, &vec![Modulus::Zq { q: 2 }; n_gb_inputs*n*p_runs]).unwrap();          // encoded garbler inputs - only W^0
    let ys = gb.receive_many(&vec![Modulus::Zq { q: 2 }; n_ev_inputs*p_runs*n]).unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Encoding & sending inputs with OT: {} ms\nPer permutation: {} us",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Encoding & sending inputs with OT: {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();

    let start = SystemTime::now();
    circ.eval(&mut gb, &xs, &ys).unwrap();
    let timing = start.elapsed().unwrap().as_millis();
    println!(
        "Garbler :: Garbling circuit: {} ms\nPer permutation: {} us",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    );
    write!(file, "Garbler :: Garbling circuit: {} ms\nPer permutation: {} us\n",
        timing, ((timing*1000) as f64) / ((s_runs*p_runs) as f64)
    ).unwrap();

    let out = (0..circ.noutputs()).map(|_| {
        gb.get_channel().flush().unwrap();
        let val = gb.get_channel().read_u16().unwrap();
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
    let circ;
    let d; let input; let n;
    let out;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create_new(!Path::new("./helper_test_files/output_TCP_log.txt").exists())
        .open("./helper_test_files/output_TCP_log.txt")
        .unwrap();


    write!(file, "--- BIN GARBLER START: {} permutation(s) in series ---
                   {} permutation(s) in parallel
                   {} has all inputs
---           PHOTON{}                ---\n\n",
                s_runs, p_runs, gb_ev, perm_id).unwrap();
    match perm_id.as_ref() {
        "100" => {
            // modulus = Modulus::GF4 { p: 19 };
            d = 5; n = 4;
            input =   vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,4,1,4,1,0];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "144" => {
            // modulus = Modulus::GF4 { p: 19 };
            d = 6; n = 4;
            input = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,1,0,1,0];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_144, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_144, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "196" => {
            // modulus = Modulus::GF4 { p: 19 };
            d = 7; n = 4;
            input = vec![0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,2,8,2,4,2,4];
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_196, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_196, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "256" => {
            // modulus = Modulus::GF4 { p: 19 };
            d = 8; n = 4;
            input = vec!(0, 0 ,0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ,0, 0, 0, 0, 0, 3, 0, 0 ,0, 0, 0, 0, 0, 8, 0, 0 ,0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0);
            if gb_ev == "ev" {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_256, &mut evaluator_input, &input, d, 4, s_runs, p_runs)
            } else {
                circ = build_photon_circuit_bin(&mut <CircuitBuilder as PhotonFancyExt>::photon_256, &mut garbler_input, &input, d, 4, s_runs, p_runs)
            }
        },
        "288" => {
            // modulus = Modulus::GF8 { p: 283 };
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

    loop {
        match TcpStream::connect(EV_ADDR) {
            Ok(sender) => {
                let total = SystemTime::now();
                println!("Successfully connected to evaluator on {}", EV_ADDR);
                if gb_ev == "ev" {
                    out = run_circuit(&circ, sender, &[], d*d, d, n, p_runs,s_runs);
                } else {
                    let mut gbs = vec![0; p_runs*input.len()];
                    (0..p_runs*input.len()).for_each(|i| gbs[i] = input[i % input.len()]);
                    out = run_circuit(&circ, sender, &gbs, 0, d, n, p_runs, s_runs);
                }
                println!("output: {:?}", out);
                let tot = total.elapsed().unwrap().as_millis();
                println!("Total: {} ms", tot);
                println!("Average computing time / permutation: {} ms", (tot as f64)/((s_runs * p_runs) as f64));
                write!(file, "Garbler :: Total: {} ms\n 
                            Average computing time / permutation: {} ms\n
    --------------------------------------\n\n", tot, (tot as f64)/((s_runs * p_runs) as f64)).unwrap();
                break;

            }
            Err(e) => println!("Failed to connect to evaluator: {}\nTrying again...", e)
        }
    }

}

