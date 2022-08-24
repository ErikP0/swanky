use fancy_garbling::{
    circuit::Circuit,circuit::CircuitBuilder,
    twopac::semihonest::{Evaluator, Garbler},
    FancyInput,
    photon::PhotonGadgets, Fancy
};
use fancy_garbling::Modulus;
use ocelot::ot::{AlszReceiver as OtReceiver, AlszSender as OtSender};
use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};
use std::time::SystemTime;

fn build_photon_circuit(poly: &Modulus, d: usize) -> Circuit {
    let mut b = CircuitBuilder::new();
    let x = b.garbler_inputs(&vec![*poly; d*d]); 
    let z = b.photon_100(&x).unwrap();
    b.outputs(&z).unwrap();
    b.finish()
}

fn run_circuit(circ: &Circuit, gb_inputs: Vec<u16>, ev_inputs: Vec<u16>, d: usize, poly: &'static Modulus) -> Vec<u16>{
    let circ_ = circ.clone();
    let (sender, receiver) = unix_channel_pair();
    let n_gb_inputs = gb_inputs.len();
    let total = SystemTime::now();
    let handle = std::thread::spawn(move || {
        let rng = AesRng::new();
        let start = SystemTime::now();
        let mut gb = Garbler::<UnixChannel, AesRng, OtSender>::new(sender, rng).unwrap();
        println!(
            "Garbler :: Initialization: {} ms",
            start.elapsed().unwrap().as_millis()
        );
        let start = SystemTime::now();
        let xs = gb.encode_many(&gb_inputs, &vec![*poly; n_gb_inputs]).unwrap();    // encoded garbler inputs - only W^0
        println!(
            "Garbler :: Encoding inputs: {} ms",
            start.elapsed().unwrap().as_millis()
        );
        let start = SystemTime::now();
        circ_.eval(&mut gb, &xs, &[]).unwrap();
        println!(
            "Garbler :: Circuit garbling: {} ms",
            start.elapsed().unwrap().as_millis()
        );
    });
    let rng = AesRng::new();
    let start = SystemTime::now();
    let mut ev = Evaluator::<UnixChannel, AesRng, OtReceiver>::new(receiver, rng).unwrap();
    println!(
        "Evaluator :: Initialization: {} ms",
        start.elapsed().unwrap().as_millis()
    );
    let start = SystemTime::now();
    let xs = ev.receive_many(&vec![*poly; n_gb_inputs]).unwrap();               // receive inputs in same order! moduli array ev == moduli array gb
    println!(
        "Evaluator :: Encoding inputs: {} ms",
        start.elapsed().unwrap().as_millis()
    );
    let start = SystemTime::now();
    let output = circ.eval(&mut ev, &xs, &[]).unwrap();
    println!(
        "Evaluator :: Circuit evaluation: {} ms",
        start.elapsed().unwrap().as_millis()
    );
    handle.join().unwrap();
    println!("Total: {} ms", total.elapsed().unwrap().as_millis());
    output.unwrap()
}



fn main() {
    let x4_x_1 = &Modulus::GF4 { p: 19 };
    let garbler_input =    vec!(0, 0 ,0, 0, 4,
                                0, 0, 0, 0, 1,
                                0, 0 ,0, 0, 4,
                                0, 0 ,0, 0, 1,
                                0, 0 ,0, 1, 0);

    let circ = build_photon_circuit(x4_x_1, 5);
    let output = run_circuit(&circ, garbler_input, vec![], 5, x4_x_1);
    println!("Output: {:?}",output);
}