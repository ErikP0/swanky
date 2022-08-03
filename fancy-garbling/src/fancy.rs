// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! The `Fancy` trait represents the kinds of computations possible in `fancy-garbling`.
//!
//! An implementer must be able to create inputs, constants, do modular arithmetic, and
//! create projections.

use crate::errors::FancyError;
use itertools::Itertools;

mod binary;
mod bundle;
mod crt;
mod input;
mod pmr;
mod reveal;
pub use binary::{BinaryBundle, BinaryGadgets};
pub use bundle::{Bundle, BundleGadgets};
pub use crt::{CrtBundle, CrtGadgets};
pub use input::FancyInput;
pub use reveal::FancyReveal;
pub use crate::wire::Modulus;
/// An object that has some modulus. Basic object of `Fancy` computations.
pub trait HasModulus {
    /// The modulus of the wire.
    fn modulus(&self) -> Modulus;
}

/// DSL for the basic computations supported by `fancy-garbling`.
pub trait Fancy {
    /// The underlying wire datatype created by an object implementing `Fancy`.
    type Item: Clone + HasModulus;

    /// Errors which may be thrown by the users of Fancy.
    type Error: std::fmt::Debug + std::fmt::Display + std::convert::From<FancyError>;

    /// Create a constant `x` with modulus `q`.
    fn constant(&mut self, x: u16, q: &Modulus) -> Result<Self::Item, Self::Error>;

    /// Add `x` and `y`.
    fn add(&mut self, x: &Self::Item, y: &Self::Item) -> Result<Self::Item, Self::Error>;

    /// Subtract `x` and `y`.
    fn sub(&mut self, x: &Self::Item, y: &Self::Item) -> Result<Self::Item, Self::Error>;

    /// Multiply `x` times the constant `c`.
    fn cmul(&mut self, x: &Self::Item, c: u16) -> Result<Self::Item, Self::Error>;

    /// Multiply `x` and `y`.
    fn mul(&mut self, x: &Self::Item, y: &Self::Item) -> Result<Self::Item, Self::Error>;

    /// Project `x` according to the truth table `tt`. Resulting wire has modulus `q`.
    ///
    /// Optional `tt` is useful for hiding the gate from the evaluator.
    fn proj(
        &mut self,
        x: &Self::Item,
        q: &Modulus,
        tt: Option<Vec<u16>>,
    ) -> Result<Self::Item, Self::Error>;

    /// Process this wire as output. Some `Fancy` implementors dont actually *return*
    /// output, but they need to be involved in the process, so they can return `None`.
    fn output(&mut self, x: &Self::Item) -> Result<Option<u16>, Self::Error>;

    ////////////////////////////////////////////////////////////////////////////////
    // Functions built on top of basic fancy operations.

    /// Sum up a slice of wires.
    fn add_many(&mut self, args: &[Self::Item]) -> Result<Self::Item, Self::Error> {
        if args.len() < 2 {
            return Err(Self::Error::from(FancyError::InvalidArgNum {
                got: args.len(),
                needed: 2,
            }));
        }
        let mut z = args[0].clone();
        for x in args.iter().skip(1) {
            z = self.add(&z, x)?;
        }
        Ok(z)
    }

    /// Xor is just addition, with the requirement that `x` and `y` are mod 2.
    fn xor(&mut self, x: &Self::Item, y: &Self::Item) -> Result<Self::Item, Self::Error> {
        if  let Modulus::Zq{ q:2 } = x.modulus() {
            if  let Modulus::Zq{ q:2 } = y.modulus() {
                self.add(x, y)
            }
            else {
                return Err(Self::Error::from(FancyError::InvalidArgMod {
                    got: y.modulus(),
                    needed: Modulus::Zq { q: 2 },
                }));
            }
        } else {
            return Err(Self::Error::from(FancyError::InvalidArgMod {
                got: x.modulus(),
                needed: Modulus::Zq { q: 2 },
            }));
        }

    }

    /// Negate by xoring `x` with `1`.
    fn negate(&mut self, x: &Self::Item) -> Result<Self::Item, Self::Error> {
        if let Modulus::Zq { q:2 } = x.modulus() {
            let one = self.constant(1, &x.modulus())?;
            self.xor(x, &one)
        }
        else {
            return Err(Self::Error::from(FancyError::InvalidArgMod {
                got: x.modulus(),
                needed: Modulus::Zq { q: 2 },
            }));
        }
    }

    /// And is just multiplication, with the requirement that `x` and `y` are mod 2.
    fn and(&mut self, x: &Self::Item, y: &Self::Item) -> Result<Self::Item, Self::Error> {
        if  let Modulus::Zq{ q:2 } = x.modulus() {
            if  let Modulus::Zq{ q:2 } = y.modulus() {
                self.mul(x, y)
            }
            else {
                return Err(Self::Error::from(FancyError::InvalidArgMod {
                    got: y.modulus(),
                    needed: Modulus::Zq { q: 2 },
                }));
            }
        } else {
            return Err(Self::Error::from(FancyError::InvalidArgMod {
                got: x.modulus(),
                needed: Modulus::Zq { q: 2 },
            }));
        }
    }

    /// Or uses Demorgan's Rule implemented with multiplication and negation.
    fn or(&mut self, x: &Self::Item, y: &Self::Item) -> Result<Self::Item, Self::Error> {
        if  let Modulus::Zq{ q:2 } = x.modulus() {
            if  let Modulus::Zq{ q:2 } = y.modulus() {
                let notx = self.negate(x)?;
                let noty = self.negate(y)?;
                let z = self.and(&notx, &noty)?;
                self.negate(&z)
            }
            else {
                return Err(Self::Error::from(FancyError::InvalidArgMod {
                    got: y.modulus(),
                    needed: Modulus::Zq { q: 2 },
                }));
            }
        } else {
            return Err(Self::Error::from(FancyError::InvalidArgMod {
                got: x.modulus(),
                needed: Modulus::Zq { q: 2 },
            }));
        }
    }

    /// Returns 1 if all wires equal 1.
    fn and_many(&mut self, args: &[Self::Item]) -> Result<Self::Item, Self::Error> {
        if args.len() < 2 {
            return Err(Self::Error::from(FancyError::InvalidArgNum {
                got: args.len(),
                needed: 2,
            }));
        }
        args.iter()
            .skip(1)
            .fold(Ok(args[0].clone()), |acc, x| self.and(&(acc?), x))
    }

    /// Returns 1 if any wire equals 1.
    fn or_many(&mut self, args: &[Self::Item]) -> Result<Self::Item, Self::Error> {
        if args.len() < 2 {
            return Err(Self::Error::from(FancyError::InvalidArgNum {
                got: args.len(),
                needed: 2,
            }));
        }
        args.iter()
            .skip(1)
            .fold(Ok(args[0].clone()), |acc, x| self.or(&(acc?), x))
    }

    /// Change the modulus of `x` to `to_modulus` using a projection gate.
    fn mod_change(&mut self, x: &Self::Item, to_modulus: u16) -> Result<Self::Item, Self::Error> {
        let from_modulus = x.modulus();

        if let Modulus::Zq { q: qfrom } = from_modulus {
            if qfrom ==to_modulus {
                return Ok(x.clone());
            } else {
                let tab = (0..qfrom).map(|x| x % to_modulus).collect_vec();
                self.proj(x, &from_modulus, Some(tab))
            }
        } else {
            panic!("Wire x is not a Zq type");
        }
}
    /// Binary adder. Returns the result and the carry.
    fn adder(
        &mut self,
        x: &Self::Item,
        y: &Self::Item,
        carry_in: Option<&Self::Item>,
    ) -> Result<(Self::Item, Self::Item), Self::Error> {
        if  let Modulus::Zq{ q:2 } = x.modulus() {
            if  let Modulus::Zq{ q:2 } = y.modulus() {
                if let Some(c) = carry_in {
                    let z1 = self.xor(x, y)?;
                    let z2 = self.xor(&z1, c)?;
                    let z3 = self.xor(x, c)?;
                    let z4 = self.and(&z1, &z3)?;
                    let carry = self.xor(&z4, x)?;
                    Ok((z2, carry))
                } else {
                    let z = self.xor(x, y)?;
                    let carry = self.and(x, y)?;
                    Ok((z, carry))
                }
            }
            else {
                return Err(Self::Error::from(FancyError::InvalidArgMod {
                    got: y.modulus(),
                    needed: Modulus::Zq { q: 2 },
                }));
            }
        } else {
            return Err(Self::Error::from(FancyError::InvalidArgMod {
                got: x.modulus(),
                needed: Modulus::Zq { q: 2 },
            }));
        }
    }

    /// If `b = 0` returns `x` else `y`.
    ///
    /// `b` must be mod 2 but `x` and `y` can be have any modulus.
    fn mux(
        &mut self,
        b: &Self::Item,
        x: &Self::Item,
        y: &Self::Item,
    ) -> Result<Self::Item, Self::Error> {
        let notb = self.negate(b)?;
        let xsel = self.mul(&notb, x)?;
        let ysel = self.mul(b, y)?;
        self.add(&xsel, &ysel)
    }

    /// If `x = 0` returns the constant `b1` else return `b2`. Folds constants if possible.
    fn mux_constant_bits(
        &mut self,
        x: &Self::Item,
        b1: bool,
        b2: bool,
    ) -> Result<Self::Item, Self::Error> {
        if let Modulus::Zq{ q:2 } = x.modulus() {
            if !b1 && b2 {
                Ok(x.clone())
            } else if b1 && !b2 {
                self.negate(x)
            } else if !b1 && !b2 {
                self.constant(0, &x.modulus())
            } else {
                self.constant(1, &x.modulus())
            }
        } else {
            return Err(Self::Error::from(FancyError::InvalidArgMod {
                got: x.modulus(),
                needed: Modulus::Zq { q: 2 },
            }));
        }
    }

    /// Output a slice of wires.
    fn outputs(&mut self, xs: &[Self::Item]) -> Result<Option<Vec<u16>>, Self::Error> {
        let mut zs = Vec::with_capacity(xs.len());
        for x in xs.iter() {
            zs.push(self.output(x)?);
        }
        Ok(zs.into_iter().collect())
    }
}
