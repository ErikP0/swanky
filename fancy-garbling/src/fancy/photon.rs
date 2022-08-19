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
#[derive(Clone, PartialEq)]
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
        let mut row: Vec<W> = Vec::with_capacity(d);

        for r in 0..d {
            row = Vec::with_capacity(d);
            for c in 0..d {
                row.push(w_vec[c*d+r].clone());
            }
            ws.push(row);
        }

        PhotonState{state_matrix: ws, d}
    }

    /// Return the moduli of all the wires in the state matrix.
    pub fn modulus(&self) -> Modulus {
        let mod0 = self.state_matrix[0][0].modulus();
        if !self.state_matrix.iter().all(|c| c.iter().all(|el| el.modulus() == mod0)) {
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

    /// Extract a row of wires from the matrix, returning it.
    pub fn extract(&mut self, row_index: usize) -> Vec<W> {
        self.state_matrix.remove(row_index)
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

impl<W: Clone + HasModulus + std::fmt::Display> std::fmt::Display for PhotonState<W> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(fmt, "Modulus state: {} \n", self.modulus()).unwrap();
        for row in self.state_matrix() {
            for el in row.iter() {
                write!(fmt, "{} ", el).unwrap();
            }
            write!(fmt, "\n").unwrap();
        }
        Ok(())
    }
}

impl<F: Fancy> PhotonGadgets for F {}

/// Extension trait for Fancy which provides Photon constructions
pub trait PhotonGadgets: Fancy {
    
    /// Output the wires that make up a PhotonState.
    fn output_photon(&mut self, x: &PhotonState<Self::Item>) -> Result<Option<Vec<u16>>, Self::Error> {
        let d = x.dim();
        let state = x.state_matrix();

        let mut outputs = Vec::with_capacity(d*d);

        for c in 0..d {
            for r in 0..d {
                outputs.push(self.output(&state[r][c])?);
            }
        }

    
        Ok(outputs.into_iter().collect())
    }



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
        // println!("initial: {}", res_state);
        for round in 0..12 {
            self.AddConstants(&mut res_state, rcs[round], ics)?;
            // println!("addc: {}", res_state);

            self.SubCells(&mut res_state, sbox)?;
            // println!("subc: {}", res_state);

            self.ShiftRows(&mut res_state)?;
            // println!("shiftr: {}", res_state);

            self.MixColumnsSerial(&mut res_state, Z)?;
            // println!("mixc: {}", res_state);
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
            state.state_matrix[i][0] = self.add(&ic_add, &w_rc)?;
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
        // let mut res_state = state.state_matrix().clone();
        for row in state.state_matrix.iter_mut() {
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
                    tmp.push(state.state_matrix()[i][j].clone());
                }
                for j in 0..d { 
                    state.state_matrix[i][j] = tmp[(j+i)%d].clone();
                }
                tmp.clear();
            }
            Ok(state)
    }

    fn MixColumnsSerial<'a>(
        &'a mut self, 
        state: &'a mut PhotonState<Self::Item>,
        Z: &[u16],
    ) -> Result<&PhotonState<Self::Item>, Self::Error> {
        let d = state.dim();
        
        let mut last_row: Vec<Self::Item> = Vec::with_capacity(d);

        

        let mut el: Self::Item;
        for _ in 0..d {
            for i in 0..d {
                let mut sum = self.cmul(&state.state_matrix()[0][i], Z[0]).unwrap();
                for j in 1..d{
                    if Z[j]!=1 {
                    el = self.cmul(&state.state_matrix()[j][i],Z[j]).unwrap();
                    sum = self.add(&sum, &el).unwrap();    
                    }
                }
                last_row.push(sum);
            }
            state.extract(0);
            state.insert(0, last_row.clone());
            state.state_matrix.rotate_left(1);
            last_row.clear();
        }
    
        


        Ok(state)
    }
}



#[cfg(test)]
mod photon_test {
    use super::*;
    use crate::{
        fancy::FancyInput,
        dummy::Dummy,
    };

    #[test]
    fn photon80_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 4,
                                          0, 0, 0, 0, 1,
                                          0, 0 ,0, 0, 4,
                                          0, 0 ,0, 0, 1,
                                          0, 0 ,0, 1, 0);
        let sbox: &Vec<u16> = &vec![0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let Z: &[u16] = &[1, 2, 9, 9, 2];
        let ics = &[0, 1, 3, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = &Modulus::GF4 { p: 19 };
        let mut init_state = f.encode_photon(&init_state_m, 5, x4_x_1).unwrap();
        let full = init_state.clone();
        println!("init: {}", init_state);
        f.AddConstants(&mut init_state, 1, ics).unwrap();
        println!("addc: {}", init_state);
        f.SubCells(&mut init_state, sbox).unwrap();
        println!("subc: {}", init_state);
        f.ShiftRows(&mut init_state).unwrap();
        println!("shiftr: {}", init_state);
        f.MixColumnsSerial(&mut init_state, Z).unwrap();
        println!("mixc: {}", init_state);

        let res = f.PermutePHOTON(&full, &[0, 1, 3, 6, 4], sbox, Z).unwrap();
        println!("full: {}", res);
        let res_state_m: Vec<u16> = vec!(3, 6, 5, 6, 0xb,
                                          3, 2, 0xc, 5, 7,
                                          0xd, 9, 4, 0xc, 7,
                                          5, 0xb, 8, 0xe, 0,
                                          0xf, 9, 1, 7, 0xc);
        assert_eq!(res_state_m, f.output_photon(&res).unwrap().unwrap());

    }

    #[test]
    fn photon128_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 0, 2,
                                          0, 0, 0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 1,
                                          0, 0 ,0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 1,
                                          0, 0, 0, 0, 0, 0);
        let sbox: &Vec<u16> = &vec![0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let Z: &[u16] = &[1, 2, 8, 5, 8, 2];
        let ics = &[0, 1, 3, 7, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = &Modulus::GF4 { p: 19 };
        let mut init_state = f.encode_photon(&init_state_m, 6, x4_x_1).unwrap();
        let full = init_state.clone();
        println!("init: {}", init_state);
        f.AddConstants(&mut init_state, 1, ics).unwrap();
        println!("addc: {}", init_state);
        f.SubCells(&mut init_state, sbox).unwrap();
        println!("subc: {}", init_state);
        f.ShiftRows(&mut init_state).unwrap();
        println!("shiftr: {}", init_state);
        f.MixColumnsSerial(&mut init_state, Z).unwrap();
        println!("mixc: {}", init_state);

        let res = f.PermutePHOTON(&full, ics, sbox, Z).unwrap();
        println!("full: {}", res);
        let res_state_m: Vec<u16> = vec!(9, 0xe, 6, 0xe, 6, 8,
                                         5, 2, 3, 0xb, 2, 0xd,
                                         0xf, 2, 2, 4, 5, 0,
                                         0xc, 0xa, 0xd, 0xe, 9, 3,
                                         3, 2, 6, 0, 2, 2,
                                         0xc, 0xa, 0xf, 0xb, 0xd, 9);
        assert_eq!(res_state_m, f.output_photon(&res).unwrap().unwrap());

    }
}