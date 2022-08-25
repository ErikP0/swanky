extern crate fancy_garbling;

use fancy_garbling::{informer::{InformerStats, Informer},
                     circuit::{Circuit, CircuitBuilder, CircuitRef}, 
                     dummy::Dummy, Modulus, photon::PhotonGadgets, 
                     Fancy, photon_bin::PhotonFancyExt};
use ocelot::ot::{AlszReceiver as OtReceiver, AlszSender as OtSender};
use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};
use std::time::SystemTime;
use colored::*;


/// File to compare primitives/photon.rs (GF[2^4] or GF[2^8] arithmetic)
/// and primitives/photon_bin.rs (Only uses AND/OR/XOR gates (GF[2]))
/// Inputs are encoded as constants in the circuit 

fn main() {
    const INPUT_GF: [u16;25] = [0, 0 ,0, 0, 4, 0, 0, 0, 0, 1, 0, 0 ,0, 0, 4, 0, 0 ,0, 0, 1, 0, 0 ,0, 1, 0];
    const INPUT_BIN: [u8; 25] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,4,1,4,1,0];

    // GF
    let circ_gf = build_circ_gf(&mut <CircuitBuilder as PhotonGadgets>::photon_100, &mut CircuitBuilder::constant, 5, &INPUT_GF, &Modulus::GF4 { p: 19 });
    let circ_gf_stats = informer(&circ_gf);

    // BIN
    let circ_bin = build_circ_bin::<5, _,_>(&mut <CircuitBuilder as PhotonFancyExt>::photon_100, &mut CircuitBuilder::constant, &INPUT_BIN, 4);
    let circ_bin_stats = informer(&circ_bin);





    println!("{}: \n {}", "* GF circuit stats".purple(), circ_gf_stats);
    println!("{}: \n {}", "* BIN circuit stats".purple(), circ_bin_stats);
}


fn informer(circ: &Circuit) -> InformerStats {
    let mut inf = Informer::new(Dummy::new()); 
    circ.eval(&mut inf, &[], &[]).unwrap(); 
    inf.stats()
}

// Function to measure data transfer + computing time for a given circuit
// fn benchmark<T>(circ: &Circuit) -> BenchData<T> {



// }

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

fn build_circ_bin<const D: usize, P,F>(photon: &mut P, fcn_input: &mut F, input: &[u8], n: usize) -> Circuit
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


fn fill_nbit<F, T, const D: usize>(bytes: &[u8], f: &mut F, n :usize) -> Vec<Vec<Vec<T>>>
    where F: FnMut(u8) -> T {
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

struct BenchData<T: Default> {
    time_gb_encode_inputs: T,
    time_circ_garbling: T,
    time_ev_encode_inputs: T,
    time_circ_evaluating: T, 
    mem_gb_total_written: f64,
    mem_gb_total_read: f64,
    mem_ev_total_written: f64,
    mem_ev_total_read: f64,
    mem_total_written: f64,
    mem_total_read: f64,
    total_mem: f64, 
}

impl<T: Default + Copy> BenchData<T> {
    pub fn new() -> BenchData<T> {
        BenchData  {time_gb_encode_inputs: T::default(), 
                    time_ev_encode_inputs: T::default(), 
                    time_circ_garbling: T::default(), 
                    time_circ_evaluating: T::default(), 
                    mem_gb_total_written: 0.0, 
                    mem_gb_total_read: 0.0, 
                    mem_ev_total_written: 0.0, 
                    mem_ev_total_read: 0.0, 
                    mem_total_written: 0.0, 
                    mem_total_read: 0.0, 
                    total_mem: 0.0}
    }
    /// Time for garbler to encode his inputs
    pub fn time_gb_enc(&self) -> T {
        self.time_gb_encode_inputs
    }

    pub fn time_ev_enc(&self) -> T {
        self.time_ev_encode_inputs
    }

    pub fn time_circ_garbling(&self) -> T {
        self.time_circ_garbling
    }

    pub fn time_circ_evaluating(&self) -> T {
        self.time_circ_evaluating
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

impl<T: Default + std::fmt::Display + Copy> std::fmt::Display for BenchData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let unit_t: &str = "ms";
        let unit_mem: &str = "kbits";
        
        writeln!(f, "{}", "Benchdata".green())?;
        writeln!(f, "  Time garbler encoding :      {:16} {}", self.time_gb_enc(), unit_t)?;
        writeln!(f, "  Time evaluator encoding :    {:16} {}", self.time_ev_enc(), unit_t)?;
        writeln!(f, "  Time circuit garbling :      {:16} {}", self.time_circ_garbling(), unit_t)?;
        writeln!(f, "  Time circuit evaluating :    {:16} {} \n", self.time_circ_evaluating(), unit_t)?;
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

