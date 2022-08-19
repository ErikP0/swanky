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
                    } else {
                        el = state.state_matrix()[j][i].clone();
                    }
                sum = self.add(&sum, &el).unwrap();
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


    #[test]
    fn photon160_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0, 2,
                                          0, 0 ,0, 0, 0, 0, 8,
                                          0, 0 ,0, 0, 0, 0, 2,
                                          0, 0 ,0, 0, 0, 0, 4,
                                          0, 0, 0, 0, 0, 0, 2,
                                          0, 0, 0, 0, 0, 0, 4);
        let sbox: &Vec<u16> = &vec![0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let Z = &[1, 4, 6, 1, 1, 6, 4];
        let ics = &[0, 1, 2, 5, 3, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = &Modulus::GF4 { p: 19 };
        let mut init_state = f.encode_photon(&init_state_m, 7, x4_x_1).unwrap();
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
        let res_state_m: Vec<u16> = vec!(1, 0xd, 0xe, 0xb, 0xf, 0xe, 3,
                                         0xf, 0xd, 0xc, 6, 6, 9, 0xa,
                                         0, 0, 0xf, 6, 4, 0, 9,
                                         0xd, 0xa, 5, 0xe, 4, 2, 0xd,
                                         4, 3, 0xb, 0, 0xc, 0, 0xe,
                                         0xa, 1, 6, 0xc, 0xe, 0xf, 7,
                                         1, 0xd, 9, 8, 0xe, 4, 4);
        assert_eq!(res_state_m, f.output_photon(&res).unwrap().unwrap());

    }


    #[test]
    fn photon192_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0, 3,
                                          0, 0 ,0, 0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 0, 4,
                                          0, 0, 0, 0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0, 4);
        let sbox: &Vec<u16> = &vec![0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let Z = &[1, 4, 6, 1, 1, 6, 4];
        let ics = &[0, 1, 2, 5, 3, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = &Modulus::GF4 { p: 19 };
        let mut init_state = f.encode_photon(&init_state_m, 7, x4_x_1).unwrap();
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
        let res_state_m: Vec<u16> = vec!(0xe, 0xd, 4, 0xe, 2, 9, 3,
                                         7, 6, 0xc, 8, 8, 0, 8,
                                         0xa, 7, 1, 1, 0xf, 7, 3,
                                         0xc, 6, 0xd, 9, 0xb, 0xc, 0xa,
                                         8, 3, 0xc, 1, 5, 0xc, 1,
                                         0xd, 3, 6, 2, 7, 9, 1,
                                         0xf, 0xb, 0xb, 4, 1, 0xb, 7);
        assert_eq!(res_state_m, f.output_photon(&res).unwrap().unwrap());

    }


    #[test]
    fn photon224_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 0, 0, 3,
                                          0, 0 ,0, 0, 0, 0, 0, 8,
                                          0, 0 ,0, 0, 0, 0, 0, 2,
                                          0, 0, 0, 0, 0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0, 0, 2,
                                          0, 0, 0, 0, 0, 0, 0, 0);
        let sbox: &Vec<u16> = &vec![0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let Z = &[2, 4, 2, 11, 2, 8, 5, 6];
        let ics = &[0, 1, 3, 7, 15, 14, 12, 8];
        let mut f = Dummy::new();
        let x4_x_1 = &Modulus::GF4 { p: 19 };
        let mut init_state = f.encode_photon(&init_state_m, 8, x4_x_1).unwrap();
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
        let res_state_m: Vec<u16> = vec!(1, 9, 8, 0, 0xc, 0xa, 7, 8,
                                         7, 0xc, 0xd, 0, 6, 0xf, 4, 9,
                                         3, 0xf, 3, 0xe, 2, 4, 8, 1,
                                         0, 2, 0xd, 2, 9, 1, 3, 6,
                                         4, 6, 9, 7, 0xb, 0xf, 0xf, 0xb,
                                         2, 0xe, 0xc, 0xb, 3, 1, 0xc, 8,
                                         4, 1, 0xf, 0xd, 0xd, 0xc, 0xc, 2,
                                         2, 0, 9, 0xc, 1, 0xb, 0, 0xc);
        assert_eq!(res_state_m, f.output_photon(&res).unwrap().unwrap());

    }


    #[test]
    fn photon256_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 0x40,
                                          0, 0 ,0, 0, 0, 0x20,
                                          0, 0, 0, 0, 0, 0x20);
        let sbox: &Vec<u16> = &vec![0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
                                    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
                                    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
                                    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
                                    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
                                    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
                                    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
                                    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
                                    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
                                    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
                                    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
                                    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
                                    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
                                    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
                                    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16];
        let Z: &[u16] = &[2, 3, 1, 2, 1, 4];
        let ics = &[0, 1, 3, 7, 6, 4];
        let mut f = Dummy::new();
        let x8_x4_x3_x_1 = &Modulus::GF8 { p: 283 };
        let mut init_state = f.encode_photon(&init_state_m, 6, x8_x4_x3_x_1).unwrap();
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
        let res_state_m: Vec<u16> = vec!(0x4D, 0xBD, 0x90, 0x36, 0x1C, 0xB5, 
                                         0xE0, 0x9E, 0x5C, 0x38, 0xA9, 0xC9, 
                                         0xE9, 0xD5, 0x66, 0x08, 0xCF, 0x52, 
                                         0xCB, 0x6B, 0xC8, 0x8B, 0x93, 0x16, 
                                         0xE8, 0xC2, 0xC0, 0x69, 0x25, 0xF7, 
                                         0x18, 0xCC, 0x62, 0x9C, 0xAE, 0x79); 
        assert_eq!(res_state_m, f.output_photon(&res).unwrap().unwrap());

    }
}