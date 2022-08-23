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
        FancyInput, Modulus,
    };
    use itertools::Itertools;
    use ocelot::ot::{ChouOrlandiReceiver, ChouOrlandiSender};
    use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};

    const SBOX_PRE: &[u16] =  &[0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
    const SBOX_AES: &[u16] =  &[0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16];


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
    fn test_photon80() {
        let mut rng = rand::thread_rng();
        let p = Modulus::GF4 { p: 19 };
        let d = 5;
        let n = d*d;
        let input = (0..n).map(|_| rng.gen_u16() % 16).collect::<Vec<u16>>();
        println!("inp: {:?}", input);

        let ics = [0, 1, 3, 6, 4];
        let Z = [1, 2, 9, 9, 2];

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input =  dummy.encode_photon(&input, d, &p).unwrap();
        let target = photon_op(&mut dummy, &[dummy_input], &ics, &SBOX_PRE, &Z).unwrap();
        println!("trgt: {:?}", target);

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_photon(&input, d, &p).unwrap();
            photon_op(&mut gb, &[xs], &ics, &SBOX_PRE, &Z);
        });

        let rng = AesRng::new();
        let mut ev =
            Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_photon(d, &p).unwrap();
        let result = photon_op(&mut ev, &[xs], &ics, &SBOX_PRE, &Z).unwrap();
        println!("res: {:?}", result);
        assert_eq!(target, result);
    }

    #[test]
    fn test_photon192() {
        let mut rng = rand::thread_rng();
        let p = Modulus::GF4 { p: 19 };
        let d = 7;
        let n = d*d;
        let input = (0..n).map(|_| rng.gen_u16() % 16).collect::<Vec<u16>>();
        println!("inp: {:?}", input);

        let ics = [0, 1, 2, 5, 3, 6, 4];
        let Z = [1, 4, 6, 1, 1, 6, 4];

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input =  dummy.encode_photon(&input, d, &p).unwrap();
        let target = photon_op(&mut dummy, &[dummy_input], &ics, &SBOX_PRE, &Z).unwrap();
        println!("trgt: {:?}", target);

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_photon(&input, d, &p).unwrap();
            photon_op(&mut gb, &[xs], &ics, &SBOX_PRE, &Z);
        });

        let rng = AesRng::new();
        let mut ev =
            Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_photon(d, &p).unwrap();
        let result = photon_op(&mut ev, &[xs], &ics, &SBOX_PRE, &Z).unwrap();
        println!("res: {:?}", result);
        assert_eq!(target, result);
    }

    #[test]
    fn test_photon256() {
        let mut rng = rand::thread_rng();
        let p = Modulus::GF8 { p: 283 };
        let d = 6;
        let n = d*d;
        let input = (0..n).map(|_| rng.gen_u16() % 256).collect::<Vec<u16>>();
        println!("inp: {:?}", input);

        let ics = [0, 1, 3, 7, 6, 4];
        let Z = [2, 3, 1, 2, 1, 4];

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input =  dummy.encode_photon(&input, d, &p).unwrap();
        let target = photon_op(&mut dummy, &[dummy_input], &ics, &SBOX_AES, &Z).unwrap();
        println!("trgt: {:?}", target);

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_photon(&input, d, &p).unwrap();
            photon_op(&mut gb, &[xs], &ics, &SBOX_AES, &Z);
        });

        let rng = AesRng::new();
        let mut ev =
            Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_photon(d, &p).unwrap();
        let result = photon_op(&mut ev, &[xs], &ics, &SBOX_AES, &Z).unwrap();
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
