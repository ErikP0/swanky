extern crate fancy_garbling;

use fancy_garbling::{informer::{InformerStats, Informer},
                     circuit::{Circuit, CircuitBuilder, CircuitRef}, 
                     dummy::Dummy, Modulus, photon::*, 
                     Fancy, photon_bin::*, twopac::semihonest::{Garbler, Evaluator},
                    FancyInput};
use ocelot::ot::{AlszReceiver as OtReceiver, AlszSender as OtSender};
use scuttlebutt::{AesRng, UnixChannel, TrackUnixChannel, TrackChannel};
use std::time::{SystemTime};
use std::{
    io::{BufReader, BufWriter},
    os::unix::net::UnixStream,
};
use colored::*;

type Reader = BufReader<UnixStream>;
type Writer = BufWriter<UnixStream>;
type MyChannel = TrackChannel<Reader, Writer>;

/// File to compare primitives/photon.rs (GF[2^4] or GF[2^8] arithmetic)
/// and primitives/photon_bin.rs (Only uses AND/OR/XOR gates (GF[2]))
/// Inputs are encoded as constants in the circuit 

fn main() {
    let input_gf: Vec<u16> = vec![0, 0 ,0, 0, 4, 0, 0, 0, 0, 1, 0, 0 ,0, 0, 4, 0, 0 ,0, 0, 1, 0, 0 ,0, 1, 0];
    let input_bin: Vec<u16> = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,4,1,4,1,0];

    // GF
    let circ_gf = build_circ_gf(&mut <CircuitBuilder as PhotonGadgets>::photon_100, &mut garbler_input, 5, &input_gf, &Modulus::GF4 { p: 19 });
    let circ_gf_bench = benchmark(&circ_gf, input_gf, vec![]);


    // BIN
    let circ_bin = build_circ_bin::<5,_,_>(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut garbler_input, &input_bin, 4);
    let circ_bin_bench = benchmark(&circ_bin, encode_input_bin::<5>(input_bin, 4), vec![]);

    println!("{}","__________________________Circuitinfo______________________".yellow());
    println!("{}:","* GF circuit info".purple());
    circ_gf.print_info().unwrap();
    println!("{}:","* BIN circuit info".purple());
    circ_bin.print_info().unwrap();
    println!("{}","__________________________Benchdata______________________".yellow());
    println!("{}: \n {}","* GF circuit benchdata".purple(),circ_gf_bench);
    println!("{}: \n {}","* BIN circuit benchdata".purple(),circ_bin_bench);
}




// STREAMING: Function to measure data transfer + computing time for a given circuit
fn benchmark(circ: &Circuit, gb_inputs: Vec<u16>, ev_inputs: Vec<u16>) -> BenchData {
    let mut benchdata = BenchData::new();
    let poly = circ.modulus(0);
    let circ_ = circ.clone();


    let (sender, receiver) = UnixStream::pair().unwrap();

    let n_gb_inputs = gb_inputs.len();
    let n_ev_inputs = ev_inputs.len();

    let total = SystemTime::now();
    let handle = std::thread::spawn(move || {
        let rng = AesRng::new();
        let poly_ = circ_.modulus(0);

        let reader = BufReader::new(sender.try_clone().unwrap());
        let writer = BufWriter::new(sender);
        let channel = TrackChannel::new(reader, writer);
        let mut gb = Garbler::<MyChannel, AesRng, OtSender>::new(channel, rng).unwrap();
       
        let start = SystemTime::now();
        let xs = gb.encode_many(&gb_inputs, &vec![poly_; n_gb_inputs]).unwrap();          // encoded garbler inputs - only W^0
        let ys = gb.receive_many(&vec![poly_; n_ev_inputs]).unwrap();               // encoded evaluator inputs - only W^0
        let time_enc_gb = start.elapsed().unwrap().as_millis(); 
        let start = SystemTime::now();
        circ_.eval(&mut gb, &xs, &ys).unwrap();
        let time_circ_garbling = start.elapsed().unwrap().as_millis();
        let gb_channel = gb.get_channel();
        let mem_gb_total_read = gb_channel.kilobytes_read();
        let mem_gb_total_written = gb_channel.kilobytes_written();


        (time_enc_gb,time_circ_garbling,mem_gb_total_read,mem_gb_total_written)
    });

    let rng = AesRng::new();
    let reader = BufReader::new(receiver.try_clone().unwrap());
    let writer = BufWriter::new(receiver);
    let channel = TrackChannel::new(reader, writer);
    let mut ev = Evaluator::<MyChannel, AesRng, OtReceiver>::new(channel, rng).unwrap();


    let start = SystemTime::now();
    let xs = ev.receive_many(&vec![poly; n_gb_inputs]).unwrap();               // receive inputs in same order! moduli array ev == moduli array gb
    let ys = ev.encode_many(&ev_inputs, &vec![poly; n_ev_inputs]).unwrap();
    benchdata.time_ev_encode_inputs = start.elapsed().unwrap().as_millis(); 


    let start = SystemTime::now();
    let output = circ.eval(&mut ev, &xs, &ys).unwrap();
    benchdata.time_circ_evaluating = start.elapsed().unwrap().as_millis();
    
    let ev_channel = ev.get_channel();
    benchdata.mem_ev_total_read = ev_channel.kilobytes_read();
    benchdata.mem_ev_total_written = ev_channel.kilobytes_written();


    (benchdata.time_gb_encode_inputs,
        benchdata.time_circ_garbling,
        benchdata.mem_gb_total_read,
        benchdata.mem_gb_total_written) = handle.join().unwrap();
    benchdata.total_time = total.elapsed().unwrap().as_millis();

    benchdata.set_mem_total_read();
    benchdata.set_mem_total_written();
    benchdata.set_total_mem();

    println!("OUTPUT: {:?}",output);
    benchdata
}



fn build_circ_gf<P,F>(photon: &mut P, fcn_input: &mut F, d: usize, input: &[u16], poly: &Modulus) -> Circuit 
    where P: FnMut(&mut CircuitBuilder, &Vec<CircuitRef>) -> Result<Vec<CircuitRef>, <CircuitBuilder as Fancy>::Error>,
          F: FnMut(&mut CircuitBuilder, u16, &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    debug_assert_eq!(d*d, input.len());

    let mut b = CircuitBuilder::new();
    let input_wires = input.iter().map(|i| fcn_input(&mut b, *i, poly).unwrap()).collect();
    let output_wires = photon(&mut b,&input_wires).unwrap();    
    b.outputs(&output_wires).unwrap();
    b.finish()
}

fn build_circ_bin<const D: usize, P,F>(photon: &mut P, fcn_input: &mut F, input: &[u16], n: usize) -> Circuit
    where P: FnMut(&mut CircuitBuilder, Vec<Vec<Vec<CircuitRef>>>) -> Result<Vec<Vec<Vec<CircuitRef>>>, <CircuitBuilder as Fancy>::Error>,
          F: FnMut(&mut CircuitBuilder, u16, &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    let mut b = CircuitBuilder::new();
    let input_wires = fill_nbit::<_, _, D>(input, &mut |i| fcn_input(&mut b, i as u16, &Modulus::Zq { q: 2 }).unwrap(),n);
    let output_wires: Vec<_> = photon(&mut b, input_wires).unwrap().into_iter().flatten().flatten().collect();
    b.outputs(&output_wires).unwrap();
    b.finish()
}




// Helper functions
fn garbler_input(b: &mut CircuitBuilder, _val: u16, modulus: &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    Ok(b.garbler_input(modulus))
}

fn evaluator_input(b: &mut CircuitBuilder, _val: u16, modulus: &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    Ok(b.evaluator_input(modulus))
}


fn fill_nbit<F, T, const D: usize>(bytes: &[u16], f: &mut F, n :usize) -> Vec<Vec<Vec<T>>>
    where F: FnMut(u16) -> T {
        assert_eq!(bytes.len(), D*D);
        let mut v = Vec::with_capacity(D);
        let mut cnt = 0;
        for _ in 0..D {
            let mut row = Vec::with_capacity(D);
            for _ in 0..D {
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

fn encode_input_bin<const D: usize>(input: Vec<u16>, n: usize) -> Vec<u16>{
    fill_nbit::<_, _, D>(&input, &mut |i| i,n).into_iter().flatten().flatten().collect()
}

pub struct BenchData {
    time_gb_encode_inputs: u128,
    time_circ_garbling: u128,
    time_ev_encode_inputs: u128,
    time_circ_evaluating: u128,
    total_time: u128,
    mem_gb_total_written: f64,
    mem_gb_total_read: f64,
    mem_ev_total_written: f64,
    mem_ev_total_read: f64,
    mem_total_written: f64,
    mem_total_read: f64,
    total_mem: f64, 
}

impl BenchData {
    pub fn new() -> BenchData {
        BenchData  {time_gb_encode_inputs: 0, 
                    time_ev_encode_inputs: 0, 
                    time_circ_garbling: 0, 
                    time_circ_evaluating: 0,
                    total_time: 0, 
                    mem_gb_total_written: 0.0, 
                    mem_gb_total_read: 0.0, 
                    mem_ev_total_written: 0.0, 
                    mem_ev_total_read: 0.0, 
                    mem_total_written: 0.0, 
                    mem_total_read: 0.0, 
                    total_mem: 0.0}
    }
    pub fn set_mem_total_written(&mut self) {
        self.mem_total_written = self.mem_gb_total_written + self.mem_ev_total_written;
    }

    pub fn set_mem_total_read(&mut self) {
        self.mem_total_read = self.mem_gb_total_read + self.mem_ev_total_read;
    }

    pub fn set_total_mem(&mut self) {
        self.total_mem = self.mem_total_written + self.mem_total_read;
    }
    
    /// Time for garbler to encode his inputs
    pub fn time_gb_enc(&self) -> u128 {
        self.time_gb_encode_inputs
    }

    pub fn time_ev_enc(&self) -> u128 {
        self.time_ev_encode_inputs
    }

    pub fn time_circ_garbling(&self) -> u128 {
        self.time_circ_garbling
    }

    pub fn time_circ_evaluating(&self) -> u128 {
        self.time_circ_evaluating
    }

    pub fn total_time(&self) -> u128 {
        self.total_time
    }

    pub fn mem_gb_written(&self) -> f64 {
        self.mem_gb_total_written
    }

    pub fn mem_gb_read(&self) -> f64 {
        self.mem_gb_total_read
    }

    pub fn mem_ev_written(&self) -> f64 {
        self.mem_ev_total_written
    }

    pub fn mem_ev_read(&self) -> f64 {
        self.mem_ev_total_read
    }
    
    pub fn mem_total_written(&self) -> f64 {
        self.mem_total_written
    }

    pub fn mem_total_read(&self) -> f64 {
        self.mem_total_read
    }

    pub fn total_mem(&self) -> f64 {
        self.total_mem
    }
}

impl std::fmt::Display for BenchData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let unit_t: &str = "ms";
        let unit_mem: &str = "Kb";
        
        writeln!(f, "{}", "Benchdata".green())?;
        writeln!(f, "  Time garbler encoding :      {:16} {}", self.time_gb_enc(), unit_t)?;
        writeln!(f, "  Time evaluator encoding :    {:16} {}", self.time_ev_enc(), unit_t)?;
        writeln!(f, "  Time circuit garbling :      {:16} {}", self.time_circ_garbling(), unit_t)?;
        writeln!(f, "  Time circuit evaluating :    {:16} {}", self.time_circ_evaluating(), unit_t)?;
        writeln!(f, "  Total time:                  {:16} {}", self.total_time(), unit_t)?;
        writeln!(f, "  Write memory garbler:        {:16} {}", self.mem_gb_written(),unit_mem)?;
        writeln!(f, "  Read memory garbler:         {:16} {}", self.mem_gb_read(),unit_mem)?;
        writeln!(f, "  Write memory Evaluator:      {:16} {}", self.mem_ev_written(),unit_mem)?;
        writeln!(f, "  Read memory Evaluator:       {:16} {}", self.mem_ev_read(),unit_mem)?;
        writeln!(f, "  Write memory total:          {:16} {}", self.mem_total_written(),unit_mem)?;
        writeln!(f, "  Read memory total:           {:16} {}", self.mem_total_read(),unit_mem)?;
        writeln!(f, "  TOTAL MEMORY:                {:16} {}", self.total_mem(),unit_mem)?;
        
        Ok(())
    }
} 

