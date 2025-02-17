// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! DSL for creating circuits compatible with fancy-garbling in the old-fashioned way,
//! where you create a circuit for a computation then garble it.

use crate::{
    dummy::{Dummy, DummyVal},
    errors::{CircuitBuilderError, DummyError, FancyError},
    fancy::{BinaryBundle, CrtBundle, Fancy, FancyInput, HasModulus},
    wire::Modulus
};
use itertools::Itertools;
use std::collections::HashMap;

/// The index and modulus of a gate in a circuit.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct CircuitRef {
    pub(crate) ix: usize,
    pub(crate) modulus: Modulus,
}

impl std::fmt::Display for CircuitRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{} | {}]", self.ix, self.modulus)
    }
}

impl HasModulus for CircuitRef {
    fn modulus(&self) -> Modulus {
        self.modulus
    }
}

/// Static representation of the type of computation supported by fancy garbling.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct Circuit {
    pub(crate) gates: Vec<Gate>,
    pub(crate) gate_moduli: Vec<Modulus>,
    pub(crate) garbler_input_refs: Vec<CircuitRef>,
    pub(crate) evaluator_input_refs: Vec<CircuitRef>,
    pub(crate) const_refs: Vec<CircuitRef>,
    pub(crate) output_refs: Vec<CircuitRef>,
    pub(crate) num_nonfree_gates: usize,
}

/// The most basic types of computation supported by fancy garbling.
///
/// `id` represents the gate number. `out` gives the output wire index; if `out
/// = None`, then we use the gate index as the output wire index.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum Gate {
    GarblerInput {
        id: usize,
    },
    EvaluatorInput {
        id: usize,
    },
    Constant {
        val: u16,
    },
    Add {
        xref: CircuitRef,
        yref: CircuitRef,
        out: Option<usize>,
    },
    Sub {
        xref: CircuitRef,
        yref: CircuitRef,
        out: Option<usize>,
    },
    Cmul {
        xref: CircuitRef,
        c: u16,
        out: Option<usize>,
    },
    Mul {
        xref: CircuitRef,
        yref: CircuitRef,
        id: usize,
        out: Option<usize>,
    },
    Proj {
        xref: CircuitRef,
        tt: Vec<u16>,
        id: usize,
        out: Option<usize>,
    },
}

impl std::fmt::Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Gate::GarblerInput { id } => write!(f, "GarblerInput {}", id),
            Gate::EvaluatorInput { id } => write!(f, "EvaluatorInput {}", id),
            Gate::Constant { val } => write!(f, "Constant {}", val),
            Gate::Add { xref, yref, out } => write!(f, "Add ( {}, {}, {:?} )", xref, yref, out),
            Gate::Sub { xref, yref, out } => write!(f, "Sub ( {}, {}, {:?} )", xref, yref, out),
            Gate::Cmul { xref, c, out } => write!(f, "Cmul ( {}, {}, {:?} )", xref, c, out),
            Gate::Mul {
                xref,
                yref,
                id,
                out,
            } => write!(f, "Mul ( {}, {}, {}, {:?} )", xref, yref, id, out),
            Gate::Proj { xref, tt, id, out } => {
                write!(f, "Proj ( {}, {:?}, {}, {:?} )", xref, tt, id, out)
            }
        }
    }
}

impl Circuit {
    /// Make a new `Circuit` object.
    pub fn new(ngates: Option<usize>) -> Circuit {
        let gates = Vec::with_capacity(ngates.unwrap_or(0));
        Circuit {
            gates,
            garbler_input_refs: Vec::new(),
            evaluator_input_refs: Vec::new(),
            const_refs: Vec::new(),
            output_refs: Vec::new(),
            gate_moduli: Vec::new(),
            num_nonfree_gates: 0,
        }
    }

    /// Evaluate the circuit using fancy object `f`.
    pub fn eval<F: Fancy>(
        &self,
        f: &mut F,
        garbler_inputs: &[F::Item],
        evaluator_inputs: &[F::Item],
    ) -> Result<Option<Vec<u16>>, F::Error> {
        let mut cache: Vec<Option<F::Item>> = vec![None; self.gates.len()];
        for (i, gate) in self.gates.iter().enumerate() {
            let q = self.modulus(i);
            let (zref_, val) = match *gate {
                Gate::GarblerInput { id } => (None, garbler_inputs[id].clone()),
                Gate::EvaluatorInput { id } => {
                    assert!(
                        id < evaluator_inputs.len(),
                        "id={} ev_inps.len()={}",
                        id,
                        evaluator_inputs.len()
                    );
                    (None, evaluator_inputs[id].clone())
                }
                Gate::Constant { val } => (None, f.constant(val, &q)?),
                Gate::Add { xref, yref, out } => (
                    out,
                    f.add(
                        cache[xref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                        cache[yref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    )?,
                ),
                Gate::Sub { xref, yref, out } => (
                    out,
                    f.sub(
                        cache[xref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                        cache[yref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    )?,
                ),
                Gate::Cmul { xref, c, out } => (
                    out,
                    f.cmul(
                        cache[xref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                        c,
                    )?,
                ),
                Gate::Proj {
                    xref, ref tt, out, ..
                } => (
                    out,
                    f.proj(
                        cache[xref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                        &q,
                        Some(tt.to_vec()),
                    )?,
                ),
                Gate::Mul {
                    xref, yref, out, ..
                } => (
                    out,
                    f.mul(
                        cache[xref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                        cache[yref.ix]
                            .as_ref()
                            .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    )?,
                ),
            };
            cache[zref_.unwrap_or(i)] = Some(val);
        }

        let mut outputs = Vec::with_capacity(self.output_refs.len());
        for r in self.output_refs.iter() {
            let r = cache[r.ix]
                .as_ref()
                .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?;
            let out = f.output(r)?;
            outputs.push(out);
        }
        Ok(outputs.into_iter().collect())
    }

    /// Evaluate the circuit in plaintext.
    pub fn eval_plain(
        &self,
        garbler_inputs: &[u16],
        evaluator_inputs: &[u16],
    ) -> Result<Vec<u16>, DummyError> {
        let mut dummy = crate::dummy::Dummy::new();

        if garbler_inputs.len() != self.garbler_input_refs.len() {
            return Err(DummyError::NotEnoughGarblerInputs);
        }

        if evaluator_inputs.len() != self.evaluator_input_refs.len() {
            return Err(DummyError::NotEnoughEvaluatorInputs);
        }

        // encode inputs as DummyVals
        let gb = garbler_inputs
            .iter()
            .zip(self.garbler_input_refs.iter())
            .map(|(x, r)| DummyVal::new(*x, r.modulus()))
            .collect_vec();
        let ev = evaluator_inputs
            .iter()
            .zip(self.evaluator_input_refs.iter())
            .map(|(x, r)| DummyVal::new(*x, r.modulus()))
            .collect_vec();

        let outputs = self.eval(&mut dummy, &gb, &ev)?;
        Ok(outputs.expect("dummy will always return Some(u16) output"))
    }

    /// Print circuit info.
    pub fn print_info(&self) -> Result<(), DummyError> {
        let mut informer = crate::informer::Informer::new(Dummy::new());

        // encode inputs as InformerVals
        let gb = self
            .garbler_input_refs
            .iter()
            .map(|r| informer.receive(&r.modulus()))
            .collect::<Result<Vec<DummyVal>, DummyError>>()?;
        let ev = self
            .evaluator_input_refs
            .iter()
            .map(|r| informer.receive(&r.modulus()))
            .collect::<Result<Vec<DummyVal>, DummyError>>()?;

        let _outputs = self.eval(&mut informer, &gb, &ev)?;
        println!("{}", informer.stats());
        Ok(())
    }

    /// Return the number of garbler inputs.
    #[inline]
    pub fn num_garbler_inputs(&self) -> usize {
        self.garbler_input_refs.len()
    }

    /// Return the number of evaluator inputs.
    #[inline]
    pub fn num_evaluator_inputs(&self) -> usize {
        self.evaluator_input_refs.len()
    }

    /// Return the number of outputs.
    #[inline]
    pub fn noutputs(&self) -> usize {
        self.output_refs.len()
    }

    /// Return the modulus of the gate indexed by `i`.
    #[inline]
    pub fn modulus(&self, i: usize) -> Modulus {
        self.gate_moduli[i]
    }

    /// Return the modulus of the garbler input indexed by `i`.
    #[inline]
    pub fn garbler_input_mod(&self, i: usize) -> Modulus {
        let r = self.garbler_input_refs[i];
        r.modulus()
    }

    /// Return the modulus of the evaluator input indexed by `i`.
    #[inline]
    pub fn evaluator_input_mod(&self, i: usize) -> Modulus {
        let r = self.evaluator_input_refs[i];
        r.modulus()
    }
}

/// CircuitBuilder is used to build circuits.
pub struct CircuitBuilder {
    next_ref_ix: usize,
    next_garbler_input_id: usize,
    next_evaluator_input_id: usize,
    const_map: HashMap<(u16, Modulus), CircuitRef>,
    circ: Circuit,
}

impl Fancy for CircuitBuilder {
    type Item = CircuitRef;
    type Error = CircuitBuilderError;

    fn constant(&mut self, val: u16, modulus: &Modulus) -> Result<CircuitRef, Self::Error> {
        match self.const_map.get(&(val, *modulus)) {
            Some(&r) => Ok(r),
            None => {
                let gate = Gate::Constant { val };
                let r = self.gate(gate, modulus);
                self.const_map.insert((val, *modulus), r);
                self.circ.const_refs.push(r);
                Ok(r)
            }
        }
    }

    fn add(&mut self, xref: &CircuitRef, yref: &CircuitRef) -> Result<CircuitRef, Self::Error> {
        if xref.modulus() != yref.modulus() {
            return Err(Self::Error::from(FancyError::UnequalModuli));
        }
        let gate = Gate::Add {
            xref: *xref,
            yref: *yref,
            out: None,
        };
        Ok(self.gate(gate, &xref.modulus()))
    }

    fn sub(&mut self, xref: &CircuitRef, yref: &CircuitRef) -> Result<CircuitRef, Self::Error> {
        if xref.modulus() != yref.modulus() {
            return Err(Self::Error::from(FancyError::UnequalModuli));
        }
        let gate = Gate::Sub {
            xref: *xref,
            yref: *yref,
            out: None,
        };
        Ok(self.gate(gate, &xref.modulus()))
    }

    fn cmul(&mut self, xref: &CircuitRef, c: u16) -> Result<CircuitRef, Self::Error> {
        Ok(self.gate(
            Gate::Cmul {
                xref: *xref,
                c,
                out: None,
            },
            &xref.modulus(),
        ))
    }

    fn proj(
        &mut self,
        xref: &CircuitRef,
        output_modulus: &Modulus,
        tt: Option<Vec<u16>>,
    ) -> Result<CircuitRef, Self::Error> {
        let tt = tt.ok_or_else(|| Self::Error::from(FancyError::NoTruthTable))?;
        if tt.len() < xref.modulus().size() as usize  || !tt.iter().all(|&x| x < output_modulus.size()) {
            return Err(Self::Error::from(FancyError::InvalidTruthTable));
        }
        let gate = Gate::Proj {
            xref: *xref,
            tt: tt.to_vec(),
            id: self.get_next_ciphertext_id(),
            out: None,
        };
        Ok(self.gate(gate, output_modulus))
    }

    fn mul(&mut self, xref: &CircuitRef, yref: &CircuitRef) -> Result<CircuitRef, Self::Error> {
        match (xref.modulus(), yref.modulus()) {
            (Modulus::Zq { q: xmod }, Modulus::Zq { q: ymod }) => {
                if xmod < ymod {
                    return self.mul(yref, xref);
                }
        
                let gate = Gate::Mul {
                    xref: *xref,
                    yref: *yref,
                    id: self.get_next_ciphertext_id(),
                    out: None,
                };

                Ok(self.gate(gate, &xref.modulus()))
            },
            // (Modulus::GF4 { p: xpoly }, Modulus::GF4 { p: ypoly }) => {
            //     // TODO
            // },
            _ => {
                Err(Self::Error::from(FancyError::InvalidArg(String::from("Not supported for combining a field and ring element."))))
            },
        }
    }

    fn output(&mut self, xref: &CircuitRef) -> Result<Option<u16>, Self::Error> {
        self.circ.output_refs.push(*xref);
        Ok(None)
    }
}

impl CircuitBuilder {
    /// Make a new `CircuitBuilder`.
    pub fn new() -> Self {
        CircuitBuilder {
            next_ref_ix: 0,
            next_garbler_input_id: 0,
            next_evaluator_input_id: 0,
            const_map: HashMap::new(),
            circ: Circuit::new(None),
        }
    }

    /// Finish circuit building, outputting the resulting circuit.
    pub fn finish(self) -> Circuit {
        self.circ
    }

    fn get_next_garbler_input_id(&mut self) -> usize {
        let current = self.next_garbler_input_id;
        self.next_garbler_input_id += 1;
        current
    }

    fn get_next_evaluator_input_id(&mut self) -> usize {
        let current = self.next_evaluator_input_id;
        self.next_evaluator_input_id += 1;
        current
    }

    fn get_next_ciphertext_id(&mut self) -> usize {
        let current = self.circ.num_nonfree_gates;
        self.circ.num_nonfree_gates += 1;
        current
    }

    fn get_next_ref_ix(&mut self) -> usize {
        let current = self.next_ref_ix;
        self.next_ref_ix += 1;
        current
    }

    fn gate(&mut self, gate: Gate, modulus: &Modulus) -> CircuitRef {
        self.circ.gates.push(gate);
        self.circ.gate_moduli.push(*modulus);
        let ix = self.get_next_ref_ix();
        CircuitRef { ix, modulus: *modulus }
    }

    /// Get CircuitRef for a garbler input wire.
    pub fn garbler_input(&mut self, modulus: &Modulus) -> CircuitRef {
        let id = self.get_next_garbler_input_id();
        let r = self.gate(Gate::GarblerInput { id }, modulus);
        self.circ.garbler_input_refs.push(r);
        r
    }

    /// Get CircuitRef for an evaluator input wire.
    pub fn evaluator_input(&mut self, modulus: &Modulus) -> CircuitRef {
        let id = self.get_next_evaluator_input_id();
        let r = self.gate(Gate::EvaluatorInput { id }, modulus);
        self.circ.evaluator_input_refs.push(r);
        r
    }

    /// Get a vec of CircuitRefs for garbler inputs.
    pub fn garbler_inputs(&mut self, mods: &[Modulus]) -> Vec<CircuitRef> {
        mods.iter().map(|q| self.garbler_input(q)).collect()
    }

    /// Get a vec of CircuitRefs for garbler inputs.
    pub fn evaluator_inputs(&mut self, mods: &[Modulus]) -> Vec<CircuitRef> {
        mods.iter().map(|q| self.evaluator_input(q)).collect()
    }

    /// Get a CrtBundle for the garbler using composite modulus Q
    pub fn crt_garbler_input(&mut self, modulus: u128) -> CrtBundle<CircuitRef> {
        CrtBundle::new(self.garbler_inputs(&crate::util::factor(modulus).into_iter().map(|q| Modulus::Zq { q }).collect::<Vec<_>>()))
    }

    /// Get a CrtBundle for the evaluator using composite modulus Q
    pub fn crt_evaluator_input(&mut self, modulus: u128) -> CrtBundle<CircuitRef> {
        CrtBundle::new(self.evaluator_inputs(&crate::util::factor(modulus).into_iter().map(|q| Modulus::Zq { q }).collect::<Vec<_>>()))
    }

    /// Get a BinaryBundle for the garbler with n bits.
    pub fn bin_garbler_input(&mut self, nbits: usize) -> BinaryBundle<CircuitRef> {
        BinaryBundle::new(self.garbler_inputs(&vec![Modulus::Zq { q:2 }; nbits]))
    }

    /// Get a BinaryBundle for the evaluator with n bits.
    pub fn bin_evaluator_input(&mut self, nbits: usize) -> BinaryBundle<CircuitRef> {
        BinaryBundle::new(self.evaluator_inputs(&vec![Modulus::Zq { q:2 }; nbits]))
    }
}

#[cfg(test)]
mod plaintext {
    use super::*;
    use crate::util::RngExt;
    use itertools::Itertools;
    use rand::{thread_rng, seq::SliceRandom, Rng};

    #[test] // and_gate_fan_n
    fn and_gate_fan_n() {
        let mut rng = thread_rng();

        let mut b = CircuitBuilder::new();
        let n = 2 + (rng.gen_usize() % 200);
        let inps = b.evaluator_inputs(&vec![Modulus::Zq { q:2 }; n]);
        let z = b.and_many(&inps).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let mut inps: Vec<u16> = Vec::new();
            for _ in 0..n {
                inps.push(RngExt::gen_bool(&mut rng) as u16);
            }
            let res = inps.iter().fold(1, |acc, &x| x & acc);
            let out = c.eval_plain(&[], &inps).unwrap()[0];
            if !(out == res) {
                println!("{:?} {} {}", inps, out, res);
                panic!("incorrect output n={}", n);
            }
        }
    }
    
    #[test] //  or_gate_fan_n
    fn or_gate_fan_n() {
        let mut rng = thread_rng();
        let mut b = CircuitBuilder::new();
        let n = 2 + (rng.gen_usize() % 200);
        let inps = b.evaluator_inputs(&vec![Modulus::Zq { q:2 }; n]);
        let z = b.or_many(&inps).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let mut inps: Vec<u16> = Vec::new();
            for _ in 0..n {
                inps.push(RngExt::gen_bool(&mut rng) as u16);
            }
            let res = inps.iter().fold(0, |acc, &x| x | acc);
            let out = c.eval_plain(&[], &inps).unwrap()[0];
            if !(out == res) {
                println!("{:?} {} {}", inps, out, res);
                panic!();
            }
        }
    }
    
    #[test] //  half_gate
    fn half_gate() {
        let mut rng = thread_rng();
        let mut b = CircuitBuilder::new();
        let q = rng.gen_prime();
        let x = b.garbler_input(&Modulus::Zq { q });
        let y = b.evaluator_input(&Modulus::Zq { q });
        let z = b.mul(&x, &y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();
        for _ in 0..16 {
            let x = rng.gen_u16() % q;
            let y = rng.gen_u16() % q;
            let out = c.eval_plain(&[x], &[y]).unwrap();
            assert_eq!(out[0], x * y % q);
        }
    }
    
    #[test] // mod_change 
    fn mod_change() {
        let mut rng = thread_rng();
        let mut b = CircuitBuilder::new();
        let p = rng.gen_prime();
        let q = rng.gen_prime();
        let x = b.garbler_input(&Modulus::Zq { q: p });
        let y = b.mod_change(&x, q).unwrap();
        let z = b.mod_change(&y, p).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();
        for _ in 0..16 {
            let x = rng.gen_u16() % p;
            let out = c.eval_plain(&[x], &[]).unwrap();
            assert_eq!(out[0], x % q);
        }
    }
    
    #[test] // add_many_mod_change 
    fn add_many_mod_change() {
        let mut b = CircuitBuilder::new();
        let n = 113;
        let args = b.garbler_inputs(&vec![Modulus::Zq { q: 2 }; n]);
        let wires = args
            .iter()
            .map(|x| b.mod_change(x, n as u16 + 1).unwrap())
            .collect_vec();
        let s = b.add_many(&wires).unwrap();
        b.output(&s).unwrap();
        let c = b.finish();

        let mut rng = thread_rng();
        for _ in 0..64 {
            let inps = (0..c.num_garbler_inputs())
                .map(|i| rng.gen_u16() % c.garbler_input_mod(i).size())
                .collect_vec();
            let s: u16 = inps.iter().sum();
            println!("{:?}, sum={}", inps, s);
            let out = c.eval_plain(&inps, &[]).unwrap();
            assert_eq!(out[0], s);
        }
    }
    
    #[test] // constants
    fn constants_Zq() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();

        let q = rng.gen_modulus();
        let c = rng.gen_u16() % q;

        let x = b.evaluator_input(&Modulus::Zq { q });
        let y = b.constant(c, &Modulus::Zq { q }).unwrap();
        let z = b.add(&x, &y).unwrap();
        b.output(&z).unwrap();

        let circ = b.finish();

        for _ in 0..64 {
            let x = rng.gen_u16() % q;
            let z = circ.eval_plain(&[], &[x]).unwrap();
            assert_eq!(z[0], (x + c) % q);
        }
    }
    

    #[test]
    fn constants_GF4() {
        let mut b = CircuitBuilder::new();
        let mut rng = thread_rng();
    
        let modulus = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
        let c = (rng.gen::<u8>()&(15)) as u16;
    
        let x = b.evaluator_input(modulus);
        let y = b.constant(c as u16, modulus).unwrap();
        let z = b.add(&x, &y).unwrap();
        b.output(&z).unwrap();
    
        let circ = b.finish();
    
        for _ in 0..64 {
            let x = (rng.gen::<u8>()&(15)) as u16;
            let z = circ.eval_plain(&[], &[x as u16]).unwrap();
            assert_eq!(z[0], (x ^ c) as u16);
        }
    }
}

#[cfg(test)]
mod bundle {
    use super::*;
    use crate::{
        fancy::{BinaryGadgets, BundleGadgets, CrtGadgets},
        util::{self, crt_factor, crt_inv_factor, RngExt},
    };
    use itertools::Itertools;
    use rand::thread_rng;

    #[test] // bundle input and output
    fn test_bundle_input_output() {
        let mut rng = thread_rng();
        let q = rng.gen_usable_composite_modulus();

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        println!("{:?} wires", x.wires().len());
        b.output_bundle(&x).unwrap();
        let c = b.finish();

        println!("{:?}", c.output_refs);

        for _ in 0..16 {
            let x = rng.gen_u128() % q;
            let res = c.eval_plain(&crt_factor(x, q), &[]).unwrap();
            println!("{:?}", res);
            let z = crt_inv_factor(&res, q);
            assert_eq!(x, z);
        }
    }

    //
    #[test] // bundle addition 
    fn test_addition() {
        let mut rng = thread_rng();
        let q = rng.gen_usable_composite_modulus();

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let y = b.crt_evaluator_input(q);
        let z = b.crt_add(&x, &y).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let res = c.eval_plain(&crt_factor(x, q), &crt_factor(y, q)).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, (x + y) % q);
        }
    }
    
    #[test] // bundle subtraction 
    fn test_subtraction() {
        let mut rng = thread_rng();
        let q = rng.gen_usable_composite_modulus();

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let y = b.crt_evaluator_input(q);
        let z = b.sub_bundles(&x, &y).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let res = c.eval_plain(&crt_factor(x, q), &crt_factor(y, q)).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, (x + q - y) % q);
        }
    }
    
    #[test] // bundle cmul
    fn test_cmul() {
        let mut rng = thread_rng();
        let q = util::modulus_with_width(16);

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let y = rng.gen_u128() % q;
        let z = b.crt_cmul(&x, y).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen_u128() % q;
            let res = c.eval_plain(&crt_factor(x, q), &[]).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, (x * y) % q);
        }
    }
    
    #[test] // bundle multiplication 
    fn test_multiplication() {
        let mut rng = thread_rng();
        let q = rng.gen_usable_composite_modulus();

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let y = b.crt_evaluator_input(q);
        let z = b.mul_bundles(&x, &y).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen_u64() as u128 % q;
            let y = rng.gen_u64() as u128 % q;
            let res = c.eval_plain(&crt_factor(x, q), &crt_factor(y, q)).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, (x * y) % q);
        }
    }
    
    #[test] // bundle cexp 
    fn test_cexp() {
        let mut rng = thread_rng();
        let q = util::modulus_with_width(10);
        let y = rng.gen_u16() % 10;

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let z = b.crt_cexp(&x, y).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..64 {
            let x = rng.gen_u16() as u128 % q;
            let should_be = x.pow(y as u32) % q;
            let res = c.eval_plain(&crt_factor(x, q), &[]).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, should_be);
        }
    }
     
    #[test] // bundle remainder 
    fn test_remainder() {
        let mut rng = thread_rng();
        let ps = rng.gen_usable_factors();
        let q = ps.iter().fold(1, |acc, &x| (x as u128) * acc);
        let p = ps[rng.gen_u16() as usize % ps.len()];

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let z = b.crt_rem(&x, &Modulus::Zq { q:p }).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..64 {
            let x = rng.gen_u128() % q;
            let should_be = x % p as u128;
            let res = c.eval_plain(&crt_factor(x, q), &[]).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, should_be);
        }
    }
    
    #[test] // bundle equality
    fn test_equality() {
        let mut rng = thread_rng();
        let q = rng.gen_usable_composite_modulus();

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let y = b.crt_evaluator_input(q);
        let z = b.eq_bundles(&x, &y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        // lets have at least one test where they are surely equal
        let x = rng.gen_u128() % q;
        let res = c.eval_plain(&crt_factor(x, q), &crt_factor(x, q)).unwrap();
        assert_eq!(res, &[(x == x) as u16]);

        for _ in 0..64 {
            let x = rng.gen_u128() % q;
            let y = rng.gen_u128() % q;
            let res = c.eval_plain(&crt_factor(x, q), &crt_factor(y, q)).unwrap();
            assert_eq!(res, &[(x == y) as u16]);
        }
    }
    
    #[test] // bundle mixed_radix_addition 
    fn test_mixed_radix_addition() {
        let mut rng = thread_rng();

        let nargs = 2 + rng.gen_usize() % 100;
        let mods = (0..7).map(|_| rng.gen_modulus()).collect_vec();

        let mut b = CircuitBuilder::new();
        let xs = (0..nargs)
            .map(|_| crate::fancy::Bundle::new(b.evaluator_inputs(&mods.iter().map(|q| Modulus::Zq { q: *q }).collect::<Vec<_>>())))
            .collect_vec();
        let z = b.mixed_radix_addition(&xs).unwrap();
        b.output_bundle(&z).unwrap();
        let circ = b.finish();

        let Q: u128 = mods.iter().map(|&q| q as u128).product();

        // test maximum overflow
        let mut ds = Vec::new();
        for _ in 0..nargs {
            ds.extend(util::as_mixed_radix(Q - 1, &mods).iter());
        }
        let res = circ.eval_plain(&[], &ds).unwrap();
        assert_eq!(
            util::from_mixed_radix(&res, &mods),
            (Q - 1) * (nargs as u128) % Q
        );

        // test random values
        for _ in 0..4 {
            let mut should_be = 0;
            let mut ds = Vec::new();
            for _ in 0..nargs {
                let x = rng.gen_u128() % Q;
                should_be = (should_be + x) % Q;
                ds.extend(util::as_mixed_radix(x, &mods).iter());
            }
            let res = circ.eval_plain(&[], &ds).unwrap();
            assert_eq!(util::from_mixed_radix(&res, &mods), should_be);
        }
    }
    
    #[test] // bundle relu 
    fn test_relu() {
        let mut rng = thread_rng();
        let q = util::modulus_with_width(10);
        println!("q={}", q);

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let z = b.crt_relu(&x, "100%", None).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..128 {
            let pt = rng.gen_u128() % q;
            let should_be = if pt < q / 2 { pt } else { 0 };
            let res = c.eval_plain(&crt_factor(pt, q), &[]).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, should_be);
        }
    }
    
    #[test] // bundle sgn 
    fn test_sgn() {
        let mut rng = thread_rng();
        let q = util::modulus_with_width(10);
        println!("q={}", q);

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let z = b.crt_sgn(&x, "100%", None).unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..128 {
            let pt = rng.gen_u128() % q;
            let should_be = if pt < q / 2 { 1 } else { q - 1 };
            let res = c.eval_plain(&crt_factor(pt, q), &[]).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, should_be);
        }
    }
    
    #[test] // bundle leq 
    fn test_leq() {
        let mut rng = thread_rng();
        let q = util::modulus_with_width(10);

        let mut b = CircuitBuilder::new();
        let x = b.crt_garbler_input(q);
        let y = b.crt_evaluator_input(q);
        let z = b.crt_lt(&x, &y, "100%").unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        // lets have at least one test where they are surely equal
        let x = rng.gen_u128() % q / 2;
        let res = c.eval_plain(&crt_factor(x, q), &crt_factor(x, q)).unwrap();
        assert_eq!(res, &[(x < x) as u16], "x={}", x);

        for _ in 0..64 {
            let x = rng.gen_u128() % q / 2;
            let y = rng.gen_u128() % q / 2;
            let res = c.eval_plain(&crt_factor(x, q), &crt_factor(y, q)).unwrap();
            assert_eq!(res, &[(x < y) as u16], "x={} y={}", x, y);
        }
    }
    
    #[test] // bundle max 
    fn test_max() {
        let mut rng = thread_rng();
        let q = util::modulus_with_width(10);
        let n = 10;
        println!("n={} q={}", n, q);

        let mut b = CircuitBuilder::new();
        let xs = (0..n).map(|_| b.crt_garbler_input(q)).collect_vec();
        let z = b.crt_max(&xs, "100%").unwrap();
        b.output_bundle(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let inps = (0..n).map(|_| rng.gen_u128() % (q / 2)).collect_vec();
            println!("{:?}", inps);
            let should_be = *inps.iter().max().unwrap();

            let enc_inps = inps
                .into_iter()
                .flat_map(|x| crt_factor(x, q))
                .collect_vec();
            let res = c.eval_plain(&enc_inps, &[]).unwrap();
            let z = crt_inv_factor(&res, q);
            assert_eq!(z, should_be);
        }
    }
    
    #[test] // binary addition 
    fn test_binary_addition() {
        let mut rng = thread_rng();
        let n = 2 + (rng.gen_usize() % 10);
        let q = 2;
        let Q = util::product(&vec![q; n]);
        println!("n={} q={} Q={}", n, q, Q);

        let mut b = CircuitBuilder::new();
        let x = b.bin_garbler_input(n);
        let y = b.bin_evaluator_input(n);
        let (zs, carry) = b.bin_addition(&x, &y).unwrap();
        b.output(&carry).unwrap();
        b.output_bundle(&zs).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen_u128() % Q;
            let y = rng.gen_u128() % Q;
            println!("x={} y={}", x, y);
            let res_should_be = (x + y) % Q;
            let carry_should_be = (x + y >= Q) as u16;
            let res = c
                .eval_plain(&util::u128_to_bits(x, n), &util::u128_to_bits(y, n))
                .unwrap();
            assert_eq!(util::u128_from_bits(&res[1..]), res_should_be);
            assert_eq!(res[0], carry_should_be);
        }
    }
    
    #[test] // binary demux 
    fn test_bin_demux() {
        let mut rng = thread_rng();
        let nbits = 1 + (rng.gen_usize() % 7);
        let Q = 1 << nbits as u128;

        let mut b = CircuitBuilder::new();
        let x = b.bin_garbler_input(nbits);
        let d = b.bin_demux(&x).unwrap();
        b.outputs(&d).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen_u128() % Q;
            println!("x={}", x);
            let mut should_be = vec![0; Q as usize];
            should_be[x as usize] = 1;

            let res = c.eval_plain(&util::u128_to_bits(x, nbits), &[]).unwrap();

            for (i, y) in res.into_iter().enumerate() {
                if i as u128 == x {
                    assert_eq!(y, 1);
                } else {
                    assert_eq!(y, 0);
                }
            }
        }
    }
    
}


#[cfg(test)]
mod GF4 {
    use super::*;

    #[test] // GF4 input and output
    fn test_GF4_input_output() {
        for modulus in &Modulus::GF4_MODULI {
            let mut b = CircuitBuilder::new();
            let x = b.garbler_input(modulus);
            b.output(&x).unwrap();
            let c = b.finish();

            for x in 0..16 {
                let res = c.eval_plain(&[x], &[]).unwrap();
                assert_eq!(x, res[0]);
            }
        }
    }

    #[test] // GF4 addition
    fn test_GF4_addition() {
        for modulus in &Modulus::GF4_MODULI {
            let mut b = CircuitBuilder::new();
            let x = b.garbler_input(modulus);
            let y = b.evaluator_input(modulus);
            let z = b.add(&x, &y).unwrap();
            b.output(&z).unwrap();
            let c = b.finish();

            for x in 0..16 {
                for y in 0..16 {
                    let res = c.eval_plain(&[x], &[y]).unwrap();
                    assert_eq!(res[0], x ^ y);
                }
            }
        }
    }

    #[test] // GF4 subtraction
    fn test_GF4_subtraction() {
        for modulus in &Modulus::GF4_MODULI {
            let mut b = CircuitBuilder::new();
            let x = b.garbler_input(modulus);
            let y = b.evaluator_input(modulus);
            let z = b.sub(&x, &y).unwrap();
            b.output(&z).unwrap();
            let c = b.finish();

            for x in 0..16 {
                for y in 0..16 {
                    let res = c.eval_plain(&[x], &[y]).unwrap();
                    assert_eq!(res[0], x ^ y);
                }
            }
        }
    }

    #[test] // bundle cmul
    fn test_GF4_cmul() {
        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(&Modulus::X4_X_1);
        // y = X^3 + X^2 + X + 1
        let y = 2_u16.pow(3) + 2_u16.pow(2) + 2 + 1;
        let z = b.cmul(&x, y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        // x = X^3 + 1
        let x = &[2_u16.pow(3) + 1];
        let res = c.eval_plain(x, &[]).unwrap();
        // x * y mod X^4 + X +1 = X^3 + X^2 + X
        assert_eq!(res[0], 2_u16.pow(3) + 2_u16.pow(2) + 2);
    }

    #[test] // GF4 proj
    fn test_GF4_proj() {
        for modulus in &Modulus::GF4_MODULI {
            let mut b = CircuitBuilder::new();
            let x = b.garbler_input(modulus);
            let tab = (0..modulus.size()).map(|i| (i * 9 + 1) % modulus.size()).collect_vec();
            let z = b.proj(&x, modulus, Some(tab.clone())).unwrap();
            assert_eq!(z.modulus(), *modulus);
            b.output(&z).unwrap();
            let c = b.finish();

            for x in 0..modulus.size() {
                let res = c.eval_plain(&[x], &[]).unwrap();
                assert_eq!(res, vec![tab[x as usize]]);
            }
        }
    }
}

#[cfg(test)]
mod GF8 {
    use rand::{thread_rng, Rng};
    use super::*;   
    use rand::seq::SliceRandom;

    #[test] // GF8 input and output
    fn test_GF8_input_output() {
        let mut rng = thread_rng();
        let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(p);
        b.output(&x).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen::<u8>();
            let res = c.eval_plain(&[x as u16], &[]).unwrap();
            assert_eq!(x as u16, res[0]);
        }
    }


    #[test] // GF8 addition
    fn test_GF8_addition() {
        let mut rng = thread_rng();
        let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(p);
        let y = b.evaluator_input(p);
        let z = b.add(&x, &y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen::<u8>();
            let y = rng.gen::<u8>();
            let res = c.eval_plain(&[x as u16], &[y as u16]).unwrap();
            assert_eq!(res[0], (x ^ y) as u16);
        }
    }

    #[test] // GF8 subtraction
    fn test_GF8_subtraction() {
        let mut rng = thread_rng();
        let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(p);
        let y = b.evaluator_input(p);
        let z = b.sub(&x, &y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = (rng.gen::<u8>()) as u16;
            let y = (rng.gen::<u8>()) as u16;
            let res = c.eval_plain(&[x], &[y]).unwrap();
            assert_eq!(res[0], (x ^ y));
        }
    }

    #[test]
    fn test_GF8_cmul() {
        let p = Modulus::GF8 { p: 283 };

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(&p);
        let y = 2_u16.pow(6) + 2_u16.pow(4) + 2;
        let z = b.cmul(&x, y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();


        let x = &[2_u16.pow(7) + 2_u16.pow(6) + 2_u16.pow(5) + 2 + 1];
        let res = c.eval_plain(x, &[]).unwrap();
        assert_eq!(res[0], 2_u16.pow(7) + 2_u16.pow(4) + 1);
    }


    #[test] // GF8 proj
    fn test_GF8_proj() {
        let mut rng = thread_rng();
        let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(p);
        let tab = (0..p.size()).map(|i| (i * 9 + 1) % p.size()).collect_vec();
        let z = b.proj(&x, &p, Some(tab)).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = &[(rng.gen::<u8>()) as u16];
            let res = c.eval_plain(x, &[]).unwrap();
            assert_eq!(res[0], (x[0] * 9 + 1) % p.size());
        }
    }
}

#[cfg(test)]
mod GFk {
    use rand::{thread_rng, Rng};
    use super::*;   
    use rand::seq::SliceRandom;

    const IRRED_GF_K: [(u16, u8); 11] = [
        (0b1101, 3), (0b1011, 3),
        (0b100101, 5), (0b110111, 5), (0b111011, 5),
        (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
        (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)
    ];

    #[test] // GFk input and output 
    fn test_GFk_input_output() {
        let mut rng = thread_rng();
        let poly = *IRRED_GF_K.choose(&mut rng).unwrap();
        let p = Modulus::GFk { p: poly.0, k: poly.1 };

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(&Modulus::GFk { p: poly.0, k: poly.1 });
        b.output(&x).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen::<u8>() % p.size() as u8;
            let res = c.eval_plain(&[x as u16], &[]).unwrap();
            assert_eq!(x as u16, res[0]);
        }
    }


    #[test] // GFk addition
    fn test_GFk_addition() {
        let mut rng = thread_rng();
        let poly = *IRRED_GF_K.choose(&mut rng).unwrap();
        let p = Modulus::GFk { p: poly.0, k: poly.1 };

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(&p);
        let y = b.evaluator_input(&p);
        let z = b.add(&x, &y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = rng.gen::<u8>() % p.size() as u8;
            let y = rng.gen::<u8>() % p.size() as u8;
            let res = c.eval_plain(&[x as u16], &[y as u16]).unwrap();
            assert_eq!(res[0], (x ^ y) as u16);
        }
    }

    #[test] // GFk subtraction
    fn test_GFk_subtraction() {
        let mut rng = thread_rng();
        let poly = *IRRED_GF_K.choose(&mut rng).unwrap();
        let p = Modulus::GFk { p: poly.0, k: poly.1 };

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(&p);
        let y = b.evaluator_input(&p);
        let z = b.sub(&x, &y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = (rng.gen::<u8>()) as u16 % p.size();
            let y = (rng.gen::<u8>()) as u16 % p.size();
            let res = c.eval_plain(&[x], &[y]).unwrap();
            assert_eq!(res[0], (x ^ y));
        }
    }

    #[test] // GFk cmul
    fn test_GFk_cmul() {
        let p = Modulus::GFk { p: 283, k: 8 };

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(&p);
        let y = 2_u16.pow(6) + 2_u16.pow(4) + 2;
        let z = b.cmul(&x, y).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        let x = &[2_u16.pow(7) + 2_u16.pow(6) + 2_u16.pow(5) + 2 + 1];
        let res = c.eval_plain(x, &[]).unwrap();
        assert_eq!(res[0], 2_u16.pow(7) + 2_u16.pow(4) + 1);
    }


    #[test] // GF8 proj
    fn test_GF8_proj() {
        let mut rng = thread_rng();
        let poly = *IRRED_GF_K.choose(&mut rng).unwrap();
        let p = Modulus::GFk { p: poly.0, k: poly.1 };

        let mut b = CircuitBuilder::new();
        let x = b.garbler_input(&p);
        let tab = (0..p.size()).map(|i| (i * 9 + 1) % p.size()).collect_vec();
        let z = b.proj(&x, &p, Some(tab)).unwrap();
        b.output(&z).unwrap();
        let c = b.finish();

        for _ in 0..16 {
            let x = &[(rng.gen::<u8>()) as u16 % p.size()];
            let res = c.eval_plain(x, &[]).unwrap();
            assert_eq!(res[0], (x[0] * 9 + 1) % p.size());
        }
    }
}