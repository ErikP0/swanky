// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2022 COSIC
// See LICENSE for licensing information.

use crate::{
    fancy::{Fancy, HasModulus},
    Modulus,
};
use itertools::Itertools;


/// A collection of wires for the PHOTON permutation, useful for the garbled gadgets defined by `PhotonGadgets`.
// [[W; D]; D] is organized in row-major order
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
        let mut ws: Vec<Vec<W>> = Vec::with_capacity(d*d);

        for r in 0..d {
            for c in 0..d {
                ws[r][c] = w_vec[r*d + c].clone();
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

    /// Insert a row in the matrix at given `row_index`
    pub fn insert(&mut self, row_index: usize, row: Vec<W>) {
        self.state_matrix.insert(row_index, row);
    }

    /// Access the underlying iterator over the rows
    pub fn iter(&self) -> std::slice::Iter<Vec<W>> {
        self.state_matrix.iter()
    }
}

impl<F: Fancy> PhotonGadgets for F {}

/// Extension trait for Fancy which provides Photon constructions
pub trait PhotonGadgets: Fancy {
    fn PermutePHOTON (
        &mut self,
        state: &PhotonState<Self::Item>,
        ics: &[u16],
        sbox: &Vec<u16>,
        Z: &[u16],
    ) -> Result<PhotonState<Self::Item>, Self::Error> {
        const rcs: [u16; 12] = [1, 3, 7, 14, 13, 11, 6, 12,  9, 2, 5, 10];

        let d = state.dim();
        debug_assert_eq!(ics.len(), d);
        debug_assert_eq!(sbox.len(), state.size());
        debug_assert_eq!(Z.len(), d);

        let mut res_state = state.clone();
        for round in 0..12 {
            self.AddConstants(&mut res_state, rcs[round], ics)?;

            self.SubCells(&mut res_state, sbox)?;

            self.MixColumnsSerial(&mut res_state, Z)?;
        }

        Ok(res_state)
    }

    fn AddConstants<'a> (
        &mut self,
        state: &'a mut PhotonState<Self::Item>,
        rc: u16,
        ics: &[u16] //hardcode!
    ) -> Result<&'a PhotonState<Self::Item>, Self::Error> {
        debug_assert_eq!(ics.len(), state.dim());
        let w_ics = ics
        .iter()
        .map(|ic| self.constant(*ic, &state.modulus()).unwrap())
        .collect_vec();
        let w_rc = self.constant(rc, &state.modulus())?;
        let d = state.dim();

        let mut ic_add: Self::Item;
        for i in 0..d {
            ic_add = self.add(&state.state_matrix()[i][0], &w_ics[i])?;
            state.state_matrix()[i][0] = self.add(&ic_add, &w_rc)?;
        }

        Ok(state)
    }

    fn SubCells<'a> (
        &mut self,
        state: &'a mut PhotonState<Self::Item>,
        sbox: &Vec<u16>,
    ) -> Result<&'a PhotonState<Self::Item>, Self::Error> {
        debug_assert_eq!(state.size(), sbox.len(), "Sbox has incorrect dimensions");

        let state_mod = state.modulus();
        let mut res_state = state.state_matrix().clone();
        for row in res_state.iter_mut() {
            for el in row.iter_mut() {
                *el = self.proj(el, &state_mod, Some(sbox.clone())).unwrap();
            }
        }

        Ok(state)

    }

    fn ShiftRows<'a>(
        &mut self, 
        state: &'a mut PhotonState<Self::Item>
    ) -> Result<&'a PhotonState<Self::Item>, Self::Error> { 
            let d = state.dim();
            let mut tmp: Vec<Self::Item> = Vec::with_capacity(d);
            for i in 1..d {
                for j in 0..d {
                    tmp[j] = state.state_matrix()[i][j].clone();
                }
                for j in 0..d { 
                    state.state_matrix[i][j] = tmp[(j+i)%d].clone();
                }
            }
            Ok(state)
    }

    fn MixColumnsSerial(
        &mut self, 
        state: &mut PhotonState<Self::Item>,
        Z: &[u16],
    ) -> Result<&PhotonState<Self::Item>, Self::Error> {
        let d = state.dim();
        
        let last_row: Vec<Self::Item> = Vec::with_capacity(d);

        

        let mut res: Self::Item;
        for i in 0..d {
            let mut sum = self.cmul(&state.state_matrix()[0][i], Z[0]).unwrap();
            for j in 1..d{
                if Z[j]!=1 {
                   sum = self.add(&sum,&self.cmul(&state.state_matrix()[j][i],Z[j]).unwrap()).unwrap();    
                }
            }
            last_row[i] = sum;
        }

        state.insert(0, last_row);
        state.state_matrix.rotate_left(1);


        Ok(state)
    }
}