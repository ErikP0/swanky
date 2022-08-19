// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! Low-level operations on wire-labels, the basic building block of garbled circuits.

use crate::{fancy::HasModulus, util};
use rand::{CryptoRng, Rng, RngCore};
use scuttlebutt::{Block, AES_HASH};

mod npaths_tab;

/// The core wire-label type.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum Wire {
    /// Representation of a `mod-2` wire.
    Mod2 {
        /// A 128-bit value.
        val: Block,
    },
    /// Representation of a `mod-3` wire.
    ///
    /// We represent a `mod-3` wire by 64 `mod-3` elements. These elements are
    /// stored as follows: the least-significant bits of each element are stored
    /// in `lsb` and the most-significant bits of each element are stored in
    /// `msb`. This representation allows for efficient addition and
    /// multiplication as described here by the paper "Hardware Implementation
    /// of Finite Fields of Characteristic Three." D. Page, N.P. Smart. CHES
    /// 2002. Link:
    /// <https://link.springer.com/content/pdf/10.1007/3-540-36400-5_38.pdf>.
    Mod3 {
        /// The least-significant bits of each `mod-3` element.
        lsb: u64,
        /// The most-significant bits of each `mod-3` element.
        msb: u64,
    },
    /// Representation of a `mod-q` wire.
    ///
    /// We represent a `mod-q` wire for `q > 3` by the modulus `q` alongside a
    /// list of `mod-q` digits.
    ModN {
        /// The modulus of this wire-label.
        q: u16,
        /// A list of `mod-q` digits.
        ds: Vec<u16>,
    },
    /// Representation of a wire in GF(2^4)
    GF4 {
        /// Irreducible polynomial.
        p: u8,
        /// A list of GF(2^4) elements.
        elts: Vec<u16>,
    },
    /// Representation of a wire in GF(2^8)
    GF8 {
        /// Irreducible polynomial
        p: u16,
        /// A list of GF(2^4) elements
        elts: Vec<u16>,
    },
    /// Representation of a wire in GF(2^k)
    GFk {
        /// k
        k: u8,
        /// Irreducible polynomial.
        p: u16,
        /// A list of GF(2^4) elements.
        elts: Vec<u16>,
    },
}

/// Modulus type, either an integer modulus for Zq, or an irreducible polynomial representation for GF(2^k)
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
// #[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum Modulus {
    /// Integer modulus for Zq.
    Zq {
        q: u16,
    },
    /// Irreducible polynomial for GF(2^4).
    GF4 {
        p: u8,
    },
    /// Irreducible polynomial for GF(2^8).
    GF8 {
        p: u16,
    },
    /// Irreducible polynomial for GF(2^k).
    GFk {
        k: u8, 
        p: u16,
    },
}

impl std::fmt::Display for Modulus {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            Modulus::Zq { q } => write!(fmt, "modulus q = {} (Zq)", q),
            Modulus::GF4 { p } => write!(fmt, "irreducible polynomial p = {} (GF4)", p), 
            Modulus::GF8 { p } => write!(fmt, "irreducible polynomial p = {} (GF8)",p),
            Modulus::GFk { k, p } => write!(fmt, "irreducible polynomial p = {} (GF{})",p,k),
        }
    }
}

impl std::default::Default for Wire {
    fn default() -> Self {
        Wire::Mod2 {
            val: Block::default(),
        }
    }
}

impl Modulus {
    pub fn value(&self) -> u16 {
        match self {
            Modulus::Zq { q } => *q,
            Modulus::GF4 { p } => *p as u16 ,
            Modulus::GF8 { p } => *p,
            Modulus::GFk { k: _, p } => *p,
        }
    }

    pub fn size(&self) -> u16 {
        match self {
            Modulus::Zq { q } => *q,
            Modulus::GF4 { .. } => 16 ,
            Modulus::GF8 { .. } => 256,
            Modulus::GFk { k, .. } => 2_u16.pow(*k as u32),
        }
    }
}
impl HasModulus for Wire {
    fn modulus(&self) -> Modulus {
        match self {
            Wire::Mod2 { .. } => Modulus::Zq { q: 2 },
            Wire::Mod3 { .. } => Modulus::Zq { q: 3 },
            Wire::ModN { q, .. } => Modulus::Zq { q: *q },
            Wire::GF4 { p, .. } => Modulus::GF4 { p: *p },
            Wire::GF8 { p, .. } =>  Modulus::GF8 { p: *p },
            Wire::GFk { k, p, .. } => Modulus::GFk { k: *k, p: *p },
        }
    }
}

impl Wire {
    /// Get the digits of the wire.
    pub fn digits(&self) -> Vec<u16> {
        match self {
            Wire::Mod2 { val } => (0..128)
                .map(|i| ((u128::from(*val) >> i) as u16) & 1)
                .collect(),
            Wire::Mod3 { lsb, msb } => (0..64)
                .map(|i| (((lsb >> i) as u16) & 1) & ((((msb >> i) as u16) & 1) << 1))
                .collect(),
            Wire::ModN { ds, .. } => ds.clone(),
            Wire::GF4 { elts, .. } => elts.clone(),
            Wire::GF8 { elts, .. } => elts.clone(),
            Wire::GFk { elts, ..} => elts.clone(),
        }
    }

    fn _from_block_lookup(inp: Block, q: u16) -> Vec<u16> {
        debug_assert!(q < 256);
        debug_assert!(base_conversion::lookup_defined_for_mod(q));
        let bytes: [u8; 16] = inp.into();
        // The digits in position 15 will be the longest, so we can use stateful
        // (fast) base `q` addition.
        let mut ds = base_conversion::lookup_digits_mod_at_position(bytes[15], q, 15).to_vec();
        for i in 0..15 {
            let cs = base_conversion::lookup_digits_mod_at_position(bytes[i], q, i);
            util::base_q_add_eq(&mut ds, &cs, q);
        }
        // Drop the digits we won't be able to pack back in again, especially if
        // they get multiplied.
        ds.truncate(util::digits_per_u128(q));
        ds
    }

    fn _unrank(inp: u128, q: u16) -> Vec<u16> {
        let mut x = inp;
        let ndigits = util::digits_per_u128(q);
        let npaths_tab = npaths_tab::lookup(q);
        x %= npaths_tab[ndigits - 1] * q as u128;

        let mut ds = vec![0; ndigits];
        for i in (0..ndigits).rev() {
            let npaths = npaths_tab[i];

            if q <= 23 {
                // linear search
                let mut acc = 0;
                for j in 0..q {
                    acc += npaths;
                    if acc > x {
                        x -= acc - npaths;
                        ds[i] = j;
                        break;
                    }
                }
            } else {
                // naive division
                let d = x / npaths;
                ds[i] = d as u16;
                x -= d * npaths;
            }
            // } else {
            //     // binary search
            //     let mut low = 0;
            //     let mut high = q;
            //     loop {
            //         let cur = (low + high) / 2;
            //         let l = npaths * cur as u128;
            //         let r = npaths * (cur as u128 + 1);
            //         if x >= l && x < r {
            //             x -= l;
            //             ds[i] = cur;
            //             break;
            //         }
            //         if x < l {
            //             high = cur;
            //         } else {
            //             // x >= r
            //             low = cur;
            //         }
            //     }
            // }
        }
        ds
    }

    /// Unpack the wire represented by a `Block` with modulus `q`. Assumes that
    /// the block was constructed through the `Wire` API.
    pub fn from_block(inp: Block, modulus: &Modulus) -> Self {
        match *modulus {
            Modulus::Zq { q } => Wire::from_block_mod(inp, q),
            Modulus::GF4 { p } => Wire::from_block_GFk(inp, p.into(), 4),
            Modulus::GF8 { p } => Wire::from_block_GFk(inp, p, 8),
            Modulus::GFk { k, p } => Wire::from_block_GFk(inp, p, k),
        }
    }

    fn from_block_mod(inp: Block, q: u16) -> Self {
        if q == 2 {
            Wire::Mod2 { val: inp }
        } else if q == 3 {
            let inp = u128::from(inp);
            let lsb = inp as u64;
            let msb = (inp >> 64) as u64;
            debug_assert_eq!(lsb & msb, 0);
            Wire::Mod3 { lsb, msb }
        } else {
            let ds = if util::is_power_of_2(q) {
                // It's a power of 2, just split the digits.
                let ndigits = util::digits_per_u128(q);
                let width = 128 / ndigits;
                let mask = (1 << width) - 1;
                let x = u128::from(inp);
                (0..ndigits)
                    .map(|i| ((x >> (width * i)) & mask) as u16)
                    .collect::<Vec<u16>>()
            } else if q <= 23 {
                Self::_unrank(u128::from(inp), q)
            } else if base_conversion::lookup_defined_for_mod(q) {
                Self::_from_block_lookup(inp, q)
            } else {
                // If all else fails, do unrank using naive division.
                Self::_unrank(u128::from(inp), q)
            };
            Wire::ModN { q, ds }
        }
    }

    fn from_block_GFk(inp: Block, p: u16, k: u8) -> Self {
        let inp = u128::from(inp);
        let mut _inp = inp;
        let mut elts: Vec<u16> = Vec::new();
        let length = 128 / k;
        for _ in 0..length {
            elts.push((_inp & (2_u16.pow(k as u32) - 1) as u128) as u16);
            _inp >>= k;
        }

        match k {
            4 => Wire::GF4 { p: p as u8, elts },
            8 => Wire::GF8 { p, elts },
            _ => Wire::GFk { k, p, elts },
            }
        }
        

    /// Pack the wire into a `Block`.
    pub fn as_block(&self) -> Block {
        match self {
            Wire::Mod2 { val } => *val,
            Wire::Mod3 { lsb, msb } => Block::from(((*msb as u128) << 64) | (*lsb as u128)),
            Wire::ModN { q, ref ds } => Block::from(util::from_base_q(ds, *q)),
            Wire::GF4 { p, elts } => Block::from(util::from_poly_p_array(elts, *p as u16, 4)),
            Wire::GF8 { p, elts } => Block::from(util::from_poly_p_array(elts, *p as u16, 8)),
            Wire::GFk { k, p, elts } => Block::from(util::from_poly_p_u128(elts, *p as u16, *k)),
        }
    }

    /// The zero wire with modulus `q`.
    pub fn zero(modulus: &Modulus) -> Self {
        match *modulus {
            Modulus::Zq { q: 0 } => panic!("[Wire::zero] mod 0 not allowed!"),
            Modulus::Zq { q: 1 } => panic!("[Wire::zero] mod 1 not allowed!"),
            Modulus::Zq { q: 2 } => Wire::Mod2 {
                val: Default::default(),
            },
            Modulus::Zq { q: 3 } => Wire::Mod3 {
                lsb: Default::default(),
                msb: Default::default(),
            },
            Modulus::Zq { q } => Wire::ModN {
                q,
                ds: vec![0; util::digits_per_u128(q)],
            },
            Modulus::GF4 { p } => Wire::GF4 { 
                p,
                elts: vec![0; 32],
            },
            Modulus::GF8 { p } => Wire::GF8 {
                p, 
                elts: vec![0; 16],
            }, 
            Modulus::GFk { k, p } => Wire::GFk {
                k, 
                p, 
                elts: vec![0; (128/k).into()],
            },
        }
    }

    /// Get a random wire label mod `q`, with the first digit set to `1`.
    pub fn rand_delta<R: CryptoRng + Rng>(rng: &mut R, modulus: &Modulus) -> Self {
        let mut w = Self::rand(rng, modulus);
        match w {
            Wire::Mod2 { ref mut val } => *val = val.set_lsb(),
            Wire::Mod3 {
                ref mut lsb,
                ref mut msb,
            } => {
                // We want the color digit to be `1`, which requires setting the
                // appropriate `lsb` element to `1` and the appropriate `msb`
                // element to `0`.
                *lsb |= 1;
                *msb &= 0xFFFF_FFFF_FFFF_FFFE;
            }
            Wire::ModN { ref mut ds, .. } => ds[0] = 1,
            Wire::GF4 { ref mut elts, .. } => elts[0] = 1,
            Wire::GF8 { ref mut elts, .. } => elts[0] = 1,
            Wire::GFk { ref mut elts, .. } => elts[0] = 1,
        }
        w
    }

    /// Get the color digit of the wire.
    pub fn color(&self) -> u16 {
        match self {
            Wire::Mod2 { val } => val.lsb() as u16,
            Wire::Mod3 { lsb, msb } => {
                let color = (((msb & 1) as u16) << 1) | ((lsb & 1) as u16);
                debug_assert_ne!(color, 3);
                color
            }
            Wire::ModN { q, ref ds } => {
                let color = ds[0];
                debug_assert!(color < *q);
                color
            }
            Wire::GF4 { ref elts, .. } => {
                let color = elts[0];
                debug_assert!(color < 16);
                color
            }
            Wire::GF8 { ref elts, .. } => {
                let color = elts[0];
                debug_assert!(color < 256);
                color
            }
            Wire::GFk { k, ref elts, .. } => {
                let color = elts[0];
                debug_assert!(color < 2_u16.pow(*k as u32));
                color
            }
        }
    }

    /// Add two wires digit-wise, returning a new wire.
    pub fn plus(&self, other: &Self) -> Self {
        self.clone().plus_mov(other)
    }

    /// Add another wire digit-wise into this one. Assumes that both wires have
    /// the same modulus.
    pub fn plus_eq<'a>(&'a mut self, other: &Wire) -> &'a mut Wire {
        match (&mut *self, other) {
            (Wire::Mod2 { val: ref mut x }, Wire::Mod2 { val: ref y }) => {
                *x ^= *y;
            }
            (
                Wire::Mod3 {
                    lsb: ref mut a1,
                    msb: ref mut a2,
                },
                Wire::Mod3 { lsb: b1, msb: b2 },
            ) => {
                // As explained in the cited paper above, the following
                // operations do element-wise addition.
                let t = (*a1 | b2) ^ (*a2 | b1);
                let c1 = (*a2 | b2) ^ t;
                let c2 = (*a1 | b1) ^ t;
                *a1 = c1;
                *a2 = c2;
            }
            (
                Wire::ModN {
                    q: ref xmod,
                    ds: ref mut xs,
                },
                Wire::ModN {
                    q: ref ymod,
                    ds: ref ys,
                },
            ) => {
                debug_assert_eq!(xmod, ymod);
                debug_assert_eq!(xs.len(), ys.len());
                xs.iter_mut().zip(ys.iter()).for_each(|(x, &y)| {
                    let (zp, overflow) = (*x + y).overflowing_sub(*xmod);
                    *x = if overflow { *x + y } else { zp }
                });
            }
            (
                Wire::GF4 { 
                    p: ref xpoly, 
                    elts: ref mut xs,
                },
                Wire::GF4 { 
                    p: ref ypoly, 
                    elts: ref ys, 
                },
            ) => {
                // Because we work in F(2^k), this is just a bitwise addition in F2. 
                debug_assert_eq!(xpoly, ypoly);
                debug_assert_eq!(xs.len(), ys.len());                
                xs.iter_mut().zip(ys.iter()).for_each(|(x,&y)| {
                    *x ^= y;
                });
            } 
            (
                Wire::GF8 { 
                    p: ref xpoly, 
                    elts: ref mut xs,
                },
                Wire::GF8 { 
                    p: ref ypoly, 
                    elts: ref ys, 
                },
            ) => {
                // Because we work in F(2^k), this is just a bitwise addition in F2. 
                debug_assert_eq!(xpoly, ypoly);
                debug_assert_eq!(xs.len(), ys.len());                
                xs.iter_mut().zip(ys.iter()).for_each(|(x,&y)| {
                    *x ^= y;
                });
            }
            (
                Wire::GFk {
                    k: ref xk, 
                    p: ref xpoly, 
                    elts: ref mut xs,
                },
                Wire::GFk { 
                    k: ref yk,
                    p: ref ypoly, 
                    elts: ref ys, 
                },
            ) => {
                // Because we work in F(2^k), this is just a bitwise addition in F2. 
                debug_assert_eq!(xk,yk);
                debug_assert_eq!(xpoly, ypoly);
                debug_assert_eq!(xs.len(), ys.len());                
                xs.iter_mut().zip(ys.iter()).for_each(|(x,&y)| {
                    *x ^= y;
                });
            }
            _ => panic!("[Wire::plus_eq] unequal moduli!"),
        }

        self
    }

    /// Add another wire into this one, consuming it for chained computations.
    pub fn plus_mov(mut self, other: &Wire) -> Wire {
        self.plus_eq(other);
        self
    }

    /// Multiply each digit by a constant `c mod q`, returning a new wire.
    pub fn cmul(&self, c: u16) -> Self {
        self.clone().cmul_mov(c)
    }
    
    /// Multiply each digit by a constant `c mod q`.
    pub fn cmul_eq(&mut self, c: u16) -> &mut Wire {
        match self {
            Wire::Mod2 { val } => {
                if c & 1 == 0 {
                    *val = Block::default();
                }
            }
            Wire::Mod3 { lsb, msb } => match c {
                0 => {
                    *lsb = 0;
                    *msb = 0;
                }
                1 => {}
                2 => {
                    // Multiplication by two is the same as negation in `mod-3`,
                    // which just involves swapping `lsb` and `msb`.
                    std::mem::swap(lsb, msb);
                }
                c => {
                    self.cmul_eq(c % 3);
                }
            },
            Wire::ModN { q, ds } => {
                ds.iter_mut()
                    .for_each(|d| *d = (*d as u32 * c as u32 % *q as u32) as u16);
            },
            Wire::GF4 { p, elts } => {
                elts.iter_mut().for_each(|d| {
                    *d = util::field_mul(*d, c, (*p).into(), 4) as u16;
                });
            }
            Wire::GF8 { p, elts } => {
                elts.iter_mut().for_each(|d| {
                    *d = util::field_mul(*d, c, *p, 8) as u16;
                });
            }
            Wire::GFk {k, p, elts} => {
                elts.iter_mut().for_each(|d| {
                    *d = util::field_mul(*d, c, *p, *k) as u16;
                });
            }
        }
        self
    }

    /// Multiply each digit by a constant `c mod q`, consuming it for chained computations.
    pub fn cmul_mov(mut self, c: u16) -> Wire {
        self.cmul_eq(c);
        self
    }

    /// Negate all the digits `mod q`, returning a new wire.
    pub fn negate(&self) -> Self {
        self.clone().negate_mov()
    }

    /// Negate all the digits mod q.
    pub fn negate_eq(&mut self) -> &mut Wire {
        match self {
            Wire::Mod2 { .. } => {
                // Do nothing. Additive inverse is a no-op for mod 2.
            }
            Wire::Mod3 { lsb, msb } => {
                // Negation just involves swapping `lsb` and `msb`.
                std::mem::swap(lsb, msb);
            }
            Wire::ModN { q, ds } => {
                ds.iter_mut().for_each(|d| {
                    if *d > 0 {
                        *d = *q - *d;
                    } else {
                        *d = 0;
                    }
                });
            }
            Wire::GF4 { .. } => {
                // Do nothing. Additive inverse is a no-op for coefficients with mod 2.
            }
            Wire::GF8 { .. } => {
                // Do nothing. Additive inverse is a no-op for coefficients with mod 2.
            }
            Wire::GFk { .. } => {
                // Do nothing. Additive inverse is a no-op for coefficients with mod 2.
            }
        }
        self
    }

    /// Negate all the digits `mod q`, consuming it for chained computations.
    pub fn negate_mov(mut self) -> Wire {
        self.negate_eq();
        self
    }

    /// Subtract two wires, returning the result.
    pub fn minus(&self, other: &Wire) -> Wire {
        self.clone().minus_mov(other)
    }

    /// Subtract a wire from this one.
    pub fn minus_eq<'a>(&'a mut self, other: &Wire) -> &'a mut Wire {
        self.plus_eq(&other.negate())
    }

    /// Subtract a wire from this one, consuming it for chained computations.
    pub fn minus_mov(mut self, other: &Wire) -> Wire {
        self.minus_eq(other);
        self
    }

    /// Get a random wire `mod q`.
    pub fn rand<R: CryptoRng + RngCore>(rng: &mut R, modulus: &Modulus) -> Wire {
        match *modulus { 
            Modulus::Zq { q } => {
                if q == 2 {
                    Wire::Mod2 { val: rng.gen() }
                } else if q == 3 {
                    // Generate 64 mod-three values and then embed them into `lsb` and
                    // `msb`.
                    let mut lsb = 0u64;
                    let mut msb = 0u64;
                    for (i, v) in (0..64).map(|_| rng.gen::<u8>() % 3).enumerate() {
                        lsb |= ((v & 1) as u64) << i;
                        msb |= (((v >> 1) & 1) as u64) << i;
                    }
                    debug_assert_eq!(lsb & msb, 0);
                    Wire::Mod3 { lsb, msb }
                } else {
                    let ds = (0..util::digits_per_u128(q))
                        .map(|_| rng.gen::<u16>() % q)
                        .collect();
                    Wire::ModN { q, ds }
                }
            },
            Modulus::GF4 { p } => {
                let elts = (0..32)
                    .map(|_| (rng.gen::<u8>()&(15)) as u16)
                    .collect();
                Wire::GF4 { p, elts }
            },
            Modulus::GF8 { p } => {
                let elts = (0..32)
                    .map(|_| (rng.gen::<u16>()&(255)) as u16)
                    .collect();
                Wire::GF8 { p, elts }
            },
            Modulus::GFk {k, p} => {
                let elts = (0..32)
                    .map(|_| (rng.gen::<u16>()&((1<<k) - 1)) as u16)
                    .collect();
                Wire::GFk { k, p, elts }
            },
        }
    }

    /// Compute the hash of this wire.
    ///
    /// Uses fixed-key AES.
    #[inline(never)]
    pub fn hash(&self, tweak: Block) -> Block {
        AES_HASH.tccr_hash(tweak, self.as_block())
    }

    /// Compute the hash of this wire, converting the result back to a wire.
    ///
    /// Uses fixed-key AES.
    pub fn hashback(&self, tweak: Block, modulus: u16) -> Wire {
        let block = self.hash(tweak);
        match *self {
            Wire::GF4 { .. } => {
                Self::from_block(block, &Modulus::GF4 { p: modulus as u8 })
            },
            Wire::GF8 { .. } => {
                Self::from_block(block, &Modulus::GF8 { p: modulus })
            },
            Wire::GFk { k, .. } => {
                Self::from_block(block, &Modulus::GFk { k, p: modulus })
            }
            _ => {
                if modulus == 3 {
                    // We have to convert `block` into a valid `Mod3` encoding. We do
                    // this by computing the `Mod3` digits using `_unrank`, and then map
                    // these to a `Mod3` encoding.
                    let mut lsb = 0u64;
                    let mut msb = 0u64;
                    let mut ds = Self::_unrank(u128::from(block), modulus);
                    for (i, v) in ds.drain(..64).enumerate() {
                        lsb |= ((v & 1) as u64) << i;
                        msb |= (((v >> 1) & 1u16) as u64) << i;
                    }
                    debug_assert_eq!(lsb & msb, 0);
                    Wire::Mod3 { lsb, msb }
                } else {
                    Self::from_block(block, &Modulus::Zq { q: modulus })
                }
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::RngExt;
    use itertools::{Itertools};
    use rand::{thread_rng, seq::SliceRandom};

    #[test]
    fn modM_eq() {
        let M1 = Modulus::Zq { q: 4 };
        let M2 = Modulus::Zq { q: 4 };

        assert_eq!(M1, M2);

        let mut vM1 = Vec::new();
        let mut vM2 = Vec::new();
        for qq in 0..5 {
            vM1.push(Modulus::Zq { q: qq });
            vM2.push(Modulus::Zq { q: qq });
        }

        assert_eq!(vM1, vM2);
    }

    #[test]
    fn packing_Zq() {
        let ref mut rng = thread_rng();
        for q in 2..256 {
            for _ in 0..1000 {
                let w = Wire::rand(rng, &Modulus::Zq{ q });
                println!("mod: {}", w.modulus());
                assert_eq!(w, Wire::from_block(w.as_block(), &Modulus::Zq{ q }));
            }
        }
    }

    #[test]
    fn packing_GF() {
        let ref mut rng = thread_rng();
        let irred_GF4 = vec!(0b10011, 0b11001, 0b11111);  // all irreducible polynomials for GF(2^4)
        for p in irred_GF4.into_iter() {  // 0b10011 is X^4 + X + 1
            for _ in 0..1000 {
                let w = Wire::rand(rng, &Modulus::GF4{ p });
                assert_eq!(w, Wire::from_block(w.as_block(), &Modulus::GF4{ p }));
            }
        }
    }

    #[test]
    fn base_conversion_lookup_method() {
        let ref mut rng = thread_rng();
        for _ in 0..1000 {
            let q = 5 + (rng.gen_u16() % 110);
            let x = rng.gen_u128();
            let w = Wire::from_block(Block::from(x), &Modulus::Zq{ q });
            let should_be = util::as_base_q_u128(x, q);
            assert_eq!(w.digits(), should_be, "x={} q={}", x, q);

            let irred_GF4 = vec!(0b10011, 0b11001, 0b11111);  // all irreducible polynomials for GF(2^4)
            for p in irred_GF4.into_iter() {
                let w = Wire::from_block(Block::from(x), &Modulus::GF4{ p });
                let should_be = util::as_base_q_u128(x, 16);
                assert_eq!(w.digits(), should_be, "x={} p={}", x, p);
            }
        }
    }

    #[test]
    fn hash_Zq() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let q = 2 + (rng.gen_u16() % 110);
            let x = Wire::rand(&mut rng, &Modulus::Zq { q });
            let y = x.hashback(Block::from(1u128), q);
            assert!(x != y);
            match y {
                Wire::Mod2 { val } => assert!(u128::from(val) > 0),
                Wire::Mod3 { lsb, msb } => assert!(lsb > 0 && msb > 0),
                Wire::ModN { ds, .. } => assert!(!ds.iter().all(|&y| y == 0)),
                _ => (),
            }
        }
    }

    #[test]
    fn hash_GF4() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let p = *vec!(19, 21, 31).choose(&mut rng).unwrap() as u8;
            let x = Wire::rand(&mut rng, &Modulus::GF4 { p });
            let y = x.hashback(Block::from(1u128), p as u16);
            assert!(x != y);
            match y {
                Wire::GF4 { elts, .. } => assert!(!elts.iter().all(|&y| y == 0)),
                _ => (),
            }
        }
    }

    #[test]
    fn negation() {
        let ref mut rng = thread_rng();
        for _ in 0..1000 {
            let q = rng.gen_modulus();
            let x = Wire::rand(rng, &Modulus::Zq { q });
            let xneg = x.negate();
            if q != 2 {
                assert!(x != xneg);
            }
            let y = xneg.negate();
            assert_eq!(x, y);
        }
    }

    #[test]
    fn negationGF4() {
        let ref mut rng = thread_rng();
        for _ in 0..1000 {
            let p = 19;
            let x = Wire::rand(rng, &Modulus::GF4 { p: p });
            let xneg = x.negate();
            let y = xneg.negate();
            assert_eq!(x, y);
        }
    }

    #[test]
    fn zero() {
        let mut rng = thread_rng();
        let p = 19; 
        for _ in 0..1000 {
            let q = 3 + (rng.gen_u16() % 110);
            let z = Wire::zero(&Modulus::Zq { q });
            let ds = z.digits();
            assert_eq!(ds, vec![0; ds.len()], "q={}", q);
        }
        // GF4 test
        let z = Wire::zero(&Modulus::GF4 { p: p }); 
        let ds = z.digits();
        assert_eq!(ds, vec![0; ds.len()]);
    }

    #[test]
    fn subzero() {
        let mut rng = thread_rng();
        let p = 19; 
        for _ in 0..1000 {
            let q = rng.gen_modulus();
            let x = Wire::rand(&mut rng, &Modulus::Zq{ q });
            let z = Wire::zero(&Modulus::Zq{ q });
            assert_eq!(x.minus(&x), z);
        }
        // GF4 test 
        let x = Wire::rand(&mut rng, &Modulus::GF4 { p:19  }); 
        let z = Wire::zero(&Modulus::GF4{ p });
        assert_eq!(x.minus(&x),z);
    }

    #[test]
    fn pluszero() {
        let mut rng = thread_rng();
        for _ in 0..1000 {
            let q = rng.gen_modulus();
            let x = Wire::rand(&mut rng, &Modulus::Zq{ q });
            assert_eq!(x.plus(&Wire::zero(&Modulus::Zq{ q })), x);
        }
        // GF4 test
        let x = Wire::rand(&mut rng, &Modulus::GF4{ p: 19 });
        assert_eq!(x.plus(&Wire::zero(&Modulus::GF4 { p: 19 })), x);
    }

    #[test]
    fn arithmetic() {
        let mut rng = thread_rng();
        for _ in 0..1024 {
            let q = rng.gen_modulus();
            let x = Wire::rand(&mut rng, &Modulus::Zq { q });
            let y = Wire::rand(&mut rng, &Modulus::Zq { q });
            assert_eq!(x.cmul(0), Wire::zero(&Modulus::Zq{ q }));
            assert_eq!(x.cmul(q), Wire::zero(&Modulus::Zq{ q }));
            assert_eq!(x.plus(&x), x.cmul(2));
            assert_eq!(x.plus(&x).plus(&x), x.cmul(3));
            assert_eq!(x.negate().negate(), x);
            if q == 2 {
                assert_eq!(x.plus(&y), x.minus(&y));
            } else {
                assert_eq!(x.plus(&x.negate()), Wire::zero(&Modulus::Zq{ q }), "q={}", q);
                assert_eq!(x.minus(&y), x.plus(&y.negate()));
            }
            let mut w = x.clone();
            let z = w.plus(&y);
            w.plus_eq(&y);
            assert_eq!(w, z);

            w = x.clone();
            w.cmul_eq(2);
            assert_eq!(x.plus(&x), w);

            w = x.clone();
            w.negate_eq();
            assert_eq!(x.negate(), w);
        }
    }

    #[test]
    fn basic_arithmeticGF4() {
        let p = 19;
        let mut rng = thread_rng(); 
        for _ in 0..1000 {
            let x = Wire::rand(&mut rng, &Modulus::GF4 { p: p });
            assert_eq!(x.cmul(0), Wire::zero(&Modulus::GF4{ p: p}));
            assert_eq!(x.negate().negate(), x);
        }
    }

    #[test]
    fn cmul_arithmeticGF4() {
        let mut rng = thread_rng(); 
        // Irreducible polynomial X^4 + X + 1
        const X4_X_1: u8 = 19;
        
        

        // (x^3 + 1) * (x^3 + x) = x^2 + 1
        let w = Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(3) + 1]}; 
        assert_eq!(w.cmul(2_u16.pow(3)+2), Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(2)+1] });

        //( x^3) * ( x^3 + x^2 + x+1)        
        let w = Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(3)]}; 
        assert_eq!(w.cmul(2_u16.pow(3)+2_u16.pow(2)+2+1), Wire::GF4 { p: (X4_X_1), elts: vec![1] });

        // ( x^2 +1) * ( x^3 + x^2 + x + 1)
        let w = Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(2)+1]}; 
        assert_eq!(w.cmul(2_u16.pow(3)+2_u16.pow(2)+2+1), Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(2)+2] });

        // ( x^3 +1) * ( x^3 + x^2 + x+1)
        let w = Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(3)+1]}; 
        assert_eq!(w.cmul(2_u16.pow(3)+2_u16.pow(2)+2+1), Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(3)+2_u16.pow(2)+2] });

        let w = Wire::GF4 { p: (X4_X_1), elts: vec!(2_u16.pow(3))}; 
        assert_eq!(w.cmul(2_u16.pow(3)), Wire::GF4 { p: (X4_X_1), elts: vec![2_u16.pow(3) + 4] });


        let w = Wire::rand(&mut rng, &Modulus::GF4 { p: X4_X_1 });
        assert_eq!(w.cmul(1),w);
    }
    #[test]
    fn cmul_arithmeticGF8() {
        // Irreducible polynomial X^8 + X^4 + X^3 + X + 1
        const X8_X4_X3_X_1: u16 = 283;

        // (x^7+x^3+1)*(x^6+x^5+x^2+x+1) = x^5 + x^4 + x^3 + 1
        let w = Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(7) + 2_u16.pow(3) +1] };
        assert_eq!(w.cmul(2_u16.pow(6) + 2_u16.pow(5) + 2_u16.pow(2) + 2 + 1),Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(5)+2_u16.pow(4)+2_u16.pow(3)+1] });

        // (x^6+x^4+x)*(x^7+x^6+x^5+x+1) = x^7 + x^4 + 1
        let w = Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(6) + 2_u16.pow(4) + 2] };
        assert_eq!(w.cmul(2_u16.pow(7) + 2_u16.pow(6) + 2_u16.pow(5) + 2 + 1),Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(7)+2_u16.pow(4)+1] });

        // (x^7 + x^3)*x^7 = x^7 + x^6 + x^5 + x^4 + x^2 + x
        let w = Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(7) + 2_u16.pow(3)] };
        assert_eq!(w.cmul(2_u16.pow(7)),Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(7)+2_u16.pow(6)+ 2_u16.pow(5)+ 2_u16.pow(4)+2_u16.pow(2)+2]});

        // x^7*x^7 = x^7 + x^4 + x^3 + x
        let w = Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(7)] };
        assert_eq!(w.cmul(2_u16.pow(7)),Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![2_u16.pow(7)+2_u16.pow(4)+ 2_u16.pow(3)+2]});

    }

    #[test]
    fn ndigits_correct() {
        let mut rng = thread_rng();
        for _ in 0..1024 {
            let q = rng.gen_modulus();
            let x = Wire::rand(&mut rng, &Modulus::Zq { q });
            assert_eq!(x.digits().len(), util::digits_per_u128(q));
        }
    }

    #[test]
    fn parallel_hash_Zq() {
        let n = 1000;
        let mut rng = thread_rng();
        let q = rng.gen_modulus();
        let ws = (0..n).map(|_| Wire::rand(&mut rng, &Modulus::Zq { q })).collect_vec();

        let hashes = crossbeam::scope(|scope| {
            let hs = ws
                .iter()
                .map(|w| scope.spawn(move |_| w.hash(Block::default())))
                .collect_vec();
            hs.into_iter().map(|h| h.join().unwrap()).collect_vec()
        })
        .unwrap();

        let should_be = ws.iter().map(|w| w.hash(Block::default())).collect_vec();

        assert_eq!(hashes, should_be);
    }

    #[test]
    fn parallel_hash_GF4() {
        let n = 1000;
        let mut rng = thread_rng();
        let p = *vec!(19, 21, 31).choose(&mut rng).unwrap() as u8;
        let ws = (0..n).map(|_| Wire::rand(&mut rng, &Modulus::GF4 { p })).collect_vec();

        let hashes = crossbeam::scope(|scope| {
            let hs = ws
                .iter()
                .map(|w| scope.spawn(move |_| w.hash(Block::default())))
                .collect_vec();
            hs.into_iter().map(|h| h.join().unwrap()).collect_vec()
        })
        .unwrap();

        let should_be = ws.iter().map(|w| w.hash(Block::default())).collect_vec();

        assert_eq!(hashes, should_be);
    }
}
