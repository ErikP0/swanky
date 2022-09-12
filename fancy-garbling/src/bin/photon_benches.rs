extern crate fancy_garbling;

use fancy_garbling::{circuit::{Circuit, CircuitBuilder, CircuitRef}, 
                    Modulus, photon::*, 
                     Fancy, photon_bin::*, twopac::semihonest::{Garbler, Evaluator},
                    FancyInput, classic::garble};
use ocelot::ot::{AlszReceiver as OtReceiver, AlszSender as OtSender};
use scuttlebutt::{AesRng,TrackChannel};
use std::{time::{SystemTime}, vec};
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
/// Inputs can be encoded as constants/garbler_inputs/evaluater_inputs in the circuit 

fn main() {
    const X4_X_1: u8 = 19;
    let poly: Modulus = Modulus::GF4 { p: X4_X_1 };

    let sruns = 20;
    let pruns = 20;

    // INPUT photon100
    let input_gf: Vec<u16> = vec![0, 0 ,0, 0, 4, 0, 0, 0, 0, 1, 0, 0 ,0, 0, 4, 0, 0 ,0, 0, 1, 0, 0 ,0, 1, 0];
    let input_bin: Vec<u16> = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,4,1,4,1,0];
    const D: usize = 5;

    // INPUT photon144 
    // let input_gf: Vec<u16> = vec![0, 0 ,0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0 ,0, 0, 0, 1, 0, 0 ,0, 0, 0, 0, 0, 0 ,0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    // let input_bin: Vec<u16> = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,1,0,1,0];
    // const D: usize = 6;
    
    // INPUT photon196
    // let input_gf: Vec<u16> = vec![0, 0 ,0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0 ,0, 0, 0, 0, 8, 0, 0 ,0, 0, 0, 0, 2, 0, 0 ,0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 4];
    // let input_bin: Vec<u16> = vec![0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,2,8,2,4,2,4];
    // const D: usize = 7;

    // INPUT photon256
    // let input_gf: Vec<u16> =vec!(0, 0 ,0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ,0, 0, 0, 0, 0, 3, 0, 0 ,0, 0, 0, 0, 0, 8, 0, 0 ,0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0);
    // let input_bin: Vec<u16> = vec![0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,3,8,2,0,2,0];
    // const D: usize = 8;



    // GF
    let circ_gf = build_circ_gf(&mut <CircuitBuilder as PhotonGadgets>::photon_100, &mut garbler_input, D, &input_gf, &poly);
    let circ_gf_extended = build_extended_circuit_gf(&mut <CircuitBuilder as PhotonGadgets>::photon_100, &mut CircuitBuilder::constant, D, &input_gf, &poly, sruns, pruns);
    let circ_gf_bench = benchmark_streaming(&circ_gf, input_gf.clone(), vec![]);
    let eval_time_gf = benchmark_non_streaming(&circ_gf_extended, vec![], vec![]);


    // BIN
    let circ_bin = build_circ_bin::<D,_,_>(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut garbler_input, &input_bin, 4);
    let circ_bin_extended = build_extended_circuit_bin::<D,_,_>(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut CircuitBuilder::constant, &input_bin, 4, sruns, pruns);
    let circ_bin_bench = benchmark_streaming(&circ_bin, encode_input_bin::<D>(input_bin.clone(), 4), vec![]);
    let eval_time_bin = benchmark_non_streaming(&circ_bin_extended, vec![], vec![]);

    println!("{}","__________________________Circuitinfo______________________________".yellow());
    println!("{}:","* GF circuit info".purple());
    circ_gf.print_info().unwrap();
    println!("{}:","* BIN circuit info".purple());
    circ_bin.print_info().unwrap();
    println!("{}","__________________________STREAMING-Benchdata______________________".yellow());
    println!("{}: \n {}","* GF circuit".purple(),circ_gf_bench);
    println!("{}: \n {}","* BIN circuit".purple(),circ_bin_bench);
    println!("{}","______________________Evaluating time (nonstreaming)_______________".yellow());
    println!("{}: \n {} {}","* GF circuit".purple(),(eval_time_gf as f64)/((sruns*pruns) as f64),"μs" );
    println!("{}: \n {} {}","* BIN circuit".purple(),(eval_time_bin as f64)/((sruns*pruns) as f64),"μs");

}




// STREAMING: Function to measure data transfer + computing time for a given circuit
fn benchmark_streaming(circ: &Circuit, gb_inputs: Vec<u16>, ev_inputs: Vec<u16>) -> BenchData {
    let mut benchdata = BenchData::new("ms","kB");
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

    //println!("OUTPUT: {:?}",output);
    benchdata
}
// Non-streaming benchmark to measure evaluating time of circuit. 
// The streaming benchmark can't measure this time because it is constantly waiting for the garbled circuit.
fn benchmark_non_streaming(circ: &Circuit, gb_inputs: Vec<u16>, ev_inputs: Vec<u16>) -> u128 {
    let (en,ev) = garble(&circ).unwrap();

    
    let xs = &en.encode_garbler_inputs(&gb_inputs);
    let ys = &en.encode_evaluator_inputs(&ev_inputs);
    

    // Run the garbled circuit evaluator.
    let start = SystemTime::now();
    let decoded = &ev.eval(&circ, xs, ys).unwrap();
    let evaluating_time = start.elapsed().unwrap().as_micros();

    //println!("Decoded: {:?}",decoded);
    // Run Dummy 
    let correct_output = circ.eval_plain(&gb_inputs, &ev_inputs);
    //println!("Correct_output: {:?}",correct_output);


    evaluating_time
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

fn build_extended_circuit_gf<P,F>(photon: &mut P, fcn_input: &mut F, d: usize, input: &[u16], poly: &Modulus, sruns: usize, pruns: usize) -> Circuit  
    where  P: FnMut(&mut CircuitBuilder, &Vec<CircuitRef>) -> Result<Vec<CircuitRef>, <CircuitBuilder as Fancy>::Error>,
           F: FnMut(&mut CircuitBuilder, u16, &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    let mut b = CircuitBuilder::new();
    let input_wires: Vec<Vec<CircuitRef>> = (0..pruns).map(|_| input.iter().map(|i| fcn_input(&mut b, *i, poly).unwrap()).collect()).collect();
    for x in input_wires.into_iter() {
        let mut z = x;
        for _ in 0..sruns {
            z = photon(&mut b, &z).unwrap();
        }
        b.outputs(&z).unwrap();
    }
    b.finish()
}

fn build_extended_circuit_bin<const D: usize,P,F>(photon: &mut P, fcn_input: &mut F, input: &[u16], n:usize, sruns: usize, pruns: usize) -> Circuit  
    where  P: FnMut(&mut CircuitBuilder, Vec<Vec<Vec<CircuitRef>>>) -> Result<Vec<Vec<Vec<CircuitRef>>>, <CircuitBuilder as Fancy>::Error>,
           F: FnMut(&mut CircuitBuilder, u16, &Modulus) -> Result<CircuitRef, <CircuitBuilder as Fancy>::Error> {
    let mut b = CircuitBuilder::new();
    let input_wires: Vec<Vec<Vec<Vec<CircuitRef>>>> = (0..pruns).map(|_| fill_nbit::<_, _, D>(input, &mut |i| fcn_input(&mut b, i as u16, &Modulus::Zq { q: 2 }).unwrap(),n)).collect();
    for x in input_wires.into_iter() {
        let mut z = x;
        for _ in 0..sruns {
            z = photon(&mut b, z).unwrap();
        }
        b.outputs(&z.into_iter().flatten().flatten().collect::<Vec<_>>()).unwrap();
    }
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
    unit_t: String,
    unit_mem: String,
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
    pub fn new(time_unit: &str, mem_unit: &str) -> BenchData {
        BenchData  {unit_t: time_unit.to_string(),
                    unit_mem: mem_unit.to_string(),
                    time_gb_encode_inputs: 0, 
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
    
    pub fn time_unit(&self) -> &str {
        self.unit_t.as_str()
    }

    pub fn mem_unit(&self) -> &str {
        self.unit_mem.as_str()
    }

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
        let unit_t = self.time_unit();
        let unit_mem = self.mem_unit();
        
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

