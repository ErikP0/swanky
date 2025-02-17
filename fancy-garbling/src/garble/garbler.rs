// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

use crate::{
    errors::{FancyError, GarblerError},
    fancy::{BinaryBundle, CrtBundle, Fancy, FancyReveal, HasModulus},
    util::{output_tweak, tweak, tweak2, RngExt},
    wire::{Wire, Modulus},
};
use rand::{CryptoRng, RngCore};
use scuttlebutt::{AbstractChannel, Block};
use std::collections::HashMap;

/// Streams garbled circuit ciphertexts through a callback.
pub struct Garbler<C, RNG> {
    channel: C,
    deltas: HashMap<Modulus, Wire>, // map from modulus to associated delta wire-label.
    current_output: usize,
    current_gate: usize,
    rng: RNG,
}

impl<C: AbstractChannel, RNG: CryptoRng + RngCore> Garbler<C, RNG> {
    /// Create a new garbler.
    pub fn new(channel: C, rng: RNG) -> Self {
        Garbler {
            channel,
            deltas: HashMap::new(),
            current_gate: 0,
            current_output: 0,
            rng,
        }
    }

    #[cfg(feature = "serde1")]
    /// Load pre-chosen deltas from a file
    pub fn load_deltas(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let f = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(f);
        let deltas: HashMap<Modulus, Wire> = serde_json::from_reader(reader)?;
        self.deltas.extend(deltas.into_iter());
        Ok(())
    }

    /// The current non-free gate index of the garbling computation
    fn current_gate(&mut self) -> usize {
        let current = self.current_gate;
        self.current_gate += 1;
        current
    }

    /// Create a delta if it has not been created yet for this modulus, otherwise just
    /// return the existing one.
    pub fn delta(&mut self, modulus: &Modulus) -> Wire {
        if let Some(delta) = self.deltas.get(modulus) {
            return delta.clone();
        }
        let w = Wire::rand_delta(&mut self.rng, modulus);
        self.deltas.insert(*modulus, w.clone());
        w
    }

    /// The current output index of the garbling computation.
    fn current_output(&mut self) -> usize {
        let current = self.current_output;
        self.current_output += 1;
        current
    }

    /// Get the deltas, consuming the Garbler.
    ///
    /// This is useful for reusing wires in multiple garbled circuit instances.
    pub fn get_deltas(self) -> HashMap<Modulus, Wire> {
        self.deltas
    }

    /// Send a wire over the established channel.
    pub fn send_wire(&mut self, wire: &Wire) -> Result<(), GarblerError> {
        self.channel.write_block(&wire.as_block())?;
        Ok(())
    }

    /// Encode a wire, producing the zero wire as well as the encoded value.
    pub fn encode_wire(&mut self, val: u16, modulus: &Modulus) -> (Wire, Wire) {
        let zero = Wire::rand(&mut self.rng, modulus);
        let delta = self.delta(modulus);
        let enc = zero.plus(&delta.cmul(val));
        (zero, enc)
    }

    /// Encode many wires, producing zero wires as well as encoded values.
    pub fn encode_many_wires(
        &mut self,
        vals: &[u16],
        moduli: &[Modulus],
    ) -> Result<(Vec<Wire>, Vec<Wire>), GarblerError> {
        if vals.len() != moduli.len() {
            return Err(GarblerError::EncodingError);
        }
        assert!(vals.len() == moduli.len());
        let mut gbs = Vec::with_capacity(vals.len());
        let mut evs = Vec::with_capacity(vals.len());
        for (x, q) in vals.iter().zip(moduli.iter()) {
            let (gb, ev) = self.encode_wire(*x, q);
            gbs.push(gb);
            evs.push(ev);
        }
        Ok((gbs, evs))
    }

    /// Encode a `CrtBundle`, producing zero wires as well as encoded values.
    pub fn crt_encode_wire(
        &mut self,
        val: u128,
        modulus: u128,
    ) -> Result<(CrtBundle<Wire>, CrtBundle<Wire>), GarblerError> {
        let ms = crate::util::factor(modulus);
        let msM = ms.iter().map(|q| Modulus::Zq { q: *q }).collect::<Vec<_>>();
        let xs = crate::util::crt(val, &ms);
        let (gbs, evs) = self.encode_many_wires(&xs, &msM)?;
        Ok((CrtBundle::new(gbs), CrtBundle::new(evs)))
    }

    /// Encode a `BinaryBundle`, producing zero wires as well as encoded values.
    pub fn bin_encode_wire(
        &mut self,
        val: u128,
        nbits: usize,
    ) -> Result<(BinaryBundle<Wire>, BinaryBundle<Wire>), GarblerError> {
        let xs = crate::util::u128_to_bits(val, nbits);
        let ms = vec![Modulus::Zq{ q: 2 }; nbits];
        let (gbs, evs) = self.encode_many_wires(&xs, &ms)?;
        Ok((BinaryBundle::new(gbs), BinaryBundle::new(evs)))
    }
}

impl<C: AbstractChannel, RNG: RngCore + CryptoRng> FancyReveal for Garbler<C, RNG> {
    fn reveal(&mut self, x: &Wire) -> Result<u16, GarblerError> {
        // The evaluator needs our cooperation in order to see the output.
        // Hence, we call output() ourselves.
        self.output(x)?;
        self.channel.flush()?;
        let val = self.channel.read_u16()?;
        Ok(val)
    }
}

impl<C: AbstractChannel, RNG: RngCore + CryptoRng> Fancy for Garbler<C, RNG> {
    type Item = Wire;
    type Error = GarblerError;

    fn constant(&mut self, x: u16, q: &Modulus) -> Result<Wire, GarblerError> {
        let zero = Wire::rand(&mut self.rng, q);
        let wire = zero.plus(&self.delta(q).cmul_eq(x));
        self.send_wire(&wire)?;
        Ok(zero)
    }

    fn add(&mut self, x: &Wire, y: &Wire) -> Result<Wire, GarblerError> {
        if x.modulus() != y.modulus() {
            return Err(GarblerError::FancyError(FancyError::UnequalModuli));
        }
        Ok(x.plus(y))
    }

    fn sub(&mut self, x: &Wire, y: &Wire) -> Result<Wire, GarblerError> {
        if x.modulus() != y.modulus() {
            return Err(GarblerError::FancyError(FancyError::UnequalModuli));
        }
        Ok(x.minus(y))
    }

    fn cmul(&mut self, x: &Wire, c: u16) -> Result<Wire, GarblerError> {
        Ok(x.cmul(c))
    }

    fn mul(&mut self, A: &Wire, B: &Wire) -> Result<Wire, GarblerError> {
        match (A.modulus(), B.modulus()) {
            (Modulus::Zq { q }, Modulus::Zq { q: qb }) => {
                if q < qb {
                    return self.mul(B, A);
                }

                let gate_num = self.current_gate();

                let modA = A.modulus();

                let D = self.delta(&modA);
                let Db = self.delta(&B.modulus());

                let r;
                let mut gate = vec![Block::default(); q as usize + qb as usize - 2];

                // hack for unequal moduli
                if q != qb {
                    // would need to pack minitable into more than one u128 to support qb > 8
                    if qb > 8 {
                        return Err(GarblerError::AsymmetricHalfGateModuliMax8(qb));
                    }

                    r = self.rng.gen_u16() % q;
                    let t = tweak2(gate_num as u64, 1);

                    let mut minitable = vec![u128::default(); qb as usize];
                    let mut B_ = B.clone();
                    for b in 0..qb {
                        if b > 0 {
                            B_.plus_eq(&Db);
                        }
                        let new_color = ((r + b) % q) as u128;
                        let ct = (u128::from(B_.hash(t)) & 0xFFFF) ^ new_color;
                        minitable[B_.color() as usize] = ct;
                    }

                    let mut packed = 0;
                    for i in 0..qb as usize {
                        packed += minitable[i] << (16 * i);
                    }
                    gate.push(Block::from(packed));
                } else {
                    r = B.color(); // secret value known only to the garbler (ev knows r+b)
                }

                let g = tweak2(gate_num as u64, 0);

                // X = H(A+aD) + arD such that a + A.color == 0
                let alpha = (q - A.color()) % q; // alpha = -A.color
                let X = A
                    .plus(&D.cmul(alpha))
                    .hashback(g, &modA)
                    .plus_mov(&D.cmul(alpha * r % q));

                // Y = H(B + bD) + (b + r)A such that b + B.color == 0
                let beta = (qb - B.color()) % qb;
                let Y = B
                    .plus(&Db.cmul(beta))
                    .hashback(g, &modA)
                    .plus_mov(&A.cmul((beta + r) % q));

                let mut precomp = Vec::with_capacity(q as usize);

                // precompute a lookup table of X.minus(&D_cmul[(a * r % q)])
                //                            = X.plus(&D_cmul[((q - (a * r % q)) % q)])
                let mut X_ = X.clone();
                precomp.push(X_.as_block());
                for _ in 1..q {
                    X_.plus_eq(&D);
                    precomp.push(X_.as_block());
                }

                let mut A_ = A.clone();
                for a in 0..q {
                    if a > 0 {
                        A_.plus_eq(&D);
                    }
                    // garbler's half-gate: outputs X-arD
                    // G = H(A+aD) ^ X+a(-r)D = H(A+aD) ^ X-arD
                    if A_.color() != 0 {
                        gate[A_.color() as usize - 1] =
                            A_.hash(g) ^ precomp[((q - (a * r % q)) % q) as usize];
                    }
                }

                precomp.clear();

                // precompute a lookup table of Y.minus(&A_cmul[((b+r) % q)])
                //                            = Y.plus(&A_cmul[((q - ((b+r) % q)) % q)])
                let mut Y_ = Y.clone();
                precomp.push(Y_.as_block());
                for _ in 1..q {
                    Y_.plus_eq(&A);
                    precomp.push(Y_.as_block());
                }

                let mut B_ = B.clone();
                for b in 0..qb {
                    if b > 0 {
                        B_.plus_eq(&Db);
                    }
                    // evaluator's half-gate: outputs Y-(b+r)D
                    // G = H(B+bD) + Y-(b+r)A
                    if B_.color() != 0 {
                        gate[q as usize - 1 + B_.color() as usize - 1] =
                            B_.hash(g) ^ precomp[((q - ((b + r) % q)) % q) as usize];
                    }
                }

                for block in gate.iter() {
                    self.channel.write_block(block)?;
                }
                Ok(X.plus_mov(&Y))
            }
            // (Modulus::GF4 { p }, Modulus::GF4 { p: pb }) => {
            //     // TODO
            // }
            _ => {
                Err(GarblerError::FancyError(FancyError::InvalidArg(format!("Multiplication of {:?} and {:?} is not supported", A.modulus(), B.modulus()))))
            }
        }
    }

    fn proj(&mut self, A: &Wire, mod_out: &Modulus, tt: Option<Vec<u16>>) -> Result<Wire, GarblerError> {
        let tt = tt.ok_or(GarblerError::TruthTableRequired)?;

        let mod_in = A.modulus();
        // let q_in:u16; let q_out:u16;
        let Din: Wire; let Dout: Wire;

        let mut gate = vec![Block::default(); mod_in.size() as usize - 1];

        let tao = A.color();
        let g = tweak(self.current_gate());
        let C: Wire;

        Din = self.delta(&mod_in);
        Dout = self.delta(&mod_out);

        // output zero-wire
        // W_g^0 <- -H(g, W_{a_1}^0 - \tao\Delta_m) - \phi(-\tao)\Delta_n
        let neg_tao= match mod_in {
            Modulus::Zq {q: q_i} =>  (q_i - tao) % q_i, // rotate negative -tao over mod q_in
            Modulus::GF4 {..} | Modulus::GF8 {..} | Modulus::GFk {..} => tao, // in GF(2^k): -tao = tao
        };
        let neg_phi_neg_tao = match mod_out {
            Modulus::Zq {q: q_o} =>  (q_o - tt[neg_tao as usize]) % q_o,
            Modulus::GF4 {..} | Modulus::GF8 {..} | Modulus::GFk {..} => tt[(tao) as usize],
        };
        C = A
            .plus(&Din.cmul(neg_tao))
            .hashback(g, mod_out)
            .plus_mov(&Dout.cmul(neg_phi_neg_tao));

        // precompute `let C_ = C.plus(&Dout.cmul(tt[x as usize]))`
        // TODO: this might compute labels that are not used in the truth table at all!
        let C_precomputed = match mod_out {
            Modulus::Zq { q: q_i } => {
                let mut C_ = C.clone();
                (0..*q_i).map(|x| {
                    if x > 0 {
                        C_.plus_eq(&Dout);
                    }
                    C_.as_block()
                }).collect::<Vec<Block>>()
            }
            Modulus::GF4 { .. } | Modulus::GF8 { .. } | Modulus::GFk { .. } => {
                let mut C_ = C.clone();
                (0..mod_out.size()).map(|x| {
                    if x > 0 {
                        C_ = C.clone();
                        C_.plus_eq(&Dout.cmul(x));
                    }
                    C_.as_block()
                }).collect::<Vec<Block>>()
            }
        };

        let mut A_ = A.clone();
        match mod_in {
            Modulus::Zq {q: q_i} => {
                for x in 0..q_i {
                    if x > 0 {
                        A_.plus_eq(&Din); // avoiding expensive cmul for `A_ = A.plus(&Din.cmul(x))`
                    }

                    let ix = (tao as usize + x as usize) % q_i as usize;
                    if ix == 0 {
                        continue;
                    }

                    let ct = A_.hash(g) ^ C_precomputed[tt[x as usize] as usize];
                    gate[ix - 1] = ct;
                }
            },
            Modulus::GF4 {..} | Modulus::GF8 {..} | Modulus::GFk {..} => {
                for x in 0..mod_in.size() {
                    if x > 0 {
                        A_ = A.clone();
                        A_.plus_eq(&Din.cmul(x));
                    }

                    let ix = (tao ^ x) as usize;
                    if ix == 0 {
                        continue;
                    }

                    let ct = A_.hash(g) ^ C_precomputed[tt[x as usize] as usize];
                    gate[ix - 1] = ct;
                }
            }
        }

        for block in gate.iter() {
            self.channel.write_block(block)?;
        }
        Ok(C)
    }

    fn output(&mut self, X: &Wire) -> Result<Option<u16>, GarblerError> {
        let modulus = X.modulus();
        let q = modulus.size();
        let i = self.current_output();
        let D = self.delta(&modulus);

        for k in 0..q {
            let block = X.plus(&D.cmul(k)).hash(output_tweak(i, k));
            self.channel.write_block(&block)?;
        }
        Ok(None)
    }
}
