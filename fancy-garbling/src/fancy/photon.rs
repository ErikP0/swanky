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
use std::{ops::Index, convert::TryInto};

/// A collection of wires for the PHOTON permutation, useful for the garbled gadgets defined by `PhotonGadgets`.
// [[W; D]; D] is organized in column-major order
#[derive(Clone)]
pub struct PhotonState<W, const D: usize>([[W; D]; D]);

impl<W: Clone + HasModulus, const D: usize> PhotonState<W, D> {
    /// Create a new PhotonState 'matrix' from some wires.
    pub fn new(ws: [[W; D]; D]) -> PhotonState<W, D> {
        PhotonState(ws)
    }

    /// Create a new PhotonState matrix from an ordered element array.
    pub fn from_vec(w_vec: Vec<W>) -> PhotonState<W, D> {
        assert_eq!(w_vec.len(), D*D);
        let mut ws: [[W; D]; D];

        for c in 0..D {
            for r in 0..D {
                ws[c][r] = w_vec[c*D + r];
            }
        }

        PhotonState(ws)
    }

    /// Return the moduli of all the wires in the state matrix.
    pub fn modulus(&self) -> Modulus {
        let mod0 = self.0[0][0].modulus();
        if self.0.iter().all(|c| c.iter().all(|el| el.modulus() == mod0)) {
            panic!("Not all elements in the state matrix have the same modulus!");
        }

        mod0
    }

    /// Get `state_matrix`, the underlying structure of PhotonState.
    pub fn state_matrix(&self) -> &[[W; D] ; D] {
        &self.0
    }

    /// Get `d`, the dimension of the state matrix
    pub fn dim(&self) -> usize {
        self.0.len()
    }

    /// Extract a wire from the matrix, returning it.
    pub fn extract(&mut self, (row_index, col_index): (usize, usize)) -> W {
        self.0[col_index][row_index].clone()
    }

    /// Insert a column in the matrix at given `col_index`
    pub fn insert(&mut self, col_index: usize, col: [W; D]) {
        self.0[col_index] = col;
    }

    /// Access the underlying iterator over the columns
    pub fn iter(&self) -> std::slice::Iter<[W; D]> {
        self.0.iter()
    }
}

impl<F: Fancy, const D: usize> PhotonGadgets<D> for F {}

/// Extension trait for Fancy which provides Photon constructions
pub trait PhotonGadgets<const D: usize>: Fancy {
    fn AddConstants (
        &mut self,
        state: &mut PhotonState<Self::Item, D>,
        rc: u16,
        ics: &[u16; D] //hardcode!
    ) -> Result<PhotonState<Self::Item, D>, Self::Error> {
        let w_ics = ics
        .iter()
        .map(|ic| self.constant(*ic, &state.modulus()).unwrap())
        .collect_vec();
        let w_rc = self.constant(rc, &state.modulus()).unwrap();

        // HOW WITH ITERATOR? DEBUG TRAIT (try_into())?
        // let first_col: [Self::Item; D] = state.state_matrix()[0]
        // .iter()
        // .zip(w_ics.iter())
        // .map(|(cell, iconst)| {
        //     let ic_add = self.add(cell, iconst).unwrap();
        //     self.add(&ic_add, &w_rc).unwrap()
        // })
        // .collect_vec().into();

        // let mut res_state = state.state_matrix();
        let ic_add;
        for i in 0..D {
            ic_add = self.add(&state.state_matrix()[0][i], &w_ics[i]).unwrap();
            state.state_matrix()[0][i] = self.add(&ic_add, &w_rc).unwrap();
        };
        Ok(*state)
    }

    fn SubCells (
        &mut self,
        sbox: Vec<u16>
    ) -> Result<PhotonState<Self::Item, D>, Self::Error> {
        
    }




    fn ShiftRows(
        &mut self, 
        state: &mut PhotonState<Self::Item, D>) { 
            let tmp: [Self::Item; D];
            for i in 1..D {
                for j in 0..D {
                    tmp[j] = state.state_matrix()[j][i];
                }
                for j in 0..D { 
                    state.state_matrix()[j][i] = tmp[(j+i)%D];
                }
            }
    }
}