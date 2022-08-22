// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! Dummy implementation of `Fancy`.
//!
//! Useful for evaluating the circuits produced by `Fancy` without actually
//! creating any circuits.


use crate::{
    errors::{DummyError, FancyError},
    fancy::{Fancy, FancyInput, FancyReveal, HasModulus}, Modulus, util,
};

/// Simple struct that performs the fancy computation over `u16`.
pub struct Dummy {}

/// Wrapper around `u16`.
#[derive(Clone, Debug)]
pub struct DummyVal {
    val: u16,
    modulus: Modulus,
}

impl HasModulus for DummyVal {
    fn modulus(&self) -> Modulus {
        self.modulus
    }
}

impl std::fmt::Display for DummyVal {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(fmt, "{:#04x}", self.val())
    }
}

impl DummyVal {
    /// Create a new DummyVal.
    pub fn new(val: u16, modulus: &Modulus) -> Self {
        Self { val, modulus: *modulus }
    }

    /// Extract the value.
    pub fn val(&self) -> u16 {
        self.val
    }
}

impl Dummy {
    /// Create a new Dummy.
    pub fn new() -> Dummy {
        Dummy {}
    }
}

impl FancyInput for Dummy {
    type Item = DummyVal;
    type Error = DummyError;

    /// Encode a single dummy value.
    fn encode(&mut self, value: u16, modulus: &Modulus) -> Result<DummyVal, DummyError> {
        Ok(DummyVal::new(value, modulus))
    }

    /// Encode a slice of inputs and a slice of moduli as DummyVals.
    fn encode_many(&mut self, xs: &[u16], moduli: &[Modulus]) -> Result<Vec<DummyVal>, DummyError> {
        if xs.len() != moduli.len() {
            return Err(DummyError::EncodingError);
        }
        Ok(xs
            .iter()
            .zip(moduli.iter())
            .map(|(x, q)| DummyVal::new(*x, q))
            .collect())
    }

    fn receive_many(&mut self, _moduli: &[Modulus]) -> Result<Vec<DummyVal>, DummyError> {
        // Receive is undefined for Dummy which is a single party "protocol"
        Err(DummyError::EncodingError)
    }
}

impl Fancy for Dummy {
    type Item = DummyVal;
    type Error = DummyError;

    fn constant(&mut self, val: u16, modulus: &Modulus) -> Result<DummyVal, Self::Error> {
        Ok(DummyVal::new(val, modulus))
    }

    fn add(&mut self, x: &DummyVal, y: &DummyVal) -> Result<DummyVal, Self::Error> {
        if x.modulus() != y.modulus() {
            return Err(Self::Error::from(FancyError::UnequalModuli));
        }
        let result = if let Modulus::Zq { q } = x.modulus() {
            (x.val + y.val) % q
        } else {
            x.val ^ y.val
        };

        Ok(DummyVal {
            val: result,
            modulus: x.modulus,
        })
    }

    fn sub(&mut self, x: &DummyVal, y: &DummyVal) -> Result<DummyVal, Self::Error> {
        if x.modulus() != y.modulus() {
            return Err(Self::Error::from(FancyError::UnequalModuli));
        }

        let result = if let Modulus::Zq { q } = x.modulus() {
            (q + x.val - y.val) % q
        } else {
            // In F2 subtraction is the same as addition ! 
            x.val ^ y.val
        };



        Ok(DummyVal {
            val: result,
            modulus: x.modulus,
        })
    }

    fn cmul(&mut self, x: &DummyVal, c: u16) -> Result<DummyVal, Self::Error> {

        let result = match x.modulus() {
            Modulus::Zq {q} => (x.val * c) % q,
            Modulus::GF4 { p } => {
                util::field_mul(x.val, c, p as u16, 4) as u16
            },
            Modulus::GF8 { p } => {
                util::field_mul(x.val, c, p, 8) as u16
            },
            Modulus::GFk { k, p } => {
                util::field_mul(x.val, c, p, k) as u16
            }
        };

        
        Ok(DummyVal {
            val: result,
            modulus: x.modulus,
        })
    }

    fn mul(&mut self, x: &DummyVal, y: &DummyVal) -> Result<DummyVal, Self::Error> {
        let result = match x.modulus() {
            Modulus::Zq { q } => (x.val * y.val) % q,
            Modulus::GF4 { p } => {
                util::field_mul(x.val, y.val  , p as u16, 4) as u16
            },
            Modulus::GF8 { p } => {
                util::field_mul(x.val, y.val, p, 8) as u16
            },
            Modulus::GFk { k, p } => {
                util::field_mul(x.val, y.val, p, k) as u16
            },
        };

        Ok(DummyVal {
            val: result,
            modulus: x.modulus,
        })
    }

    fn proj(
        &mut self,
        x: &DummyVal,
        modulus: &Modulus,
        tt: Option<Vec<u16>>,
    ) -> Result<DummyVal, Self::Error> {
        let tt = tt.ok_or_else(|| Self::Error::from(FancyError::NoTruthTable))?;
        let xmodulus = x.modulus();

        match *modulus {
            Modulus::Zq { q } => {
                if tt.len() < xmodulus.size() as usize || !tt.iter().all(|&x| x < q) {
                    return Err(Self::Error::from(FancyError::InvalidTruthTable));
                }
            },
            Modulus::GF4 { .. } => {
                if tt.len() < xmodulus.size() as usize || !tt.iter().all(|&x| x < 16) {
                    return Err(Self::Error::from(FancyError::InvalidTruthTable));
                }
            },
            Modulus::GF8 { .. } => {
                if tt.len() < xmodulus.size() as usize || !tt.iter().all(|&x| x < 256) {
                    return Err(Self::Error::from(FancyError::InvalidTruthTable));
                }
            },
            Modulus::GFk { k, .. } => {
                if tt.len() < xmodulus.size() as usize || !tt.iter().all(|&x| x < 2_u16.pow(k.into())) {
                    return Err(Self::Error::from(FancyError::InvalidTruthTable));
                }
            },
        }
       
        let val = tt[x.val as usize];
        // println!("xval: {}, val: {}, tt: {:?}", x.val, val, tt);
        Ok(DummyVal { val, modulus: *modulus })
    }

    fn output(&mut self, x: &DummyVal) -> Result<Option<u16>, Self::Error> {
        Ok(Some(x.val))
    }
}

impl FancyReveal for Dummy {
    fn reveal(&mut self, x: &DummyVal) -> Result<u16, DummyError> {
        Ok(x.val)
    }
}

#[cfg(test)]
mod bundle {
    use super::*;
    use crate::{
        fancy::{BinaryGadgets, Bundle, BundleGadgets, CrtGadgets},
        util::{self, RngExt},
    };
    use itertools::Itertools;
    use rand::thread_rng;

    const NITERS: usize = 1 << 10;

    #[test]
    fn test_addition() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let q = rng.gen_usable_composite_modulus();
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.crt_encode(x, q).unwrap();
                let y = d.crt_encode(y, q).unwrap();
                let z = d.crt_add(&x, &y).unwrap();
                out = d.crt_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, (x + y) % q);
        }
    }

    #[test]
    fn test_subtraction() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let q = rng.gen_usable_composite_modulus();
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.crt_encode(x, q).unwrap();
                let y = d.crt_encode(y, q).unwrap();
                let z = d.crt_sub(&x, &y).unwrap();
                out = d.crt_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, (x + q - y) % q);
        }
    }

    #[test]
    fn test_binary_cmul() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = 64;
            let q = 1 << nbits;
            let x = rng.gen_u128() % q;
            let c = 1 + rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let z = d.bin_cmul(&x, c, nbits).unwrap();
                out = d.bin_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, (x * c) % q);
        }
    }

    #[test]
    fn test_binary_multiplication() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = 64;
            let q = 1 << nbits;
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_encode(y, nbits).unwrap();
                let z = d.bin_multiplication_lower_half(&x, &y).unwrap();
                out = d.bin_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, (x * y) % q);
        }
    }

    #[test]
    fn test_shift_extend() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = 64;
            let q = 1 << nbits;
            let shift_size = rng.gen_usize() % nbits;
            let x = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                use crate::BinaryBundle;
                let x = d.bin_encode(x, nbits).unwrap();
                let z = d.shift_extend(&x, shift_size).unwrap();
                out = d.bin_output(&BinaryBundle::from(z)).unwrap().unwrap();
            }
            assert_eq!(out, x << shift_size);
        }
    }

    #[test]
    fn test_binary_full_multiplication() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = 64;
            let q = 1 << nbits;
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_encode(y, nbits).unwrap();
                let z = d.bin_mul(&x, &y).unwrap();
                println!("z.len() = {}", z.size());
                out = d.bin_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x * y);
        }
    }

    #[test]
    fn test_binary_division() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = 64;
            let q = 1 << nbits;
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_encode(y, nbits).unwrap();
                let z = d.bin_div(&x, &y).unwrap();
                out = d.bin_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x / y);
        }
    }

    #[test]
    fn max() {
        let mut rng = thread_rng();
        let q = util::modulus_with_width(10);
        let n = 10;
        for _ in 0..NITERS {
            let inps = (0..n).map(|_| rng.gen_u128() % (q / 2)).collect_vec();
            let should_be = *inps.iter().max().unwrap();
            let mut d = Dummy::new();
            let out;
            {
                let xs = inps
                    .into_iter()
                    .map(|x| d.crt_encode(x, q).unwrap())
                    .collect_vec();
                let z = d.crt_max(&xs, "100%").unwrap();
                out = d.crt_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, should_be);
        }
    }

    #[test]
    fn twos_complement() {
        let mut rng = thread_rng();
        let nbits = 16;
        let q = 1 << nbits;
        for _ in 0..NITERS {
            let x = rng.gen_u128() % q;
            let should_be = (((!x) % q) + 1) % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_twos_complement(&x).unwrap();
                out = d.bin_output(&y).unwrap().unwrap();
            }
            assert_eq!(out, should_be, "x={} y={} should_be={}", x, out, should_be);
        }
    }

    #[test]
    fn binary_addition() {
        let mut rng = thread_rng();
        let nbits = 16;
        let q = 1 << nbits;
        for _ in 0..NITERS {
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let should_be = (x + y) % q;
            let mut d = Dummy::new();
            let out;
            let overflow;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_encode(y, nbits).unwrap();
                let (z, _overflow) = d.bin_addition(&x, &y).unwrap();
                overflow = d.output(&_overflow).unwrap().unwrap();
                out = d.bin_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, should_be);
            assert_eq!(overflow > 0, x + y >= q);
        }
    }

    #[test]
    fn binary_subtraction() {
        let mut rng = thread_rng();
        let nbits = 16;
        let q = 1 << nbits;
        for _ in 0..NITERS {
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let (should_be, _) = x.overflowing_sub(y);
            let should_be = should_be % q;
            let mut d = Dummy::new();
            let overflow;
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_encode(y, nbits).unwrap();
                let (z, _overflow) = d.bin_subtraction(&x, &y).unwrap();
                overflow = d.output(&_overflow).unwrap().unwrap();
                out = d.bin_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, should_be);
            assert_eq!(overflow > 0, (y != 0 && x >= y), "x={} y={}", x, y);
        }
    }

    #[test]
    fn binary_lt() {
        let mut rng = thread_rng();
        let nbits = 16;
        let q = 1 << nbits;
        for _ in 0..NITERS {
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let should_be = x < y;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_encode(y, nbits).unwrap();
                let z = d.bin_lt(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out > 0, should_be, "x={} y={}", x, y);
        }
    }

    #[test]
    fn binary_max() {
        let mut rng = thread_rng();
        let n = 10;
        let nbits = 16;
        let q = 1 << nbits;
        for _ in 0..NITERS {
            let inps = (0..n).map(|_| rng.gen_u128() % q).collect_vec();
            let should_be = *inps.iter().max().unwrap();
            let mut d = Dummy::new();
            let out;
            {
                let xs = inps
                    .into_iter()
                    .map(|x| d.bin_encode(x, nbits).unwrap())
                    .collect_vec();
                let z = d.bin_max(&xs).unwrap();
                out = d.bin_output(&z).unwrap().unwrap();
            }
            assert_eq!(out, should_be);
        }
    }

    #[test] // bundle relu
    fn test_relu() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let q = crate::util::modulus_with_nprimes(4 + rng.gen_usize() % 7); // exact relu supports up to 11 primes
            let x = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.crt_encode(x, q).unwrap();
                let z = d.crt_relu(&x, "100%", None).unwrap();
                out = d.crt_output(&z).unwrap().unwrap();
            }
            if x >= q / 2 {
                assert_eq!(out, 0);
            } else {
                assert_eq!(out, x);
            }
        }
    }

    #[test]
    fn test_mask() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let q = crate::util::modulus_with_nprimes(4 + rng.gen_usize() % 7);
            let x = rng.gen_u128() % q;
            let b = rng.gen_bool();
            let mut d = Dummy::new();
            let out;
            {
                let b = d.encode(b as u16, &Modulus::Zq { q: (2) }).unwrap();
                let x = d.crt_encode(x, q).unwrap();
                let z = d.mask(&b, &x).unwrap().into();
                out = d.crt_output(&z).unwrap().unwrap();
            }
            assert!(
                if b { out == x } else { out == 0 },
                "b={} x={} z={}",
                b,
                x,
                out
            );
        }
    }

    #[test]
    fn binary_abs() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = 64;
            let q = 1 << nbits;
            let x = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let z = d.bin_abs(&x).unwrap();
                out = d.bin_output(&z).unwrap().unwrap();
            }
            let should_be = if x >> (nbits - 1) > 0 {
                ((!x) + 1) & ((1 << nbits) - 1)
            } else {
                x
            };
            assert_eq!(out, should_be);
        }
    }

    #[test]
    fn binary_demux() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = 8;
            let q = 1 << nbits;
            let x = rng.gen_u128() % q;
            let mut d = Dummy::new();
            let outs;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let zs = d.bin_demux(&x).unwrap();
                outs = d.outputs(&zs).unwrap().unwrap();
            }
            for (i, z) in outs.into_iter().enumerate() {
                if i as u128 == x {
                    assert_eq!(z, 1);
                } else {
                    assert_eq!(z, 0);
                }
            }
        }
    }

    #[test]
    fn binary_eq() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nbits = rng.gen_usize() % 100 + 2;
            let q = 1 << nbits;
            let x = rng.gen_u128() % q;
            let y = if rng.gen_bool() {
                x
            } else {
                rng.gen_u128() % q
            };
            let mut d = Dummy::new();
            let out;
            {
                let x = d.bin_encode(x, nbits).unwrap();
                let y = d.bin_encode(y, nbits).unwrap();
                let z = d.eq_bundles(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, (x == y) as u16);
        }
    }

    #[test]
    fn test_mixed_radix_addition_msb_only() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let nargs = 2 + rng.gen_usize() % 10;
            let mods = (0..7).map(|_| rng.gen_modulus()).collect_vec();
            let Q: u128 = util::product(&mods);

            println!("nargs={} mods={:?} Q={}", nargs, mods, Q);

            // test maximum overflow
            let xs = (0..nargs)
                .map(|_| {
                    Bundle::new(
                        util::as_mixed_radix(Q - 1, &mods)
                            .into_iter()
                            .zip(&mods)
                            .map(|(x, q)| DummyVal::new(x, &Modulus::Zq { q: *q }))
                            .collect_vec(),
                    )
                })
                .collect_vec();

            let mut d = Dummy::new();

            let z = d.mixed_radix_addition_msb_only(&xs).unwrap();
            let res = d.output(&z).unwrap().unwrap();

            let should_be = *util::as_mixed_radix((Q - 1) * (nargs as u128) % Q, &mods)
                .last()
                .unwrap();
            assert_eq!(res, should_be);

            // test random values
            for _ in 0..4 {
                let mut sum = 0;

                let xs = (0..nargs)
                    .map(|_| {
                        let x = rng.gen_u128() % Q;
                        sum = (sum + x) % Q;
                        Bundle::new(
                            util::as_mixed_radix(x, &mods)
                                .into_iter()
                                .zip(&mods)
                                .map(|(x, q)| DummyVal::new(x, &Modulus::Zq { q: *q }))
                                .collect_vec(),
                        )
                    })
                    .collect_vec();

                let mut d = Dummy::new();
                let z = d.mixed_radix_addition_msb_only(&xs).unwrap();
                let res = d.output(&z).unwrap().unwrap();

                let should_be = *util::as_mixed_radix(sum, &mods).last().unwrap();
                assert_eq!(res, should_be);
            }
        }
    }
}

#[cfg(test)]
mod pmr_tests {
    use super::*;
    use crate::{
        fancy::{BundleGadgets, CrtGadgets, FancyInput},
        util::RngExt,
    };

    #[test]
    fn pmr() {
        let mut rng = rand::thread_rng();
        for _ in 0..16 {
            let ps = rng.gen_usable_factors();
            let q = crate::util::product(&ps);
            let pt = rng.gen_u128() % q;

            let mut f = Dummy::new();
            let x = f.crt_encode(pt, q).unwrap();
            let z = f.crt_to_pmr(&x).unwrap();
            let res = f.output_bundle(&z).unwrap().unwrap();

            let should_be = to_pmr_pt(pt, &ps);
            assert_eq!(res, should_be);
        }
    }

    fn to_pmr_pt(x: u128, ps: &[u16]) -> Vec<u16> {
        let mut ds = vec![0; ps.len()];
        let mut q = 1;
        for i in 0..ps.len() {
            let p = ps[i] as u128;
            ds[i] = ((x / q) % p) as u16;
            q *= p;
        }
        ds
    }

    #[test]
    fn pmr_lt() {
        let mut rng = rand::thread_rng();
        for _ in 0..8 {
            let qs = rng.gen_usable_factors();
            let n = qs.len();
            let q = crate::util::product(&qs);
            let q_ = crate::util::product(&qs[..n - 1]);
            let pt_x = rng.gen_u128() % q_;
            let pt_y = rng.gen_u128() % q_;

            let mut f = Dummy::new();
            let crt_x = f.crt_encode(pt_x, q).unwrap();
            let crt_y = f.crt_encode(pt_y, q).unwrap();
            let z = f.pmr_lt(&crt_x, &crt_y).unwrap();
            let res = f.output(&z).unwrap().unwrap();

            let should_be = if pt_x < pt_y { 1 } else { 0 };
            assert_eq!(res, should_be, "q={}, x={}, y={}", q, pt_x, pt_y);
        }
    }

    #[test]
    fn pmr_geq() {
        let mut rng = rand::thread_rng();
        for _ in 0..8 {
            let qs = rng.gen_usable_factors();
            let n = qs.len();
            let q = crate::util::product(&qs);
            let q_ = crate::util::product(&qs[..n - 1]);
            let pt_x = rng.gen_u128() % q_;
            let pt_y = rng.gen_u128() % q_;

            let mut f = Dummy::new();
            let crt_x = f.crt_encode(pt_x, q).unwrap();
            let crt_y = f.crt_encode(pt_y, q).unwrap();
            let z = f.pmr_geq(&crt_x, &crt_y).unwrap();
            let res = f.output(&z).unwrap().unwrap();

            let should_be = if pt_x >= pt_y { 1 } else { 0 };
            assert_eq!(res, should_be, "q={}, x={}, y={}", q, pt_x, pt_y);
        }
    }

    #[test]
    #[ignore]
    fn crt_div() {
        let mut rng = rand::thread_rng();
        for _ in 0..8 {
            let qs = rng.gen_usable_factors();
            let n = qs.len();
            let q = crate::util::product(&qs);
            let q_ = crate::util::product(&qs[..n - 1]);
            let pt_x = rng.gen_u128() % q_;
            let pt_y = rng.gen_u128() % q_;

            let mut f = Dummy::new();
            let crt_x = f.crt_encode(pt_x, q).unwrap();
            let crt_y = f.crt_encode(pt_y, q).unwrap();
            let z = f.crt_div(&crt_x, &crt_y).unwrap();
            let res = f.crt_output(&z).unwrap().unwrap();

            let should_be = pt_x / pt_y;
            assert_eq!(res, should_be, "q={}, x={}, y={}", q, pt_x, pt_y);
        }
    }
}

#[cfg(test)]
mod GF4_dummy {
    use super::*;
    use itertools::Itertools;
    use rand::{thread_rng, seq::SliceRandom, Rng};

    const NITERS: usize = 1 << 10;

    #[test]
    fn test_addition() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let p = Modulus::GF4 { p: *vec!(19, 21, 31).choose(&mut rng).unwrap() as u8 };
            let x = (rng.gen::<u8>()&(15)) as u16;
            let y = (rng.gen::<u8>()&(15)) as u16;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let y = d.encode(y, &p).unwrap();
                let z = d.add(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x ^ y);
        }
    }

    #[test]
    fn test_subtraction() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let p = Modulus::GF4 { p: *vec!(19, 21, 31).choose(&mut rng).unwrap() as u8 };
            let x = (rng.gen::<u8>()&(15)) as u16;
            let y = (rng.gen::<u8>()&(15)) as u16;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let y = d.encode(y, &p).unwrap();
                let z = d.sub(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x ^ y);
        }
    }

    #[test]
    fn test_cmul() {
        // let mut rng = thread_rng();
        for _ in 0..NITERS { // iterate over possibilities?
            let p = Modulus::GF4 { p: 19 };
            let x = 2_u16.pow(3)+1;
            let c = 2_u16.pow(3)+2_u16.pow(2)+2+1;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let z = d.cmul(&x, c).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, 2_u16.pow(3)+2_u16.pow(2)+2);
        }
    }

    #[test]
        fn test_proj() {
            let mut rng = thread_rng();
            for _ in 0..NITERS {
                let p = Modulus::GF4 { p: *vec!(19, 21, 31).choose(&mut rng).unwrap() as u8 };
        
                let x = (rng.gen::<u8>()&(15)) as u16;
                let tab = (0..p.size()).map(|i| (i*9 + 1) % p.size()).collect_vec();
                let mut d = Dummy::new();
                let out;
                
                {
                    let x = d.encode(x, &p).unwrap();
                    let z = d.proj(&x, &p, Some(tab)).unwrap();
                    out = d.output(&z).unwrap().unwrap();
                }
                assert_eq!(out, (x*9 + 1) % p.size());
            
            }
        }
        //}}}
}

#[cfg(test)]
mod GF8_dummy {
    use super::*;
    use itertools::Itertools;
    use rand::{thread_rng, seq::SliceRandom, Rng};

    const NITERS: usize = 1 << 10;
    const IRRED_GF8: [u16; 30] = [0b100011011, 0b100011101, 0b100101011, 0b100101101, 0b100111001, 
        0b100111111, 0b101001101, 0b101011111, 0b101100011, 0b101100101,
        0b101101001, 0b101110001, 0b101110111, 0b101111011, 0b110000111,
        0b110001011, 0b110001101, 0b110011111, 0b110100011, 0b110101011,
        0b110110001, 0b110111101, 0b111000011, 0b111001111, 0b111010111,
        0b111011101, 0b111100111, 0b111110011, 0b111110101, 0b111111001];

    #[test]
    fn test_addition() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let p = Modulus::GF8 { p: *IRRED_GF8.choose(&mut rng).unwrap() };
            let x = (rng.gen::<u8>()) as u16;
            let y = (rng.gen::<u8>()) as u16;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let y = d.encode(y, &p).unwrap();
                let z = d.add(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x ^ y);
        }
    }

    #[test]
    fn test_subtraction() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let p = Modulus::GF8 { p: *IRRED_GF8.choose(&mut rng).unwrap() };
            let x = (rng.gen::<u8>()) as u16;
            let y = (rng.gen::<u8>()) as u16;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let y = d.encode(y, &p).unwrap();
                let z = d.sub(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x ^ y);
        }
    }

    #[test]
    fn test_cmul() {
        // let mut rng = thread_rng();
        for _ in 0..NITERS { // iterate over possibilities?
            let p = Modulus::GF8 { p: 283 };
            let x = 2_u16.pow(7) + 2_u16.pow(3) +1;
            let c = 2_u16.pow(6) + 2_u16.pow(5) + 2_u16.pow(2) + 2 + 1;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let z = d.cmul(&x, c).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, 2_u16.pow(5)+2_u16.pow(4)+2_u16.pow(3)+1);
        }
    }

    #[test]
        fn test_proj() {
            let mut rng = thread_rng();
            for _ in 0..NITERS {
                let p = Modulus::GF8 { p: *IRRED_GF8.choose(&mut rng).unwrap()};
        
                let x = (rng.gen::<u8>()) as u16;
                let tab = (0..p.size()).map(|i| (i*9 + 1) % p.size()).collect_vec();
                let mut d = Dummy::new();
                let out;
                
                {
                    let x = d.encode(x, &p).unwrap();
                    let z = d.proj(&x, &p, Some(tab)).unwrap();
                    out = d.output(&z).unwrap().unwrap();
                }
                assert_eq!(out, (x*9 + 1) % p.size());
            
            }
        }
        //}}}
}

#[cfg(test)]
mod GFk_dummy {
    use super::*;
    use itertools::Itertools;
    use rand::{thread_rng, seq::SliceRandom, Rng};

    const NITERS: usize = 1 << 10;
    const IRRED_GFk: [(u16, u8); 11] = [(0b1101, 3), (0b1011, 3),
    (0b100101, 5), (0b110111, 5), (0b111011, 5), 
    (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
    (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)]; 

    #[test]
    fn test_addition() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let poly = *IRRED_GFk.choose(&mut rng).unwrap();
            let p = Modulus::GFk { p: poly.0, k: poly.1 };
            let x = rng.gen::<u8>() as u16;
            let y = rng.gen::<u8>() as u16;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let y = d.encode(y, &p).unwrap();
                let z = d.add(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x ^ y);
        }
    }

    #[test]
    fn test_subtraction() {
        let mut rng = thread_rng();
        for _ in 0..NITERS {
            let poly = *IRRED_GFk.choose(&mut rng).unwrap();
            let p = Modulus::GFk { p: poly.0, k: poly.1 };
            let x = (rng.gen::<u8>()) as u16;
            let y = (rng.gen::<u8>()) as u16;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let y = d.encode(y, &p).unwrap();
                let z = d.sub(&x, &y).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, x ^ y);
        }
    }

    #[test]
    fn test_cmul() {
        // let mut rng = thread_rng();
        for _ in 0..NITERS { // iterate over possibilities?
            let p = Modulus::GFk { p: 283, k: 8 };
            let x = 2_u16.pow(7) + 2_u16.pow(3) +1;
            let c = 2_u16.pow(6) + 2_u16.pow(5) + 2_u16.pow(2) + 2 + 1;
            let mut d = Dummy::new();
            let out;
            {
                let x = d.encode(x, &p).unwrap();
                let z = d.cmul(&x, c).unwrap();
                out = d.output(&z).unwrap().unwrap();
            }
            assert_eq!(out, 2_u16.pow(5)+2_u16.pow(4)+2_u16.pow(3)+1);
        }
    }

    #[test]
        fn test_proj() {
            let mut rng = thread_rng();
            for _ in 0..NITERS {
                let poly = *IRRED_GFk.choose(&mut rng).unwrap();
                let p = Modulus::GFk { p: poly.0, k: poly.1 };
                let x = (rng.gen::<u8>() % p.size() as u8) as u16;
                let tab = (0..p.size()).map(|i| (i*9 + 1) % p.size()).collect_vec();
                let mut d = Dummy::new();
                let out;
                
                {
                    let x = d.encode(x, &p).unwrap();
                    let z = d.proj(&x, &p, Some(tab)).unwrap();
                    out = d.output(&z).unwrap().unwrap();
                }
                assert_eq!(out, (x*9 + 1) % p.size());
            
            }
        }
        //}}}
}
