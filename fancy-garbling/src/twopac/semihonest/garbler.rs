// -*- mode: rust; -*-
//
// This file is part of twopac.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

use crate::{errors::TwopacError, Fancy, FancyInput, FancyReveal, Garbler as Gb, Wire, Modulus};
use ocelot::ot::Sender as OtSender;
use rand::{CryptoRng, Rng, SeedableRng};
use scuttlebutt::{AbstractChannel, Block, SemiHonest};

/// Semi-honest garbler.
pub struct Garbler<C, RNG, OT> {
    garbler: Gb<C, RNG>,
    channel: C,
    ot: OT,
    rng: RNG,
}

impl<C, OT, RNG> std::ops::Deref for Garbler<C, RNG, OT> {
    type Target = Gb<C, RNG>;
    fn deref(&self) -> &Self::Target {
        &self.garbler
    }
}

impl<C, OT, RNG> std::ops::DerefMut for Garbler<C, RNG, OT> {
    fn deref_mut(&mut self) -> &mut Gb<C, RNG> {
        &mut self.garbler
    }
}

impl<
        C: AbstractChannel,
        RNG: CryptoRng + Rng + SeedableRng<Seed = Block>,
        OT: OtSender<Msg = Block> + SemiHonest,
    > Garbler<C, RNG, OT>
{
    /// Make a new `Garbler`.
    pub fn new(mut channel: C, mut rng: RNG) -> Result<Self, TwopacError> {
        let ot = OT::init(&mut channel, &mut rng)?;

        let garbler = Gb::new(channel.clone(), RNG::from_seed(rng.gen()));
        Ok(Garbler {
            garbler,
            channel,
            ot,
            rng,
        })
    }

    /// Get a reference to the internal channel.
    pub fn get_channel(&mut self) -> &mut C {
        &mut self.channel
    }

    fn _evaluator_input(&mut self, delta: &Wire, modulus: &Modulus) -> (Wire, Vec<(Block, Block)>) {
        let len = match modulus {
            Modulus::Zq { q: qq } => f32::from(*qq).log(2.0).ceil() as u16,
            Modulus::GF4 { .. } => 4,
            Modulus::GF8 { .. } => 8,
            Modulus::GFk { k, .. } => (*k).into(), 
        };
        let mut wire = Wire::zero(modulus);
        let inputs = (0..len)
            .map(|i| {
                let zero = Wire::rand(&mut self.rng, modulus);
                let one = zero.plus(&delta);
                wire = wire.plus(&zero.cmul(1 << i));   // see 7.1 in paper for binary representation labels
                (zero.as_block(), one.as_block())
            })
            .collect::<Vec<(Block, Block)>>();
        (wire, inputs)
    }
}

impl<
        C: AbstractChannel,
        RNG: CryptoRng + Rng + SeedableRng<Seed = Block>,
        OT: OtSender<Msg = Block> + SemiHonest,
    > FancyInput for Garbler<C, RNG, OT>
{
    type Item = Wire;
    type Error = TwopacError;

    fn encode(&mut self, val: u16, modulus: &Modulus) -> Result<Wire, TwopacError> {
        let (mine, theirs) = self.garbler.encode_wire(val, modulus);
        self.garbler.send_wire(&theirs)?;
        self.channel.flush()?;
        Ok(mine)
    }

    fn encode_many(&mut self, vals: &[u16], moduli: &[Modulus]) -> Result<Vec<Wire>, TwopacError> {
        let ws = vals
            .iter()
            .zip(moduli.iter())
            .map(|(x, q)| {
                let (mine, theirs) = self.garbler.encode_wire(*x, q);
                self.garbler.send_wire(&theirs)?;
                Ok(mine)
            })
            .collect();
        self.channel.flush()?;
        ws
    }

    fn receive_many(&mut self, ms: &[Modulus]) -> Result<Vec<Wire>, TwopacError> {
        let n = ms.len();
        let lens = ms.iter().map(|q| q.bit_length());
        let mut wires = Vec::with_capacity(n);
        let mut inputs = Vec::with_capacity(lens.sum());

        for q in ms.iter() {
            let delta = self.garbler.delta(q);
            let (wire, input) = self._evaluator_input(&delta, q);
            wires.push(wire);
            for i in input.into_iter() {
                inputs.push(i);
            }
        }
        self.ot.send(&mut self.channel, &inputs, &mut self.rng)?;
        Ok(wires)
    }
}

impl<C: AbstractChannel, RNG: CryptoRng + Rng, OT> Fancy for Garbler<C, RNG, OT> {
    type Item = Wire;
    type Error = TwopacError;

    fn constant(&mut self, x: u16, modulus: &Modulus) -> Result<Self::Item, Self::Error> {
        self.garbler.constant(x, modulus).map_err(Self::Error::from)
    }

    fn add(&mut self, x: &Wire, y: &Wire) -> Result<Self::Item, Self::Error> {
        self.garbler.add(x, y).map_err(Self::Error::from)
    }

    fn sub(&mut self, x: &Wire, y: &Wire) -> Result<Self::Item, Self::Error> {
        self.garbler.sub(x, y).map_err(Self::Error::from)
    }

    fn cmul(&mut self, x: &Wire, c: u16) -> Result<Self::Item, Self::Error> {
        self.garbler.cmul(x, c).map_err(Self::Error::from)
    }

    fn mul(&mut self, x: &Wire, y: &Wire) -> Result<Self::Item, Self::Error> {
        self.garbler.mul(x, y).map_err(Self::Error::from)
    }

    fn proj(&mut self, x: &Wire, modulus: &Modulus, tt: Option<Vec<u16>>) -> Result<Self::Item, Self::Error> {
        self.garbler.proj(x, modulus, tt).map_err(Self::Error::from)
    }

    fn output(&mut self, x: &Self::Item) -> Result<Option<u16>, Self::Error> {
        self.garbler.output(x).map_err(Self::Error::from)
    }
}

impl<C: AbstractChannel, RNG: CryptoRng + Rng, OT> FancyReveal for Garbler<C, RNG, OT> {
    fn reveal(&mut self, x: &Self::Item) -> Result<u16, Self::Error> {
        self.garbler.reveal(x).map_err(Self::Error::from)
    }
}

impl<C, RNG, OT> SemiHonest for Garbler<C, RNG, OT> {}
