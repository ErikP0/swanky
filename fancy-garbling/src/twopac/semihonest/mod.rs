// -*- mode: rust; -*-
//
// This file is part of twopac.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! Implementation of semi-honest two-party computation.

mod evaluator;
mod garbler;

pub use evaluator::Evaluator;
pub use garbler::Garbler;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        circuit::Circuit,
        dummy::Dummy,
        util::RngExt,
        CrtBundle,
        CrtGadgets,
        Fancy,
        FancyInput, Modulus, PhotonState, PhotonGadgets,
    };
    use itertools::Itertools;
    use ocelot::ot::{ChouOrlandiReceiver, ChouOrlandiSender};
    use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};

    fn addition<F: Fancy>(f: &mut F, a: &F::Item, b: &F::Item) -> Result<Option<u16>, F::Error> {
        // let x3_x_1 = 11;
        // let a_ = f.cmul(a, x3_x_1).unwrap();
        let a_ = a;
        let c = f.add(&a_, &b)?;
        f.output(&c)
    }

    #[test]
    fn test_addition_circuit() {
        for a in 0..3 {
            for b in 0..3 {
                let (sender, receiver) = unix_channel_pair();
                std::thread::spawn(move || {
                    let rng = AesRng::new();
                    let mut gb =
                        Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng)
                            .unwrap();
                    let x = gb.encode(a, &Modulus::Zq { q: (3) }).unwrap();
                    let ys = gb.receive_many(&[Modulus::Zq { q: (3) }]).unwrap();
                    addition(&mut gb, &x, &ys[0]).unwrap();
                });
                let rng = AesRng::new();
                let mut ev =
                    Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng)
                        .unwrap();
                let x = ev.receive(&Modulus::Zq { q: (3) }).unwrap();
                let ys = ev.encode_many(&[b], &[Modulus::Zq { q: (3) }]).unwrap();
                let output = addition(&mut ev, &x, &ys[0]).unwrap().unwrap();
                assert_eq!((a + b) % 3, output);
            }
        }
    }

    #[test]
    fn test_addition_circuit_GF4() {
        for a in 0..16 {
            for b in 0..16{
                let (sender, receiver) = unix_channel_pair();
                std::thread::spawn(move || {
                    let rng = AesRng::new();
                    let mut gb =
                        Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng)
                            .unwrap();
                    let x = gb.encode(a, &Modulus::GF4 { p: 19 }).unwrap();
                    let ys = gb.receive_many(&[Modulus::GF4 { p: 19 }]).unwrap();
                    addition(&mut gb, &x, &ys[0]).unwrap();
                });
                let rng = AesRng::new();
                let mut ev =
                    Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng)
                        .unwrap();
                let x = ev.receive(&Modulus::GF4 { p: 19 }).unwrap();
                let ys = ev.encode_many(&[b], &[Modulus::GF4 { p: 19 }]).unwrap();
                let output = addition(&mut ev, &x, &ys[0]).unwrap().unwrap();
                assert_eq!( a ^ b, output);
            }
        }
    }

    fn photon_op<F: Fancy>(b: &mut F, xs: &[PhotonState<F::Item>], 
                           ics: &[u16], sbox: &'static[u16], Z: &[u16]) -> Option<Vec<Vec<u16>>> {
        let mut outputs = Vec::new();
        for x in xs.iter() {
            let z = b.PermutePHOTON(x, ics, sbox, Z).unwrap();
            outputs.push(b.output_photon(&z).unwrap());
        }
        outputs.into_iter().collect()
    }

    #[test]
    fn test_photon() {
        let mut rng = rand::thread_rng();
        let p = Modulus::GF4 { p: 19 };
        let d = 5;
        let n = d*d;
        let input = (0..n).map(|_| rng.gen_u16() % 16).collect::<Vec<u16>>();
        println!("inp: {:?}", input);

        const sbox: &[u16] =  &[0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let ics = [0, 1, 3, 6, 4];
        let Z = [1, 2, 9, 9, 2];

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input =  dummy.encode_photon(&input, d, &p).unwrap();
        let target = photon_op(&mut dummy, &[dummy_input], &ics, &sbox, &Z).unwrap();
        println!("trgt: {:?}", target);

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_photon(&input, d, &p).unwrap();
            photon_op(&mut gb, &[xs], &ics, &sbox, &Z);
        });

        let rng = AesRng::new();
        let mut ev =
            Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_photon(d, &p).unwrap();
        let result = photon_op(&mut ev, &[xs], &ics, &sbox, &Z).unwrap();
        println!("res: {:?}", result);
        assert_eq!(target, result);
    }

    fn relu<F: Fancy>(b: &mut F, xs: &[CrtBundle<F::Item>]) -> Option<Vec<u128>> {
        let mut outputs = Vec::new();
        for x in xs.iter() {
            let q = x.composite_modulus();
            let c = b.crt_constant_bundle(1, q).unwrap();
            let y = b.crt_mul(&x, &c).unwrap();
            let z = b.crt_relu(&y, "100%", None).unwrap();
            outputs.push(b.crt_output(&z).unwrap());
        }
        outputs.into_iter().collect()
    }

    #[test]
    fn test_relu() {
        let mut rng = rand::thread_rng();
        let n = 10;
        let ps = crate::util::primes_with_width(10);
        let q = crate::util::product(&ps);
        let input = (0..n).map(|_| rng.gen_u128() % q).collect::<Vec<u128>>();

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input = input
            .iter()
            .map(|x| dummy.crt_encode(*x, q).unwrap())
            .collect_vec();
        let target = relu(&mut dummy, &dummy_input).unwrap();

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.crt_encode_many(&input, q).unwrap();
            relu(&mut gb, &xs);
        });

        let rng = AesRng::new();
        let mut ev =
            Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.crt_receive_many(n, q).unwrap();
        let result = relu(&mut ev, &xs).unwrap();
        assert_eq!(target, result);
    }

    #[test]
    fn test_aes() {
        let circ = Circuit::parse("circuits/AES-non-expanded.txt").unwrap();

        circ.print_info().unwrap();

        let circ_ = circ.clone();
        let (sender, receiver) = unix_channel_pair();
        let handle = std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_many(&vec![0_u16; 128], &vec![Modulus::Zq { q: (2) }; 128]).unwrap();
            let ys = gb.receive_many(&vec![Modulus::Zq { q: (2) }; 128]).unwrap();
            circ_.eval(&mut gb, &xs, &ys).unwrap();
        });
        let rng = AesRng::new();
        let mut ev =
            Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_many(&vec![Modulus::Zq { q: (2) }; 128]).unwrap();
        let ys = ev.encode_many(&vec![0_u16; 128], &vec![Modulus::Zq { q: (2) }; 128]).unwrap();
        circ.eval(&mut ev, &xs, &ys).unwrap();
        handle.join().unwrap();
    }
}
