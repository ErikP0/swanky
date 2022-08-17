// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2022 COSIC
// See LICENSE for licensing information.

use crate::{
    errors::FancyError,
    fancy::{Fancy, HasModulus},
    Modulus,
};
use itertools::Itertools;
use std::ops::Index;

/// A collection of wires for the PHOTON permutation, useful for the garbled gadgets defined by `PhotonGadgets`.
// [[W; D]; D] is organized in column-major order
#[derive(Clone)]
pub struct PhotonState<W, const D: usize>([[W; D]; D]);

impl<W: Clone + HasModulus, const D: usize> PhotonState<W, D> {
    /// Create a new PhotonState 'matrix' from some wires.
    pub fn new(ws: [[W; D]; D]) -> PhotonState<W, D> {
        PhotonState(ws)
    }

    /// Create a new PhotonState matrix from a row-major ordered element array.
    // pub fn new(w_vec: Vec<W>) -> PhotonState<W, D> {
    //     assert_eq!(w_vec.len(), D*D);
    //     let mut ws: [[W; D]; D];

    //     for c in 0..D {
    //         ws[c] = w_vec[]
    //     }

    //     PhotonState(ws)
    // }

    /// Return the moduli of all the wires in the bundle.
    pub fn modulus(&self) -> Modulus {
        let mod0 = self.0[0][0].modulus();
        assert!(self.0.iter().all(|c| c.iter().all(|el| el.modulus() == mod0)));

        mod0
    }

    /// Extract the wires from this bundle.
    pub fn state_matrix(&self) -> &[[W; D] ; D] {
        &self.0
    }

    /// Get `d`, the dimension of the state matrix
    pub fn dim(&self) -> usize {
        self.0.len()
    }

    /// Extract a wire from the Bundle, returning it.
    pub fn extract(&mut self, (row_index, col_index): (usize, usize)) -> W {
        self.0[col_index][row_index].clone()
    }

    /// Insert a wire from the Bundle
    pub fn insert(&mut self, (row_index, col_index): (usize, usize), val: W) {
        self.0[col_index][row_index] = val;
    }

    /// Access the underlying iterator over the columns
    pub fn iter(&self) -> std::slice::Iter<[W; D]> {
        self.0.iter()
    }
}

impl<F: Fancy> PhotonGadgets for F {}

/// Extension trait for Fancy which provides Photon constructions
pub trait PhotonGadgets: Fancy {
    fn constant_state(
        &mut self,
        xs: &[u16],
        p: &Modulus,
    ) -> Result<PhotonState<Self::Item, >, Self::Error> {

    }
}