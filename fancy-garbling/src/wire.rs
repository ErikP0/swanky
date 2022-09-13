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
    /// An element in the field GF(2^k) is represented by the coefficients of the polynomial
    /// EXAMPLE: x^3 + x + 1 in GF(2^4)
    ///   ===>   Elt: u16 = (0000...0 1 0 1 1) = 11
    /// 
    /// 
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
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
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
            Modulus::Zq { q } => write!(fmt, "Zq(q={})", q),
            Modulus::GF4 { p } => write!(fmt, "GF4(p={})", p),
            Modulus::GF8 { p } => write!(fmt, "GF8(p={})",p),
            Modulus::GFk { k, p } => write!(fmt, "GF2^{}(p={})",k,p),
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
    /// Irreducible polynomial X^4 + X + 1 in GF(2^4)
    pub const X4_X_1: Modulus = Modulus::GF4 {p: 0b10011};
    /// Irreducible polynomial X^4 + X^2 + 1 in GF(2^4)
    pub const X4_X2_1: Modulus = Modulus::GF4 {p: 0b10101};
    /// Irreducible polynomial X^4 + X^3 + X^2 + X + 1 in GF(2^4)
    pub const X4_X3_X2_X_1: Modulus = Modulus::GF4 {p: 0b11111};

    /// All moduli of GF(2^4)
    pub const GF4_MODULI: [Modulus; 3] = [Self::X4_X_1, Self::X4_X2_1, Self::X4_X3_X2_X_1];

    /// All moduli of GF(2^8)
    pub const GF8_MODULI: [Modulus; 30] = [
        Modulus::GF8 { p: 0b100011011 }, Modulus::GF8 { p: 0b100011101 }, Modulus::GF8 { p: 0b100101011 },
        Modulus::GF8 { p: 0b100101101 }, Modulus::GF8 { p: 0b100111001 }, Modulus::GF8 { p: 0b100111111 },
        Modulus::GF8 { p: 0b101001101 }, Modulus::GF8 { p: 0b101011111 }, Modulus::GF8 { p: 0b101100011 },
        Modulus::GF8 { p: 0b101100101 }, Modulus::GF8 { p: 0b101101001 }, Modulus::GF8 { p: 0b101110001 },
        Modulus::GF8 { p: 0b101110111 }, Modulus::GF8 { p: 0b101111011 }, Modulus::GF8 { p: 0b110000111 },
        Modulus::GF8 { p: 0b110001011 }, Modulus::GF8 { p: 0b110001101 }, Modulus::GF8 { p: 0b110011111 },
        Modulus::GF8 { p: 0b110100011 }, Modulus::GF8 { p: 0b110101001 }, Modulus::GF8 { p: 0b110110001 },
        Modulus::GF8 { p: 0b110111101 }, Modulus::GF8 { p: 0b111000011 }, Modulus::GF8 { p: 0b111001111 },
        Modulus::GF8 { p: 0b111010111 }, Modulus::GF8 { p: 0b111011101 }, Modulus::GF8 { p: 0b111100111 },
        Modulus::GF8 { p: 0b111110011 }, Modulus::GF8 { p: 0b111110101 }, Modulus::GF8 { p: 0b111111001 },
    ];

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

    /// Returns true if the modulus is GF(2^n) for some n
    pub fn is_field(&self) -> bool {
        match self {
            Modulus::Zq {..} => false,
            Modulus::GF4 {..} | Modulus::GF8 {..} | Modulus::GFk {..} => true
        }
    }

    /// returns the number of bits required to represent an element mod self
    pub fn bit_length(&self) -> usize {
        match self {
            Modulus::Zq{ q } => f32::from(*q).log(2.0).ceil() as usize,
            Modulus::GF4 { .. } => 4,
            Modulus::GF8 { .. } => 8,
            Modulus::GFk { k, .. } => (*k).into(),
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
        let mut inp = u128::from(inp);
        let mut elts: Vec<u16> = Vec::new();
        let length = 128 / k;
        let mask = ((1 << k)-1) as u16;
        for _ in 0..length {
            elts.push((inp & mask as u128) as u16);
            inp >>= k;
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
                Wire::GF4 { p: ref xpoly, elts: ref mut xs },
                Wire::GF4 { p: ref ypoly, elts: ref ys},
            ) => {
                // Because we work in F(2^k), this is just a bitwise addition in F2.
                debug_assert_eq!(xpoly, ypoly);
                debug_assert_eq!(xs.len(), ys.len());
                xs.iter_mut().zip(ys.iter()).for_each(|(x,&y)| {
                    *x ^= y;
                });
            },
            (
                Wire::GF8 { p: ref xpoly, elts: ref mut xs },
                Wire::GF8 { p: ref ypoly, elts: ref ys},
            )
            | (
                Wire::GFk { p: ref xpoly, elts: ref mut xs, .. },
                Wire::GFk { p: ref ypoly, elts: ref ys, .. },
            )
            => {
                // Because we work in F(2^k), this is just a bitwise addition in F2. 
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
            // Uses the field_mul function to multiply to elements(polynomials)
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
            Wire::GF4 { .. } | Wire::GF8 { .. } | Wire::GFk { .. } => {
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
            // Generate random number and and it with a mask to make it 
            // a random element in the field.
            Modulus::GF4 { p } => {
                let elts = (0..32)
                    .map(|_| (rng.gen::<u8>()&(15)) as u16)
                    .collect();
                Wire::GF4 { p, elts }
            },
            Modulus::GF8 { p } => {
                let elts = (0..16)
                    .map(|_| (rng.gen::<u16>()&(255)) as u16)
                    .collect();
                Wire::GF8 { p, elts }
            },
            Modulus::GFk {k, p} => {
                let elts = (0..(128 / k))
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
    pub fn hashback(&self, tweak: Block, modulus: &Modulus) -> Wire {
        let block = self.hash(tweak);
        if let Modulus::Zq {q: 3} = modulus {
            // We have to convert `block` into a valid `Mod3` encoding. We do
            // this by computing the `Mod3` digits using `_unrank`, and then map
            // these to a `Mod3` encoding.
            let mut lsb = 0u64;
            let mut msb = 0u64;
            let mut ds = Self::_unrank(u128::from(block), 3);
            for (i, v) in ds.drain(..64).enumerate() {
                lsb |= ((v & 1) as u64) << i;
                msb |= (((v >> 1) & 1u16) as u64) << i;
            }
            debug_assert_eq!(lsb & msb, 0);
            return Wire::Mod3 { lsb, msb }
        }
        Self::from_block(block, modulus)
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
    use std::fs::File;
    use std::io::{BufReader,prelude::*};
    use std::path::Path;
    
     
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
    fn packing_GF4() {
        let ref mut rng = thread_rng();
        // iterate over all irreducible polynomials for GF(2^4)
        for p in &Modulus::GF4_MODULI {
            for _ in 0..1000 {
                let w = Wire::rand(rng, p);
                assert_eq!(w, Wire::from_block(w.as_block(), &p));
            }
        }
    }

    #[test]
    fn packing_GF8() {
        let ref mut rng = thread_rng();
        // all irreducible polynomials for GF(2^8)
        for p in &Modulus::GF8_MODULI {
            for _ in 0..1000 {
                let w = Wire::rand(rng, &p);
                assert_eq!(w, Wire::from_block(w.as_block(), &p));
            }
        }
    }

    #[test]
    fn packing_GFk() {
        let ref mut rng = thread_rng();
        let irred_GFk = vec!(0b1101, 0b1011,
                             0b100101, 0b110111, 0b111011,
                             0b1000011, 0b1101101, 0b1110101,
                             0b10000011, 0b10011101, 0b10111111);  // some irreducible polynomials for GF(2^k)
        let ks = vec!(3, 3,
                      5, 5, 5,
                      6, 6, 6,
                      7, 7, 7);
        for i in 0..irred_GFk.len() {
            for _ in 0..1000 {
                let w = Wire::rand(rng, &Modulus::GFk { p: irred_GFk[i], k: ks[i] });
                assert_eq!(w, Wire::from_block(w.as_block(), &Modulus::GFk { p: irred_GFk[i], k: ks[i] }));
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

            // all irreducible polynomials for GF(2^4)
            for p in &Modulus::GF4_MODULI {
                let w = Wire::from_block(Block::from(x), p);
                let should_be = util::as_base_q_u128(x, 16);
                assert_eq!(w.digits(), should_be, "x={} p={}", x, p.value());
            }
        }
    }

    #[test]
    fn hash_Zq() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let q = 2 + (rng.gen_u16() % 110);
            let x = Wire::rand(&mut rng, &Modulus::Zq { q });
            let y = x.hashback(Block::from(1u128), &Modulus::Zq { q });
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
            let p = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
            let x = Wire::rand(&mut rng, &p);
            let y = x.hashback(Block::from(1u128), &p);
            assert_ne!(x, y);
            match y {
                Wire::GF4 { elts, .. } => assert!(!elts.iter().all(|&y| y == 0)),
                _ => panic!(),
            }
        }
    }

    #[test]
    fn hash_GFk() {
        let mut rng = thread_rng();
        let irred_GFk = vec!((0b1101, 3), (0b1011, 3),
            (0b100101, 5), (0b110111, 5), (0b111011, 5), 
            (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
            (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)); 
        for _ in 0..100 {
            let p = *irred_GFk.choose(&mut rng).unwrap();
            let x = Wire::rand(&mut rng, &Modulus::GFk { p: p.0, k: p.1 });
            let y = x.hashback(Block::from(1u128), &Modulus::GFk { p: p.0, k: p.1 });
            match y {
                Wire::GFk { elts, .. } => assert!(!elts.iter().all(|&y| y == 0)),
                _ => panic!(),
            }
        }
    }

    #[test]
    fn hash_GF8() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
            let x = Wire::rand(&mut rng, p);
            let y = x.hashback(Block::from(1u128), p);
            assert_ne!(x, y);
            match y {
                Wire::GF8 { elts, .. } => assert!(!elts.iter().all(|&y| y == 0)),
                _ => panic!(),
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
    fn negationGF() {
        let ref mut rng = thread_rng();
        let mut x; let mut y;
        let irred_GFk = vec!((0b1101, 3), (0b1011, 3),
            (0b100101, 5), (0b110111, 5), (0b111011, 5), 
            (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
            (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)); 
        for _ in 0..1000 {
            // GF4
            x = Wire::rand(rng, &Modulus::X4_X_1);
            let xneg = x.negate();
            y = xneg.negate();
            assert_eq!(x, y);

            // GF8
            x = Wire::rand(rng, &Modulus::GF8 { p: 283 });
            let xneg = x.negate();
            y = xneg.negate();
            assert_eq!(x, y);

            // GFk
            let (p,k) = irred_GFk.choose(rng).unwrap();
            x = Wire::rand(rng, &Modulus::GFk { p: *p, k: *k });
            let xneg = x.negate();
            y = xneg.negate();
            assert_eq!(x, y);
        }
    }

    #[test]
    fn zero() {
        let mut rng = thread_rng();
        for _ in 0..1000 {
            let q = 3 + (rng.gen_u16() % 110);
            let z = Wire::zero(&Modulus::Zq { q });
            let ds = z.digits();
            assert_eq!(ds, vec![0; ds.len()], "q={}", q);
        }
        // GF4 test
        let z = Wire::zero(&Modulus::X4_X_1);
        let ds = z.digits();
        assert_eq!(ds, vec![0; ds.len()]);

        // GF8 test
        let z = Wire::zero(&Modulus::GF8 { p: 283 });
        let ds = z.digits();
        assert_eq!(ds, vec![0; ds.len()]);

        // GFk test
        let irred_GFk = vec!((0b1101, 3), (0b1011, 3),
            (0b100101, 5), (0b110111, 5), (0b111011, 5), 
            (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
            (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)); 
        let (p,k) = irred_GFk.choose(&mut rng).unwrap();
        let z = Wire::zero(&Modulus::GFk { p: *p, k: *k });
        let ds = z.digits();
        assert_eq!(ds, vec![0; ds.len()]);
    }

    #[test]
    fn subzero() {
        let mut rng = thread_rng();
        for _ in 0..1000 {
            let q = rng.gen_modulus();
            let x = Wire::rand(&mut rng, &Modulus::Zq{ q });
            let z = Wire::zero(&Modulus::Zq{ q });
            assert_eq!(x.minus(&x), z);
        }
        // GF4 test
        let x = Wire::rand(&mut rng, &Modulus::X4_X_1);
        let z = Wire::zero(&Modulus::X4_X_1);
        assert_eq!(x.minus(&x),z);

        // GF8 test
        let x = Wire::rand(&mut rng, &Modulus::GF8 { p: 283  });
        let z = Wire::zero(&Modulus::GF8{ p: 283 });
        assert_eq!(x.minus(&x),z);

        // GFk test 
        let irred_GFk = vec!((0b1101, 3), (0b1011, 3),
            (0b100101, 5), (0b110111, 5), (0b111011, 5), 
            (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
            (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)); 
        let (p, k) = irred_GFk.choose(&mut rng).unwrap();
        let x = Wire::rand(&mut rng, &Modulus::GFk { p: *p, k: *k  });
        let z = Wire::zero(&Modulus::GFk{ p: *p, k: *k });
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
        let x = Wire::rand(&mut rng, &Modulus::X4_X_1);
        assert_eq!(x.plus(&Wire::zero(&Modulus::X4_X_1)), x);

        // GF8 test
        let x = Wire::rand(&mut rng, &Modulus::GF8 { p: 283 });
        let z = Wire::zero(&Modulus::GF8{ p: 283 });
        assert_eq!(x.plus(&x),z);

        // GFk test 
        let irred_GFk = vec!((0b1101, 3), (0b1011, 3),
            (0b100101, 5), (0b110111, 5), (0b111011, 5), 
            (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
            (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)); 
        let (p, k) = irred_GFk.choose(&mut rng).unwrap();
        let x = Wire::rand(&mut rng, &Modulus::GFk { p: *p, k: *k  });
        let z = Wire::zero(&Modulus::GFk{ p: *p, k: *k });
        assert_eq!(x.plus(&x),z);

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
    fn basic_arithmeticGF() {
        let mut rng = thread_rng();
        let p_GF4 = Modulus::X4_X_1;
        let p_GF8 = Modulus::GF8 { p: 283 };
        let irred_GFk = vec!((0b1101, 3), (0b1011, 3),
            (0b100101, 5), (0b110111, 5), (0b111011, 5), 
            (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
            (0b10000011, 7), (0b10011101, 7), (0b10111111, 7)); 
        let p_GFk = irred_GFk.choose(&mut rng).unwrap();
        
        let mut rng = thread_rng(); 
        for _ in 0..1000 {
            // GF 4
            let x = Wire::rand(&mut rng, &p_GF4);
            assert_eq!(x.cmul(0), Wire::zero(&p_GF4));
            assert_eq!(x.negate().negate(), x);

            // GF8
            let x = Wire::rand(&mut rng, &p_GF8);
            assert_eq!(x.cmul(0), Wire::zero(&p_GF8));
            assert_eq!(x.negate().negate(), x);

            // GF8
            let x = Wire::rand(&mut rng, &Modulus::GFk { p: p_GFk.0, k: p_GFk.1 });
            assert_eq!(x.cmul(0), Wire::zero(&Modulus::GFk { p: p_GFk.0, k: p_GFk.1 }));
            assert_eq!(x.negate().negate(), x);
        }
    }


    #[test]
    fn cmul_GF4_x4_x_1() {
        let filename = Path::new("./helper_test_files/test_gf4_x4_x_1.txt");
        let file = File::open(filename)
            .expect("file not found!");
        let  buf_reader = BufReader::new(file);

        const X4_X_1: u8 = 19;

        for line in buf_reader.lines() {
            let line = line.expect("Unable to read line");
            let p: Vec<&str> = line.split(' ').collect();
            let (x, y, z) = p.into_iter().map(|s| s.parse::<u16>().unwrap()).collect_tuple().unwrap();
            let w = Wire::GF4 { p: (X4_X_1), elts: vec![x]};
            assert_eq!(w.cmul(y), Wire::GF4 { p: (X4_X_1), elts: vec![z] }); 
        }
    }

    #[test]
    fn cmul_GF4_x4_x3_1() {
        let filename = Path::new("./helper_test_files/test_gf4_x4_x3_1.txt");
        let file = File::open(filename)
            .expect("file not found!");
        let  buf_reader = BufReader::new(file);

        const X4_X3_1: u8 = 25;

        for line in buf_reader.lines() {
            let line = line.expect("Unable to read line");
            let p: Vec<&str> = line.split(' ').collect();
            let (x, y, z) = p.into_iter().map(|s| s.parse::<u16>().unwrap()).collect_tuple().unwrap();
            let w = Wire::GF4 { p: (X4_X3_1), elts: vec![x]};
            assert_eq!(w.cmul(y), Wire::GF4 { p: (X4_X3_1), elts: vec![z] }); 
        }
    }

    #[test]
    fn cmul_GF4_x4_x3_x2_x_1() {
        let filename = Path::new("./helper_test_files/test_gf4_x4_x3_x2_x_1.txt");
        let file = File::open(filename)
            .expect("file not found!");
        let  buf_reader = BufReader::new(file);

        const X4_X_1: u8 = 31;

        for line in buf_reader.lines() {
            let line = line.expect("Unable to read line");
            let p: Vec<&str> = line.split(' ').collect();
            let (x, y, z) = p.into_iter().map(|s| s.parse::<u16>().unwrap()).collect_tuple().unwrap();
            let w = Wire::GF4 { p: (X4_X_1), elts: vec![x]};
            assert_eq!(w.cmul(y), Wire::GF4 { p: (X4_X_1), elts: vec![z] }); 
        }
    }

    #[test]
    fn cmul_GF8_x8_x4_x3_x_1() {
        let filename = Path::new("./helper_test_files/test_gf8_x8_x4_x3_x_1.txt");
        let file = File::open(filename)
            .expect("file not found!");
        let  buf_reader = BufReader::new(file);

        // Irreducible polynomial X^8 + X^4 + X^3 + X + 1
        const X8_X4_X3_X_1: u16 = 283;
        

        for line in buf_reader.lines() {
            let line = line.expect("Unable to read line");
            let p: Vec<&str> = line.split(' ').collect();
            let (x, y, z) = p.into_iter().map(|s| s.parse::<u16>().unwrap()).collect_tuple().unwrap();
            let w = Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![x]};
            assert_eq!(w.cmul(y), Wire::GF8 { p: (X8_X4_X3_X_1), elts: vec![z] }); 
        }
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
        let p = Modulus::GF4_MODULI.choose(&mut rng).unwrap();
        let ws = (0..n).map(|_| Wire::rand(&mut rng, p)).collect_vec();

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
    fn parallel_hash_GF8() {
        let n = 1000;
        let mut rng = thread_rng();
        let p = Modulus::GF8_MODULI.choose(&mut rng).unwrap();
        let ws = (0..n).map(|_| Wire::rand(&mut rng, p)).collect_vec();

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
    fn parallel_hash_GFk() {
        let n = 1000;
        let mut rng = thread_rng();
        let irred_GFk = vec!((0b1101, 3), (0b1011, 3),
                             (0b100101, 5), (0b110111, 5), (0b111011, 5),
                             (0b1000011, 6), (0b1101101, 6), (0b1110101, 6),
                             (0b10000011, 7), (0b10011101, 7), (0b10111111, 7));
        let (p, k) = *irred_GFk.choose(&mut rng).unwrap();
        let ws = (0..n).map(|_| Wire::rand(&mut rng, &Modulus::GFk { p, k })).collect_vec();

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
