// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! Structs and functions for creating, streaming, and evaluating garbled circuits.

mod evaluator;
mod garbler;

pub use crate::garble::{evaluator::Evaluator, garbler::Garbler};

////////////////////////////////////////////////////////////////////////////////
// tests

#[cfg(test)]
mod nonstreaming {
    use crate::{
        circuit::{Circuit, CircuitBuilder},
        classic::garble,
        fancy::{Bundle, BundleGadgets, Fancy},
        util::{self, RngExt},
        Modulus,
    };
    use itertools::Itertools;
    use rand::{thread_rng, SeedableRng};
    use scuttlebutt::{AesRng, Block};

    // helper
    fn garble_test_helper<F>(f: F)
    where
        F: Fn(&Modulus) -> Circuit,
    {
        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = rng.gen_prime();
            let mut c = &mut f(&Modulus::Zq { q });
            let (en, ev) = garble(&mut c).unwrap();
            for _ in 0..16 {
                let mut inps = Vec::new();
                for i in 0..c.num_evaluator_inputs() {
                    let q = c.evaluator_input_mod(i);
                    let x = rng.gen_u16() % q.size();
                    inps.push(x);
                }
                // Run the garbled circuit evaluator.
                let xs = &en.encode_evaluator_inputs(&inps);
                let decoded = &ev.eval(&mut c, &[], xs).unwrap();

                // Run the dummy evaluator.
                let should_be = c.eval_plain(&[], &inps).unwrap();
                assert_eq!(decoded[0], should_be[0]);
            }
        }
    }

    #[test] // add
    fn add() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let y = b.evaluator_input(q);
            let z = b.add(&x, &y).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // add_many
    fn add_many() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let xs = b.evaluator_inputs(&vec![*q; 16]);
            let z = b.add_many(&xs).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // or_many
    fn or_many() {
        garble_test_helper(|_| {
            let mut b = CircuitBuilder::new();
            let xs = b.evaluator_inputs(&vec![Modulus::Zq { q:2 }; 16]);
            let z = b.or_many(&xs).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // sub
    fn sub() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let y = b.evaluator_input(q);
            let z = b.sub(&x, &y).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // cmul
    fn cmul() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z;
            if q.size() > 2 {
                z = b.cmul(&x, 2).unwrap();
            } else {
                z = b.cmul(&x, 1).unwrap();
            }
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // proj_cycle
    fn proj_cycle() {
        garble_test_helper(|q| {
            let mut tab = Vec::new();
            for i in 0..q.size() {
                tab.push((i + 1) % q.size());
            }
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z = b.proj(&x, q, Some(tab)).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // proj_rand
    fn proj_rand() {
        garble_test_helper(|q| {
            let mut rng = thread_rng();
            let mut tab = Vec::new();
            for _ in 0..q.size() {
                tab.push(rng.gen_u16() % q.size());
            }
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z = b.proj(&x, q, Some(tab)).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // mod_change
    fn mod_change() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z = b.mod_change(&x, q.size() * 2).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // half_gate
    fn half_gate() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let y = b.evaluator_input(q);
            let z = b.mul(&x, &y).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // half_gate_unequal_mods
    fn half_gate_unequal_mods() {
        let mut rng = AesRng::from_seed(Block::from(0 as u128));
        for q in 3..16 {
            let ymod = 2 + rng.gen_u16() % 6; // lower mod is capped at 8 for now
            println!("\nTESTING MOD q={} ymod={}", q, ymod);

            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(&Modulus::Zq { q });
            let y = b.evaluator_input(&Modulus::Zq { q: ymod });
            let z = b.mul(&x, &y).unwrap();
            b.output(&z).unwrap();
            let mut c = b.finish();

            let (en, ev) = garble(&mut c).unwrap();

            for x in 0..q {
                for y in 0..ymod {
                    println!("TEST x={} y={}", x, y);
                    let xs = &en.encode_evaluator_inputs(&[x, y]);
                    let decoded = &ev.eval(&mut c, &[], xs).unwrap();
                    let should_be = c.eval_plain(&[], &[x, y]).unwrap();
                    assert_eq!(decoded[0], should_be[0]);
                }
            }
        }
    }

    #[test] // mixed_radix_addition
    fn mixed_radix_addition() {
        let mut rng = thread_rng();

        let nargs = 2 + rng.gen_usize() % 100;
        let mods = vec![3, 7, 10, 2, 13];
        let modsM = mods.iter().map(|q| Modulus::Zq { q: *q }).collect::<Vec<_>>();

        let mut b = CircuitBuilder::new();
        let xs = (0..nargs)
            .map(|_| Bundle::new(b.evaluator_inputs(&modsM)))
            .collect_vec();
        let z = b.mixed_radix_addition(&xs).unwrap();
        b.output_bundle(&z).unwrap();
        let mut circ = b.finish();

        let (en, ev) = garble(&mut circ).unwrap();
        println!("mods={:?} nargs={} size={}", modsM, nargs, ev.size());

        let Q: u128 = mods.iter().map(|&q| q as u128).product();

        // test random values
        for _ in 0..16 {
            let mut should_be = 0;
            let mut ds = Vec::new();
            for _ in 0..nargs {
                let x = rng.gen_u128() % Q;
                should_be = (should_be + x) % Q;
                ds.extend(util::as_mixed_radix(x, &mods).iter());
            }
            let X = en.encode_evaluator_inputs(&ds);
            let outputs = ev.eval(&mut circ, &[], &X).unwrap();
            assert_eq!(util::from_mixed_radix(&outputs, &mods), should_be);
        }
    }

    #[test] // basic constants
    fn basic_constant() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();

        let q = rng.gen_modulus();
        let c = rng.gen_u16() % q;

        let y = b.constant(c, &Modulus::Zq { q }).unwrap();
        b.output(&y).unwrap();

        let mut circ = b.finish();
        let (_, ev) = garble(&mut circ).unwrap();

        for _ in 0..64 {
            let outputs = circ.eval_plain(&[], &[]).unwrap();
            assert_eq!(outputs[0], c, "plaintext eval failed");
            let outputs = ev.eval(&mut circ, &[], &[]).unwrap();
            assert_eq!(outputs[0], c, "garbled eval failed");
        }
    }

    #[test] // constants
    fn constants() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();

        let q = rng.gen_modulus();
        let c = rng.gen_u16() % q;

        let x = b.evaluator_input(&Modulus::Zq { q });
        let y = b.constant(c, &Modulus::Zq { q }).unwrap();
        let z = b.add(&x, &y).unwrap();
        b.output(&z).unwrap();

        let mut circ = b.finish();
        let (en, ev) = garble(&mut circ).unwrap();

        for _ in 0..64 {
            let x = rng.gen_u16() % q;
            let outputs = circ.eval_plain(&[], &[x]).unwrap();
            assert_eq!(outputs[0], (x + c) % q, "plaintext");

            let X = en.encode_evaluator_inputs(&[x]);
            let Y = ev.eval(&mut circ, &[], &X).unwrap();
            assert_eq!(Y[0], (x + c) % q, "garbled");
        }
    }
}

#[cfg(test)]
mod streaming {
    use crate::{
        dummy::{Dummy, DummyVal},
        util::RngExt,
        Evaluator,
        Fancy,
        FancyInput,
        Garbler,
        Wire,
        Modulus,
    };
    use itertools::Itertools;
    use rand::thread_rng;
    use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};

    // helper - checks that Streaming evaluation of a fancy function equals Dummy
    // evaluation of the same function
    fn streaming_test<FGB, FEV, FDU>(
        mut f_gb: FGB,
        mut f_ev: FEV,
        mut f_du: FDU,
        input_mods: &[Modulus],
    ) where
        FGB: FnMut(&mut Garbler<UnixChannel, AesRng>, &[Wire]) -> Option<u16> + Send + Sync,
        FEV: FnMut(&mut Evaluator<UnixChannel>, &[Wire]) -> Option<u16>,
        FDU: FnMut(&mut Dummy, &[DummyVal]) -> Option<u16>,
    {
        let mut rng = AesRng::new();
        let inputs = input_mods.iter().map(|q| rng.gen_u16() % q.size()).collect_vec();

        // evaluate f_gb as a dummy
        let mut dummy = Dummy::new();
        let dinps = dummy.encode_many(&inputs, input_mods).unwrap();
        let should_be = f_du(&mut dummy, &dinps).unwrap();

        let (sender, receiver) = unix_channel_pair();

        crossbeam::scope(|s| {
            s.spawn(move |_| {
                let mut gb = Garbler::new(sender, rng);
                let (gb_inp, ev_inp) = gb.encode_many_wires(&inputs, &input_mods).unwrap();
                for w in ev_inp.iter() {
                    gb.send_wire(w).unwrap();
                }
                f_gb(&mut gb, &gb_inp);
            });

            let mut ev = Evaluator::new(receiver);
            let ev_inp = input_mods
                .iter()
                .map(|q| ev.read_wire(q).unwrap())
                .collect_vec();
            let result = f_ev(&mut ev, &ev_inp).unwrap();

            assert_eq!(result, should_be)
        })
        .unwrap();
    }

    #[test]
    fn addition() {
        fn fancy_addition<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.add(&xs[0], &xs[1]).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::Zq { q: rng.gen_modulus() };
            streaming_test(
                move |b, xs| fancy_addition(b, xs),
                move |b, xs| fancy_addition(b, xs),
                move |b, xs| fancy_addition(b, xs),
                &[q, q],
            );
        }
    }

    #[test]
    fn subtraction() {
        fn fancy_subtraction<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.sub(&xs[0], &xs[1]).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::Zq { q: rng.gen_modulus() };
            streaming_test(
                move |b, xs| fancy_subtraction(b, xs),
                move |b, xs| fancy_subtraction(b, xs),
                move |b, xs| fancy_subtraction(b, xs),
                &[q, q],
            );
        }
    }

    #[test]
    fn multiplication() {
        fn fancy_multiplication<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.mul(&xs[0], &xs[1]).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::Zq { q: rng.gen_modulus() };
            streaming_test(
                move |b, xs| fancy_multiplication(b, xs),
                move |b, xs| fancy_multiplication(b, xs),
                move |b, xs| fancy_multiplication(b, xs),
                &[q, q],
            );
        }
    }

    #[test]
    fn cmul() {
        fn fancy_cmul<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.cmul(&xs[0], 5).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::Zq { q: rng.gen_modulus() };
            streaming_test(
                move |b, xs| fancy_cmul(b, xs),
                move |b, xs| fancy_cmul(b, xs),
                move |b, xs| fancy_cmul(b, xs),
                &[q],
            );
        }
    }

    #[test]
    fn proj() {
        fn fancy_projection<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let tab = (0..q.size()).map(|i| (i + 1) % q.size()).collect_vec();
            let z = b.proj(&xs[0], q, Some(tab)).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::Zq { q: rng.gen_modulus() };
            streaming_test(
                move |b, xs| fancy_projection(b, xs, &q),
                move |b, xs| fancy_projection(b, xs, &q),
                move |b, xs| fancy_projection(b, xs, &q),
                &[q],
            );
        }
    }
}

#[cfg(test)]
mod complex {
    use crate::{
        dummy::Dummy,
        util::RngExt,
        CrtBundle,
        CrtGadgets,
        Evaluator,
        Fancy,
        FancyInput,
        Garbler,
        Modulus,
    };
    use itertools::Itertools;
    use rand::thread_rng;
    use scuttlebutt::{unix_channel_pair, AesRng};

    fn complex_gadget<F: Fancy>(
        b: &mut F,
        xs: &[CrtBundle<F::Item>],
    ) -> Result<Option<Vec<u128>>, F::Error> {
        let mut zs = Vec::with_capacity(xs.len());
        for x in xs.iter() {
            let c = b.crt_constant_bundle(1, x.composite_modulus())?;
            let y = b.crt_mul(x, &c)?;
            let z = b.crt_relu(&y, "100%", None)?;
            zs.push(z);
        }
        b.crt_outputs(&zs)
    }

    #[test]
    fn test_complex_gadgets() {
        let mut rng = thread_rng();
        let N = 10;
        let qs = crate::util::primes_with_width(10);
        let Q = crate::util::product(&qs);
        for _ in 0..16 {
            let input = (0..N).map(|_| rng.gen_u128() % Q).collect_vec();

            // Compute the correct answer using `Dummy`.
            let mut dummy = Dummy::new();
            let dinps = input
                .iter()
                .map(|x| {
                    let xs = crate::util::crt(*x, &qs);
                    CrtBundle::new(dummy.encode_many(&xs, &qs.iter().map(|q| Modulus::Zq { q: *q }).collect::<Vec<_>>()).unwrap())
                })
                .collect_vec();
            let should_be = complex_gadget(&mut dummy, &dinps).unwrap();

            // test streaming garbler and evaluator
            let (sender, receiver) = unix_channel_pair();

            crossbeam::scope(|s| {
                s.spawn(move |_| {
                    let mut garbler = Garbler::new(sender, AesRng::new());

                    // encode input and send it to the evaluator
                    let mut gb_inp = Vec::with_capacity(N);
                    for X in &input {
                        let (zero, enc) = garbler.crt_encode_wire(*X, Q).unwrap();
                        for w in enc.iter() {
                            garbler.send_wire(w).unwrap();
                        }
                        gb_inp.push(zero);
                    }
                    complex_gadget(&mut garbler, &gb_inp).unwrap();
                });

                let mut evaluator = Evaluator::new(receiver);

                // receive encoded wires from the garbler thread
                let mut ev_inp = Vec::with_capacity(N);
                for _ in 0..N {
                    let ws = qs
                        .iter()
                        .map(|q| evaluator.read_wire(&Modulus::Zq { q: *q }).unwrap())
                        .collect_vec();
                    ev_inp.push(CrtBundle::new(ws));
                }

                let result = complex_gadget(&mut evaluator, &ev_inp).unwrap();
                assert_eq!(result, should_be);
            })
            .unwrap();
        }
    }
}

#[cfg(test)]
mod GF4_nonstreaming {
    use crate::{
        circuit::{Circuit, CircuitBuilder},
        classic::garble,
        fancy::Fancy,
        Modulus,
    };
    use rand::{thread_rng, seq::SliceRandom, Rng};

    // helper
    fn garble_test_helper<F>(f: F)
        where
            F: Fn(&Modulus) -> Circuit,
    {
        let mut rng = thread_rng();
        for _ in 0..16 {
            let p = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            let mut c = &mut f(&p);
            let (en, ev) = garble(&mut c).unwrap();
            for _ in 0..16 {
                let mut inps = Vec::new();
                for _ in 0..c.num_evaluator_inputs() {
                    let x = (rng.gen::<u8>() & (15)) as u16;
                    inps.push(x);
                }
                // Run the garbled circuit evaluator.
                let xs = &en.encode_evaluator_inputs(&inps);
                let decoded = &ev.eval(&mut c, &[], xs).unwrap();

                // Run the dummy evaluator.
                let should_be = c.eval_plain(&[], &inps).unwrap();
                assert_eq!(decoded[0], should_be[0]);
            }
        }
    }

    #[test] // add
    fn add_GF4() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let y = b.evaluator_input(q);
            let z = b.add(&x, &y).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // add_many
    fn add_many() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let xs = b.evaluator_inputs(&vec![*q; 16]);
            let z = b.add_many(&xs).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // sub
    fn sub() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let y = b.evaluator_input(q);
            let z = b.sub(&x, &y).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // cmul
    fn cmul() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z;
            if q.size() > 2 {
                z = b.cmul(&x, 2).unwrap();
            } else {
                z = b.cmul(&x, 1).unwrap();
            }
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // proj_cycle
    fn proj_cycle() {
        garble_test_helper(|q| {
            let mut tab = Vec::new();
            for i in 0..q.size() {
                tab.push((i + 1) % q.size());
            }
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z = b.proj(&x, q, Some(tab)).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // proj_rand
    fn proj_rand() {
        garble_test_helper(|q| {
            let mut rng = thread_rng();
            let mut tab = Vec::new();
            for _ in 0..q.size() {
                tab.push((rng.gen::<u8>() & (15)) as u16);
            }
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z = b.proj(&x, q, Some(tab)).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // basic constants
    fn basic_constant() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();

        let p = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
        let c = (rng.gen::<u8>() & (15)) as u16;

        let y = b.constant(c, p).unwrap();
        b.output(&y).unwrap();

        let mut circ = b.finish();
        let (_, ev) = garble(&mut circ).unwrap();

        for _ in 0..64 {
            let outputs = circ.eval_plain(&[], &[]).unwrap();
            assert_eq!(outputs[0], c, "plaintext eval failed");
            let outputs = ev.eval(&mut circ, &[], &[]).unwrap();
            assert_eq!(outputs[0], c, "garbled eval failed");
        }
    }

    #[test] // constants
    fn constants() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();

        let p = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
        let c = (rng.gen::<u8>() & (15)) as u16;

        let x = b.evaluator_input(p);
        let y = b.constant(c, p).unwrap();
        let z = b.add(&x, &y).unwrap();
        b.output(&z).unwrap();

        let mut circ = b.finish();
        let (en, ev) = garble(&mut circ).unwrap();

        for _ in 0..64 {
            let x = (rng.gen::<u8>() & (15)) as u16;
            let outputs = circ.eval_plain(&[], &[x]).unwrap();
            assert_eq!(outputs[0], x ^ c, "plaintext");

            let X = en.encode_evaluator_inputs(&[x]);
            let Y = ev.eval(&mut circ, &[], &X).unwrap();
            assert_eq!(Y[0], x ^ c, "garbled");
        }
    }
}


#[cfg(test)]
mod GF4_streaming {
    use crate::{
        dummy::{Dummy, DummyVal},
        Evaluator,
        Fancy,
        FancyInput,
        Garbler,
        Wire,
        Modulus,
    };
    use itertools::Itertools;
    use rand::{thread_rng, seq::SliceRandom, Rng};
    use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};

    // helper - checks that Streaming evaluation of a fancy function equals Dummy
    // evaluation of the same function
    fn streaming_test_GF4<FGB, FEV, FDU>(
        mut f_gb: FGB,
        mut f_ev: FEV,
        mut f_du: FDU,
        input_mods: &[Modulus],
    ) where
        FGB: FnMut(&mut Garbler<UnixChannel, AesRng>, &[Wire]) -> Option<u16> + Send + Sync,
        FEV: FnMut(&mut Evaluator<UnixChannel>, &[Wire]) -> Option<u16>,
        FDU: FnMut(&mut Dummy, &[DummyVal]) -> Option<u16>,
    {
        let mut rng = AesRng::new();
        let inputs = input_mods.iter().map(|_| (rng.gen::<u8>() & (15)) as u16).collect_vec();

        // evaluate f_gb as a dummy
        let mut dummy = Dummy::new();
        let dinps = dummy.encode_many(&inputs, input_mods).unwrap();
        let should_be = f_du(&mut dummy, &dinps).unwrap();
        // println!("inp: {} -> {}", inputs[0], should_be);

        let (sender, receiver) = unix_channel_pair();

        crossbeam::scope(|s| {
            s.spawn(move |_| {
                let mut gb = Garbler::new(sender, rng);
                let (gb_inp, ev_inp) = gb.encode_many_wires(&inputs, &input_mods).unwrap();
                for w in ev_inp.iter() {
                    gb.send_wire(w).unwrap();
                }
                f_gb(&mut gb, &gb_inp);
            });

            let mut ev = Evaluator::new(receiver);
            let ev_inp = input_mods
                .iter()
                .map(|q| ev.read_wire(q).unwrap())
                .collect_vec();
            let result = f_ev(&mut ev, &ev_inp).unwrap();

            assert_eq!(result, should_be)
        })
            .unwrap();
    }

    #[test]
    fn addition() {
        fn fancy_addition<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.add(&xs[0], &xs[1]).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF4(
                move |b, xs| fancy_addition(b, xs),
                move |b, xs| fancy_addition(b, xs),
                move |b, xs| fancy_addition(b, xs),
                &[*q, *q],
            );
        }
    }

    #[test]
    fn subtraction() {
        fn fancy_subtraction<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.sub(&xs[0], &xs[1]).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF4(
                move |b, xs| fancy_subtraction(b, xs),
                move |b, xs| fancy_subtraction(b, xs),
                move |b, xs| fancy_subtraction(b, xs),
                &[*q, *q],
            );
        }
    }

    #[test]
    fn cmul() {
        fn fancy_cmul<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.cmul(&xs[0], 5).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF4(
                move |b, xs| fancy_cmul(b, xs),
                move |b, xs| fancy_cmul(b, xs),
                move |b, xs| fancy_cmul(b, xs),
                &[*q],
            );
        }
    }

    #[test]
    fn proj() {
        fn fancy_projection<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let tab = (0..q.size()).map(|i| (i + 1) % q.size()).collect_vec();
            let z = b.proj(&xs[0], q, Some(tab)).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF4(
                move |b, xs| fancy_projection(b, xs, &q),
                move |b, xs| fancy_projection(b, xs, &q),
                move |b, xs| fancy_projection(b, xs, &q),
                &[*q],
            );
        }
    }

    #[test]
    fn projproj() {
        fn fancy_2xprojection<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let tab = (0..q.size()).map(|i| (i + 1) % q.size()).collect_vec();
            let y = b.proj(&xs[0], q, Some(tab)).unwrap();
            let tab2 = (0..q.size()).map(|i| (i + 5) % q.size()).collect_vec();
            let z = b.proj(&y, q, Some(tab2)).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF4(
                move |b, xs| fancy_2xprojection(b, xs, &q),
                move |b, xs| fancy_2xprojection(b, xs, &q),
                move |b, xs| fancy_2xprojection(b, xs, &q),
                &[*q],
            );
        }
    }

    #[test]
    fn addproj() {
        fn fancy_addproj<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let y = b.add(&xs[0], &xs[1]).unwrap();
            let tab = (0..q.size()).map(|i| (i + 5) % q.size()).collect_vec();
            let z = b.proj(&y, q, Some(tab)).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF4(
                move |b, xs| fancy_addproj(b, xs, q),
                move |b, xs| fancy_addproj(b, xs, q),
                move |b, xs| fancy_addproj(b, xs, q),
                &[*q, *q],
            );
        }
    }

    #[test]
    fn all_op() {
        fn fancy_allop<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let y = b.add(&xs[0], &xs[1]).unwrap();
            let tab = (0..q.size()).map(|i| (i + 5) % q.size()).collect_vec();
            let z = b.proj(&y, q, Some(tab)).unwrap();
            let tab2 = (0..q.size()).map(|i| (i * 5) % q.size()).collect_vec();
            let z2 = b.proj(&y, q, Some(tab2)).unwrap();

            let a = b.sub(&z, &z2).unwrap();
            let a2 = b.cmul(&a, 12).unwrap();
            b.output(&a2).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF4(
                move |b, xs| fancy_allop(b, xs, q),
                move |b, xs| fancy_allop(b, xs, q),
                move |b, xs| fancy_allop(b, xs, q),
                &[*q, *q],
            );
        }
    }
}

#[cfg(test)]
mod GF8_nonstreaming {
    use crate::{
        circuit::{Circuit, CircuitBuilder},
        classic::garble,
        fancy::Fancy,
        Modulus,
    };
    use rand::{thread_rng, seq::SliceRandom, Rng};

    // helper
    fn garble_test_helper<F>(f: F)
        where
            F: Fn(&Modulus) -> Circuit,
    {
        let mut rng = thread_rng();
        for _ in 0..16 {
            let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            let mut c = &mut f(&p);
            let (en, ev) = garble(&mut c).unwrap();
            for _ in 0..16 {
                let mut inps = Vec::new();
                for _ in 0..c.num_evaluator_inputs() {
                    let x = rng.gen::<u8>() as u16;
                    inps.push(x);
                }
                // Run the garbled circuit evaluator.
                let xs = &en.encode_evaluator_inputs(&inps);
                let decoded = &ev.eval(&mut c, &[], xs).unwrap();

                // Run the dummy evaluator.
                let should_be = c.eval_plain(&[], &inps).unwrap();
                assert_eq!(decoded[0], should_be[0]);
            }
        }
    }

    #[test] // add
    fn add_GF8() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let y = b.evaluator_input(q);
            let z = b.add(&x, &y).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // add_many
    fn add_many() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let xs = b.evaluator_inputs(&vec![*q; 16]);
            let z = b.add_many(&xs).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // sub
    fn sub() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let y = b.evaluator_input(q);
            let z = b.sub(&x, &y).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // cmul
    fn cmul() {
        garble_test_helper(|q| {
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z;
            if q.size() > 2 {
                z = b.cmul(&x, 2).unwrap();
            } else {
                z = b.cmul(&x, 1).unwrap();
            }
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // proj_cycle
    fn proj_cycle() {
        garble_test_helper(|q| {
            let mut tab = Vec::new();
            for i in 0..q.size() {
                tab.push((i + 1) % q.size());
            }
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z = b.proj(&x, q, Some(tab)).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // proj_rand
    fn proj_rand() {
        garble_test_helper(|q| {
            let mut rng = thread_rng();
            let mut tab = Vec::new();
            for _ in 0..q.size() {
                tab.push((rng.gen::<u8>()) as u16);
            }
            let mut b = CircuitBuilder::new();
            let x = b.evaluator_input(q);
            let z = b.proj(&x, q, Some(tab)).unwrap();
            b.output(&z).unwrap();
            b.finish()
        });
    }

    #[test] // basic constants
    fn basic_constant() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();

        let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
        let c = (rng.gen::<u8>()) as u16;

        let y = b.constant(c, &p).unwrap();
        b.output(&y).unwrap();

        let mut circ = b.finish();
        let (_, ev) = garble(&mut circ).unwrap();

        for _ in 0..64 {
            let outputs = circ.eval_plain(&[], &[]).unwrap();
            assert_eq!(outputs[0], c, "plaintext eval failed");
            let outputs = ev.eval(&mut circ, &[], &[]).unwrap();
            assert_eq!(outputs[0], c, "garbled eval failed");
        }
    }

    #[test] // constants
    fn constants() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();

        let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
        let c = (rng.gen::<u8>()) as u16;

        let x = b.evaluator_input(&p);
        let y = b.constant(c, &p).unwrap();
        let z = b.add(&x, &y).unwrap();
        b.output(&z).unwrap();

        let mut circ = b.finish();
        let (en, ev) = garble(&mut circ).unwrap();

        for _ in 0..64 {
            let x = (rng.gen::<u8>()) as u16;
            let outputs = circ.eval_plain(&[], &[x]).unwrap();
            assert_eq!(outputs[0], x ^ c, "plaintext");

            let X = en.encode_evaluator_inputs(&[x]);
            let Y = ev.eval(&mut circ, &[], &X).unwrap();
            assert_eq!(Y[0], x ^ c, "garbled");
        }
    }
}


#[cfg(test)]
mod GF8_streaming {
    use crate::{
        dummy::{Dummy, DummyVal},
        Evaluator,
        Fancy,
        FancyInput,
        Garbler,
        Wire,
        Modulus,
    };
    use itertools::Itertools;
    use rand::{thread_rng, seq::SliceRandom, Rng};
    use scuttlebutt::{unix_channel_pair, AesRng, UnixChannel};

    // helper - checks that Streaming evaluation of a fancy function equals Dummy
    // evaluation of the same function
    fn streaming_test_GF8<FGB, FEV, FDU>(
        mut f_gb: FGB,
        mut f_ev: FEV,
        mut f_du: FDU,
        input_mods: &[Modulus],
    ) where
        FGB: FnMut(&mut Garbler<UnixChannel, AesRng>, &[Wire]) -> Option<u16> + Send + Sync,
        FEV: FnMut(&mut Evaluator<UnixChannel>, &[Wire]) -> Option<u16>,
        FDU: FnMut(&mut Dummy, &[DummyVal]) -> Option<u16>,
    {
        let mut rng = AesRng::new();
        let inputs = input_mods.iter().map(|_| (rng.gen::<u8>()) as u16).collect_vec();

        // evaluate f_gb as a dummy
        let mut dummy = Dummy::new();
        let dinps = dummy.encode_many(&inputs, input_mods).unwrap();
        let should_be = f_du(&mut dummy, &dinps).unwrap();

        let (sender, receiver) = unix_channel_pair();

        crossbeam::scope(|s| {
            s.spawn(move |_| {
                let mut gb = Garbler::new(sender, rng);
                let (gb_inp, ev_inp) = gb.encode_many_wires(&inputs, &input_mods).unwrap();
                for w in ev_inp.iter() {
                    gb.send_wire(w).unwrap();
                }
                f_gb(&mut gb, &gb_inp);
            });

            let mut ev = Evaluator::new(receiver);
            let ev_inp = input_mods
                .iter()
                .map(|q| ev.read_wire(q).unwrap())
                .collect_vec();
            let result = f_ev(&mut ev, &ev_inp).unwrap();

            assert_eq!(result, should_be)
        })
            .unwrap();
    }

    #[test]
    fn addition() {
        fn fancy_addition<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.add(&xs[0], &xs[1]).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF8(
                move |b, xs| fancy_addition(b, xs),
                move |b, xs| fancy_addition(b, xs),
                move |b, xs| fancy_addition(b, xs),
                &[*q, *q],
            );
        }
    }

    #[test]
    fn subtraction() {
        fn fancy_subtraction<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.sub(&xs[0], &xs[1]).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF8(
                move |b, xs| fancy_subtraction(b, xs),
                move |b, xs| fancy_subtraction(b, xs),
                move |b, xs| fancy_subtraction(b, xs),
                &[*q, *q],
            );
        }
    }

    #[test]
    fn cmul() {
        fn fancy_cmul<F: Fancy>(b: &mut F, xs: &[F::Item]) -> Option<u16> {
            let z = b.cmul(&xs[0], 5).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF8(
                move |b, xs| fancy_cmul(b, xs),
                move |b, xs| fancy_cmul(b, xs),
                move |b, xs| fancy_cmul(b, xs),
                &[*q],
            );
        }
    }

    #[test]
    fn proj() {
        fn fancy_projection<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let tab = (0..q.size()).map(|i| (i + 1) % q.size()).collect_vec();
            let z = b.proj(&xs[0], q, Some(tab)).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF8(
                move |b, xs| fancy_projection(b, xs, &q),
                move |b, xs| fancy_projection(b, xs, &q),
                move |b, xs| fancy_projection(b, xs, &q),
                &[*q],
            );
        }
    }

    #[test]
    fn projproj() {
        fn fancy_2xprojection<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let tab = (0..q.size()).map(|i| (i + 1) % q.size()).collect_vec();
            let y = b.proj(&xs[0], q, Some(tab)).unwrap();
            let tab2 = (0..q.size()).map(|i| (i + 5) % q.size()).collect_vec();
            let z = b.proj(&y, q, Some(tab2)).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF8(
                move |b, xs| fancy_2xprojection(b, xs, &q),
                move |b, xs| fancy_2xprojection(b, xs, &q),
                move |b, xs| fancy_2xprojection(b, xs, &q),
                &[*q],
            );
        }
    }

    #[test]
    fn addproj() {
        fn fancy_addproj<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let y = b.add(&xs[0], &xs[1]).unwrap();
            let tab = (0..q.size()).map(|i| (i + 5) % q.size()).collect_vec();
            let z = b.proj(&y, q, Some(tab)).unwrap();
            b.output(&z).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF8(
                move |b, xs| fancy_addproj(b, xs, &q),
                move |b, xs| fancy_addproj(b, xs, &q),
                move |b, xs| fancy_addproj(b, xs, &q),
                &[*q, *q],
            );
        }
    }

    #[test]
    fn all_op() {
        fn fancy_allop<F: Fancy>(b: &mut F, xs: &[F::Item], q: &Modulus) -> Option<u16> {
            let y = b.add(&xs[0], &xs[1]).unwrap();
            let tab = (0..q.size()).map(|i| (i + 5) % q.size()).collect_vec();
            let z = b.proj(&y, q, Some(tab)).unwrap();
            let tab2 = (0..q.size()).map(|i| (i * 5) % q.size()).collect_vec();
            let z2 = b.proj(&y, q, Some(tab2)).unwrap();

            let a = b.sub(&z, &z2).unwrap();
            let a2 = b.cmul(&a, 12).unwrap();
            b.output(&a2).unwrap()
        }

        let mut rng = thread_rng();
        for _ in 0..16 {
            let q = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            streaming_test_GF8(
                move |b, xs| fancy_allop(b, xs, &q),
                move |b, xs| fancy_allop(b, xs, &q),
                move |b, xs| fancy_allop(b, xs, &q),
                &[*q, *q],
            );
        }
    }
}