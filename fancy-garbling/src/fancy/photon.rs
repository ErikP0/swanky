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
use serde_json::de;
use std::{ops::Index, convert::TryInto};

/// A collection of wires for the PHOTON permutation, useful for the garbled gadgets defined by `PhotonGadgets`.
// [[W; D]; D] is organized in column-major order
#[derive(Clone)]
pub struct PhotonState<W> {
    state_matrix: Vec<Vec<W>>,
    d: usize,
}

impl<W> PhotonState<W> 
    where
        W: Clone + HasModulus
    {
    /// Create a new PhotonState 'matrix' from some wires.
    pub fn new(ws: Vec<Vec<W>>, d: usize) -> PhotonState<W> {
        debug_assert_eq!(ws.len(), d);
        debug_assert!(ws.iter().all(|c| c.len() == d));

        PhotonState{state_matrix: ws, d}
    }

    /// Create a new PhotonState matrix from an ordered element array.
    pub fn from_vec(w_vec: Vec<W>, d: usize) -> PhotonState<W> {
        assert_eq!(w_vec.len(), d*d);
        let mut ws: Vec<Vec<W>>;

        for c in 0..d {
            for r in 0..d {
                ws[c][r] = w_vec[c*d + r];
            }
        }

        PhotonState{state_matrix: ws, d}
    }

    /// Return the moduli of all the wires in the state matrix.
    pub fn modulus(&self) -> Modulus {
        let mod0 = self.state_matrix[0][0].modulus();
        if self.state_matrix.iter().all(|c| c.iter().all(|el| el.modulus() == mod0)) {
            panic!("Not all elements in the state matrix have the same modulus!");
        }

        mod0
    }

    // Return the size of the filed of all the wires in the state matrix.
    pub fn size(&self) -> usize {
        self.modulus().size().into()

    }
    /// Get `state_matrix`, the underlying structure of PhotonState.
    pub fn state_matrix(&self) -> &Vec<Vec<W>> {
        &self.state_matrix
    }

    /// Get `d`, the dimension of the state matrix
    pub fn dim(&self) -> usize {
        self.state_matrix.len()
    }

    /// Extract a wire from the matrix, returning it.
    pub fn extract(&mut self, (row_index, col_index): (usize, usize)) -> W {
        self.state_matrix[col_index][row_index].clone()
    }

    /// Insert a column in the matrix at given `col_index`
    pub fn insert(&mut self, col_index: usize, col: Vec<W>) {
        self.state_matrix.insert(col_index, col);
    }

    /// Access the underlying iterator over the columns
    pub fn iter(&self) -> std::slice::Iter<Vec<W>> {
        self.state_matrix.iter()
    }
}

impl<F: Fancy> PhotonGadgets for F {}

/// Extension trait for Fancy which provides Photon constructions
pub trait PhotonGadgets: Fancy {
    fn AddConstants (
        &mut self,
        state: &mut PhotonState<Self::Item>,
        rc: u16,
        ics: &[u16] //hardcode!
    ) -> Result<&PhotonState<Self::Item>, Self::Error> {
        let w_ics = ics
        .iter()
        .map(|ic| self.constant(*ic, &state.modulus()).unwrap())
        .collect_vec();
        let w_rc = self.constant(rc, &state.modulus())?;
        let d = state.dim();

        let first_col = state.state_matrix()[0]
        .iter()
        .zip(w_ics.iter())
        .map(|(cell, iconst)| {
            let ic_add = self.add(cell, iconst).unwrap();
            self.add(&ic_add, &w_rc).unwrap()
        })
        .collect_vec();

        state.state_matrix().insert(0, first_col);

        Ok(state)
    }

    fn SubCells (
        &mut self,
        state: &mut PhotonState<Self::Item>,
        sbox: Vec<u16>,
    ) -> Result<&PhotonState<Self::Item>, Self::Error> {
        debug_assert_eq!(state.size(), sbox.len(), "Sbox has incorrect dimensions");

        let state_mod = state.modulus();
        let mut res_state = state.state_matrix().clone();
        for col in res_state.iter_mut() {
            for el in col.iter_mut() {
                *el = self.proj(el, &state_mod, Some(sbox)).unwrap();
            }
        }

        Ok(state)

    }

    fn ShiftRows(
        &mut self, 
        state: &mut PhotonState<Self::Item>
    ) -> Result<&PhotonState<Self::Item>, Self::Error> { 
            let d = state.dim();
            let tmp: Vec<Self::Item>;
            for i in 1..d {
                for j in 0..d {
                    tmp[j] = state.state_matrix()[j][i];
                }
                for j in 0..d { 
                    state.state_matrix()[j][i] = tmp[(j+i)%d];
                }
            }
            Ok(state)
    }

    fn MixColumnsSerial(
        &mut self, 
        state: &mut PhotonState<Self::Item>,
        Z: &[u16],
    ) -> Result<&PhotonState<Self::Item>, Self::Error> {
        
    }
}