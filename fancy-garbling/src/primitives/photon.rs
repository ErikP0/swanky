// -*- mode: rust; -*-
//
// This file is part of `fancy-garbling`.
// Copyright © 2022 COSIC 
// Elias Wils & Robbe Vermeiren
// See LICENSE for licensing information.

use crate::{
    fancy::{Fancy, HasModulus},
    Modulus,
};
use itertools::Itertools;

const SBOX_PRE: &[u16] =  &[0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
const SBOX_PRE_INV: &[u16] = &[0x5, 0xe, 0xf, 0x8, 0xc, 0x1, 0x2, 0xd, 0xb, 0x4, 0x6, 0x3, 0x0, 0x7, 0x9, 0xa];
const SBOX_AES: &[u16] =  &[0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
                            0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
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
const SBOX_AES_INV: &[u16] =  &[0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
                            0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
                                0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
                                0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
                                0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
                                0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
                                0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
                                0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
                                0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
                                0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
                                0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
                                0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
                                0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
                                0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
                                0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
                                0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d];


/// A collection of wires for the PHOTON permutation, useful for the garbled gadgets defined by `PhotonGadgets`.
// [[W; D]; D] is =organized in row-major order
#[derive(Clone, PartialEq)]
struct PhotonState<F: Fancy> {
    state_matrix: Vec<Vec<F::Item>>,
    d: usize,
}

impl<F: Fancy> PhotonState<F> 
    {
    /// Create a new PhotonState matrix from an ordered element array.
    pub fn new(w_vec: &Vec<F::Item>, d: usize) -> Self {
        assert_eq!(w_vec.len(), d*d);
        let mut ws: Vec<Vec<F::Item>> = Vec::with_capacity(d*d);
        let mut row: Vec<F::Item>;

        for r in 0..d {
            row = Vec::with_capacity(d);
            for c in 0..d {
                row.push(w_vec[c*d+r].clone());
            }
            ws.push(row);
        }

        PhotonState{state_matrix: ws, d}
    }
    /// Create a new PhotonState 'matrix' from some wires.
    pub fn from_matrix(ws: Vec<Vec<F::Item>>, d: usize) -> Self {
        debug_assert_eq!(ws.len(), d);
        debug_assert!(ws.iter().all(|c| c.len() == d));

        PhotonState{state_matrix: ws, d}
    }

    /// Return the moduli of all the wires in the state matrix.
    fn modulus(&self) -> Modulus {
        let mod0 = self.state_matrix[0][0].modulus();
        if !self.state_matrix.iter().all(|c| c.iter().all(|el| el.modulus() == mod0)) {
            panic!("Not all elements in the state matrix have the same modulus!");
        }
        mod0
    }
    /// Return copy of `state_matrix`
    fn state_matrix(&self) -> Vec<Vec<F::Item>> {
        self.state_matrix.clone()
    }

    fn size(&self) -> usize {
        self.modulus().size().into()
    }

    /// Get `d`, the dimension of the state matrix
    fn dim(&self) -> usize {
        self.state_matrix.len()
    }

    /// Extract a row of wires from the matrix, returning it.
    fn extract(&mut self, row_index: usize) -> Vec<F::Item> {
        self.state_matrix.remove(row_index)
    }

    /// Insert a row in the matrix at given `row_index`
    fn insert(&mut self, row_index: usize, row: Vec<F::Item>) {
        self.state_matrix.insert(row_index, row);
    }

    /// Access the underlying iterator over the rows
    fn iter(&self) -> std::slice::Iter<Vec<F::Item>> {
        self.state_matrix.iter()
    }

    /// Output the wires that make up a PhotonState.
    pub fn output_photon(&self) -> Result<Vec<F::Item>, F::Error> {
        let d = self.dim();

        let mut outputs = Vec::with_capacity(d*d);

        for c in 0..d {
            for r in 0..d {
                outputs.push(self.state_matrix[r][c].clone());
            }
        }
    
        Ok(outputs.into_iter().collect())
    }
    
    /// 
    fn PermutePHOTON (
        &mut self,
        f: &mut F,
        ics: &[u16],
        sbox: &'static [u16],
        Z: &[u16],
    ) -> Result<(), F::Error> {
        const RCS: [u16; 12] = [1, 3, 7, 14, 13, 11, 6, 12,  9, 2, 5, 10];

        let d = self.dim();
        debug_assert_eq!(ics.len(), d);
        debug_assert_eq!(sbox.len(), self.size());
        debug_assert_eq!(Z.len(), d);

        // let mut res_state = self.clone();
        // println!("initial: {}", res_state);
        for round in 0..12 {
            self.AddConstants(f, RCS[round], ics)?;
            // println!("addc: {}", res_state);

            self.SubCells(f, sbox)?;
            // println!("subc: {}", res_state);

            self.ShiftRows()?;
            // println!("shiftr: {}", res_state);

            self.MixColumnsSerial(f, Z)?;
            // println!("mixc: {}", res_state);
        }

        Ok(())
    }

    fn PermutePHOTONInverse (
        &mut self, 
        f: &mut F,
        ics: &[u16], 
        sbox: &'static [u16], 
        Z: &[u16]
    ) -> Result<(), F::Error> {
        const RCS: [u16; 12] = [10,5,2,9,12,6,11,13,14,7,3,1];
        
        let d = self.dim();
        debug_assert_eq!(ics.len(), d);
        debug_assert_eq!(sbox.len(), self.size());
        debug_assert_eq!(Z.len(), d);

        // let mut res_state = self.clone();
        // println!("initial: {}", res_state);
        for round in 0..12 {
            self.MixColumnsSerialInv(f, Z)?;       // Input Z coefficients as in the forward permutation
            // println!("mixc: {}", self);

            self.ShiftRowsInv()?;
            // println!("shiftr: {}", self);

            self.SubCells(f, sbox)?; // Just use the same function as in the forward permutation with the inverse SBOX
            // println!("subc: {}", self);

            self.AddConstants(f, RCS[round], ics)?; // Inverse is also just addition because we do arithmetic in GF(2^k)
            // println!("addc: {}", self);
        }

        Ok(())

    }

    fn AddConstants (
        &mut self,
        f: &mut F,
        rc: u16,
        ics: &[u16] //hardcode!
    ) -> Result<(), F::Error> {
        debug_assert_eq!(ics.len(), self.dim());
        let w_ics = ics
        .iter()
        .map(|ic| f.constant(*ic, &self.modulus()).unwrap())
        .collect_vec();
        let w_rc = f.constant(rc, &self.modulus())?;
        let d = self.dim();

        let mut ic_add: F::Item;
        for i in 0..d {
            ic_add = f.add(&self.state_matrix[i][0], &w_ics[i])?;
            self.state_matrix[i][0] = f.add(&ic_add, &w_rc)?;
        }

        Ok(())
    }

    fn SubCells (
        &mut self,
        f: &mut F,
        sbox: &'static [u16],
    ) -> Result<(), F::Error> {
        debug_assert_eq!(self.size(), sbox.len(), "Sbox has incorrect dimensions");

        let state_mod = self.modulus();
        // let mut res_state = state.state_matrix().clone();
        for row in self.state_matrix.iter_mut() {
            for el in row.iter_mut() {
                *el = f.proj(el, &state_mod, Some(sbox.to_vec())).unwrap();
            }
        }

        Ok(())

    }


    fn ShiftRows(
        &mut self, 
    ) -> Result<(), F::Error> { 
            let d = self.dim();
            let mut tmp: Vec<F::Item> = Vec::with_capacity(d);
            for i in 1..d {
                for j in 0..d {
                    tmp.push(self.state_matrix[i][j].clone());
                }
                for j in 0..d { 
                    self.state_matrix[i][j] = tmp[(j+i)%d].clone();
                }
                tmp.clear();
            }
            Ok(())
    }

    fn ShiftRowsInv(
        &mut self, 
    ) -> Result<(), F::Error> { 
            let d = self.dim();
            let mut tmp: Vec<F::Item> = Vec::with_capacity(d);
            for i in 1..d {
                for j in 0..d {
                    tmp.push(self.state_matrix[i][j].clone());
                }
                for j in 0..d { 
                    self.state_matrix[i][j] = tmp[(j+d-i)%d].clone();
                }
                tmp.clear();
            }
            Ok(())
    }

    fn MixColumnsSerial(
        &mut self, 
        f: &mut F,
        Z: &[u16],
    ) -> Result<(), F::Error> {
        let d = self.dim();
        
        let mut last_row: Vec<F::Item> = Vec::with_capacity(d);

        let mut el: F::Item;
        for _ in 0..d {
            for i in 0..d {
                let mut sum = f.cmul(&self.state_matrix[0][i], Z[0]).unwrap();
                for j in 1..d{
                    if Z[j]!=1 {
                        el = f.cmul(&self.state_matrix[j][i],Z[j]).unwrap();
                    } else {
                        el = self.state_matrix[j][i].clone();
                    }
                sum = f.add(&sum, &el).unwrap();
                }
                last_row.push(sum);
            }
            self.extract(0);
            self.insert(0, last_row.clone());
            self.state_matrix.rotate_left(1);
            last_row.clear();
        }

        Ok(())
    }
    fn MixColumnsSerialInv(
        &mut self,
        f: &mut F,
        Z: &[u16]
    ) -> Result<(),F::Error> {
        let Zrev: Vec<u16> = Z.to_vec().into_iter().rev().collect_vec();
        

        let d = self.dim();
        
        let mut first_row: Vec<F::Item> = Vec::with_capacity(d);

        let mut el: F::Item;
        for _ in 0..d {
            for i in 0..d {
                let mut sum = f.cmul(&self.state_matrix[0][i], Zrev[0]).unwrap();
                for j in 1..d{
                    if Z[j]!=1 {
                        el = f.cmul(&self.state_matrix[j][i],Zrev[j]).unwrap();
                    } else {
                        el = self.state_matrix[j][i].clone();
                    }
                sum = f.add(&sum, &el).unwrap();
                }
                first_row.push(sum);
            }
            self.extract(d-1);
            self.insert(d-1, first_row.clone());
            self.state_matrix.rotate_right(1);
            first_row.clear();
        }


        Ok(())
    }
}

impl<F: Fancy> std::fmt::Display for PhotonState<F>
    where F::Item: std::fmt::Display {
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

/// Extension trait for Fancy which provides Photon constructions
pub trait PhotonGadgets: Fancy {
    fn photon_100(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error>;

    fn photon_144(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error>;

    fn photon_196(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error>;

    fn photon_256(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error>;

    fn photon_288(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error>;   
    
    fn photon_custom(&mut self, input: &Vec<Self::Item>, d: usize, ics: &[u16], Zi: &[u16], in_GF4: bool) -> Result<Vec<Self::Item>, Self::Error>;   

    fn photon_custom_inv(&mut self, input: &Vec<Self::Item>, d: usize, ics: &[u16], Zi: &[u16], in_GF4: bool) -> Result<Vec<Self::Item>, Self::Error>;   
}

impl<F: Fancy> PhotonGadgets for F {
    fn photon_100(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state: PhotonState<F> = PhotonState::new(input, 5);
        state.PermutePHOTON(self, &[0,1,3,6,4], SBOX_PRE, &[1,2,9,9,2])?;
        Ok(state.output_photon().unwrap())
    }

    fn photon_144(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state: PhotonState<F> = PhotonState::new(input, 6);
        state.PermutePHOTON(self, &[0, 1, 3, 7, 6, 4], SBOX_PRE, &[1, 2, 8, 5, 8, 2])?;
        Ok(state.output_photon().unwrap())
    }

    fn photon_196(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state: PhotonState<F> = PhotonState::new(input, 7);
        state.PermutePHOTON(self, &[0, 1, 2, 5, 3, 6, 4], SBOX_PRE, &[1, 4, 6, 1, 1, 6, 4])?;
        Ok(state.output_photon().unwrap())
    }

    fn photon_256(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state: PhotonState<F> = PhotonState::new(input, 8);
        state.PermutePHOTON(self, &[0, 1, 3, 7, 15, 14, 12, 8], SBOX_PRE, &[2, 4, 2, 11, 2, 8, 5, 6])?;
        Ok(state.output_photon().unwrap())
    }

    fn photon_288(&mut self, input: &Vec<Self::Item>) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state: PhotonState<F> = PhotonState::new(input, 6);
        state.PermutePHOTON(self, &[0, 1, 3, 7, 6, 4], SBOX_AES, &[2, 3, 1, 2, 1, 4])?;
        Ok(state.output_photon().unwrap())
    }

    fn photon_custom(&mut self, input: &Vec<Self::Item>, d: usize, ics: &[u16], Zi: &[u16], in_GF4: bool) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state: PhotonState<F> = PhotonState::new(input, d);
        let sbox = if in_GF4 {SBOX_PRE} else {SBOX_AES};
        state.PermutePHOTON(self, ics, sbox, Zi)?;
        Ok(state.output_photon().unwrap())
    }

    fn photon_custom_inv(&mut self, input: &Vec<Self::Item>, d: usize, ics: &[u16], Zi: &[u16], in_GF4: bool) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state: PhotonState<F> = PhotonState::new(input, d);
        let sbox = if in_GF4 {SBOX_PRE_INV} else {SBOX_AES_INV};
        state.PermutePHOTONInverse(self, ics, sbox, Zi)?;
        Ok(state.output_photon().unwrap())
    }

}


#[cfg(test)]
mod photon_test {
    use ocelot::ot::{ChouOrlandiReceiver, ChouOrlandiSender};
    use rand::Rng;
    use scuttlebutt::{UnixChannel, AesRng, unix_channel_pair};

    use super::*;
    use crate::{
        fancy::FancyInput,
        dummy::{Dummy, DummyVal}, circuit::CircuitBuilder, 
        Wire, Evaluator, Garbler, util::RngExt, twopac,
    };
    #[test]
    fn photon80_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 4,
                                          0, 0, 0, 0, 1,
                                          0, 0 ,0, 0, 4,
                                          0, 0 ,0, 0, 1,
                                          0, 0 ,0, 1, 0);
        // let Z: &[u16] = &[1, 2, 9, 9, 2];
        // let ics = &[0, 1, 3, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = Modulus::GF4 { p: 19 };
        let init_state_enc = f.encode_many(&init_state_m, &[x4_x_1; 25]).unwrap();
        let state = f.photon_100(&init_state_enc).unwrap();
        // println!("full: {}", res);
        let res_state_m: Vec<u16> = vec!(3, 6, 5, 6, 0xb,
                                          3, 2, 0xc, 5, 7,
                                          0xd, 9, 4, 0xc, 7,
                                          5, 0xb, 8, 0xe, 0,
                                          0xf, 9, 1, 7, 0xc);
        assert_eq!(res_state_m, state.into_iter().map(|w| w.val()).collect_vec());
    }

    #[test]
    fn photon128_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 0, 2,
                                          0, 0, 0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 1,
                                          0, 0 ,0, 0, 0, 0,
                                          0, 0 ,0, 0, 0, 1,
                                          0, 0, 0, 0, 0, 0);
        // let Z: &[u16] = &[1, 2, 8, 5, 8, 2];
        // let ics = &[0, 1, 3, 7, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = Modulus::GF4 { p: 19 };
        let init_state_enc = f.encode_many(&init_state_m, &[x4_x_1; 36]).unwrap();
        
        let state = f.photon_144(&init_state_enc).unwrap();
        let res_state_m: Vec<u16> = vec!(9, 0xe, 6, 0xe, 6, 8,
                                         5, 2, 3, 0xb, 2, 0xd,
                                         0xf, 2, 2, 4, 5, 0,
                                         0xc, 0xa, 0xd, 0xe, 9, 3,
                                         3, 2, 6, 0, 2, 2,
                                         0xc, 0xa, 0xf, 0xb, 0xd, 9);

        assert_eq!(res_state_m, state.into_iter().map(|w| w.val()).collect_vec());

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
        // let Z = &[1, 4, 6, 1, 1, 6, 4];
        // let ics = &[0, 1, 2, 5, 3, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = Modulus::GF4 { p: 19 };
        let init_state_enc = f.encode_many(&init_state_m, &[x4_x_1; 49]).unwrap();
        
        let state = f.photon_196(&init_state_enc).unwrap();
        // println!("full: {}", res);
        let res_state_m: Vec<u16> = vec!(1, 0xd, 0xe, 0xb, 0xf, 0xe, 3,
                                         0xf, 0xd, 0xc, 6, 6, 9, 0xa,
                                         0, 0, 0xf, 6, 4, 0, 9,
                                         0xd, 0xa, 5, 0xe, 4, 2, 0xd,
                                         4, 3, 0xb, 0, 0xc, 0, 0xe,
                                         0xa, 1, 6, 0xc, 0xe, 0xf, 7,
                                         1, 0xd, 9, 8, 0xe, 4, 4);
        assert_eq!(res_state_m, state.into_iter().map(|w| w.val()).collect_vec());

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
        // let Z = &[1, 4, 6, 1, 1, 6, 4];
        // let ics = &[0, 1, 2, 5, 3, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = Modulus::GF4 { p: 19 };
        let init_state_enc = f.encode_many(&init_state_m, &[x4_x_1; 49]).unwrap();

        let state = f.photon_196(&init_state_enc).unwrap();
        // println!("full: {}", res);
        let res_state_m: Vec<u16> = vec!(0xe, 0xd, 4, 0xe, 2, 9, 3,
                                         7, 6, 0xc, 8, 8, 0, 8,
                                         0xa, 7, 1, 1, 0xf, 7, 3,
                                         0xc, 6, 0xd, 9, 0xb, 0xc, 0xa,
                                         8, 3, 0xc, 1, 5, 0xc, 1,
                                         0xd, 3, 6, 2, 7, 9, 1,
                                         0xf, 0xb, 0xb, 4, 1, 0xb, 7);
        assert_eq!(res_state_m, state.into_iter().map(|w| w.val()).collect_vec());

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
        // let Z = &[2, 4, 2, 11, 2, 8, 5, 6];
        // let ics = &[0, 1, 3, 7, 15, 14, 12, 8];
        let mut f = Dummy::new();
        let x4_x_1 = Modulus::GF4 { p: 19 };
        let init_state_enc = f.encode_many(&init_state_m, &[x4_x_1; 64]).unwrap();

        let state = f.photon_256(&init_state_enc).unwrap();
        // println!("full: {}", res);
        let res_state_m: Vec<u16> = vec!(1, 9, 8, 0, 0xc, 0xa, 7, 8,
                                         7, 0xc, 0xd, 0, 6, 0xf, 4, 9,
                                         3, 0xf, 3, 0xe, 2, 4, 8, 1,
                                         0, 2, 0xd, 2, 9, 1, 3, 6,
                                         4, 6, 9, 7, 0xb, 0xf, 0xf, 0xb,
                                         2, 0xe, 0xc, 0xb, 3, 1, 0xc, 8,
                                         4, 1, 0xf, 0xd, 0xd, 0xc, 0xc, 2,
                                         2, 0, 9, 0xc, 1, 0xb, 0, 0xc);
        assert_eq!(res_state_m, state.into_iter().map(|w| w.val()).collect_vec());

    }


    #[test]
    fn photon256_du() {
        let init_state_m = vec!(0, 0 ,0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0,
                                0, 0 ,0, 0, 0, 0,
                                0, 0 ,0, 0, 0, 0x40,
                                0, 0 ,0, 0, 0, 0x20,
                                0, 0, 0, 0, 0, 0x20);
        
        // let Z: &[u16] = &[2, 3, 1, 2, 1, 4];
        // let ics = &[0, 1, 3, 7, 6, 4];
        let mut f = Dummy::new();
        let x8_x4_x3_x_1 = Modulus::GF8 { p: 283 };
        let init_state_enc = f.encode_many(&init_state_m, &[x8_x4_x3_x_1; 36]).unwrap();

        let state = f.photon_288(&init_state_enc).unwrap();
        // println!("full: {}", res);
        let res_state_m: Vec<u16> = vec!(0x4D, 0xE0, 0xE9, 0xCB, 0xE8, 0x18, 
                                         0xBD, 0x9E, 0xD5, 0x6B, 0xC2, 0xCC, 
                                         0x90, 0x5C, 0x66, 0xC8, 0xC0, 0x62, 
                                         0x36, 0x38, 0x08, 0x8B, 0x69, 0x9C, 
                                         0x1C, 0xA9, 0xCF, 0x93, 0x25, 0xAE, 
                                         0xB5, 0xC9, 0x52, 0x16, 0xF7, 0x79); 
        assert_eq!(res_state_m, state.into_iter().map(|w| w.val()).collect_vec());

    }

    // Builds photon circuit with circuitbuilder and evaluate it 
    // with the eval_plain function
    #[test]
    fn circuit_photon80() {
        // let Z: &[u16] = &[1, 2, 9, 9, 2];
        // let ics = &[0, 1, 3, 6, 4];
        let x4_x_1 = Modulus::GF4 { p: 19 };
        const D: usize = 5;

        // Build circuit
        let circ = {
            let mut b = CircuitBuilder::new();
            let x_vec = b.garbler_inputs(&[x4_x_1; D*D]); 
            let z = b.photon_100(&x_vec).unwrap();
            b.outputs(&z);
            b.finish()
        };

        // Evaluate with test vector (see test photon80_du)
        let garbler_input =    &[0, 0 ,0, 0, 4,
                                0, 0, 0, 0, 1,
                                0, 0 ,0, 0, 4,
                                0, 0 ,0, 0, 1,
                                0, 0 ,0, 1, 0];
        let res = circ.eval_plain(garbler_input, &[]).unwrap();
        let res_state_m: Vec<u16> = vec!(3, 6, 5, 6, 0xb,
                                        3, 2, 0xc, 5, 7,
                                        0xd, 9, 4, 0xc, 7,
                                        5, 0xb, 8, 0xe, 0,
                                        0xf, 9, 1, 7, 0xc);
        
        assert_eq!(res,res_state_m);
    }

    #[test]
    fn circuit_photon160() {
        // let Z: &[u16] = &[1, 4, 6, 1, 1, 6, 4];
        // let ics = &[0, 1, 2, 5, 3, 6, 4];
        let x4_x_1 = Modulus::GF4 { p: 19 };
        const D: usize = 6;

        // Build circuit
        let circ = {
            let mut b = CircuitBuilder::new();
            let x_vec = b.garbler_inputs(&[x4_x_1; D*D]); 
            let z = b.photon_196(&x_vec).unwrap();
            b.outputs(&z);
            b.finish()
        };

        // Evaluate with test vector 
        let garbler_input =    &[0, 0 ,0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 2,
                                0, 0 ,0, 0, 0, 0, 8,
                                0, 0 ,0, 0, 0, 0, 2,
                                0, 0 ,0, 0, 0, 0, 4,
                                0, 0, 0, 0, 0, 0, 2,
                                0, 0, 0, 0, 0, 0, 4];

        let res = circ.eval_plain(garbler_input, &[]).unwrap();
        let res_state_m: Vec<u16> = vec!(1, 0xd, 0xe, 0xb, 0xf, 0xe, 3,
                                         0xf, 0xd, 0xc, 6, 6, 9, 0xa,
                                         0, 0, 0xf, 6, 4, 0, 9,
                                         0xd, 0xa, 5, 0xe, 4, 2, 0xd,
                                         4, 3, 0xb, 0, 0xc, 0, 0xe,
                                         0xa, 1, 6, 0xc, 0xe, 0xf, 7,
                                         1, 0xd, 9, 8, 0xe, 4, 4);
        
        assert_eq!(res,res_state_m);
    }

    #[test]
    fn circuit_photon256() {
        // let Z: &[u16] = &[2, 3, 1, 2, 1, 4];
        // let ics = &[0, 1, 3, 7, 6, 4];
        let x8_x4_x3_x_1 = Modulus::GF8 { p: 283 };
        const D: usize = 6;

        // Build circuit
        let circ = {
            let mut b = CircuitBuilder::new();
            let x_vec = b.garbler_inputs(&[x8_x4_x3_x_1; D*D]); 
            let z = b.photon_288(&x_vec).unwrap();
            b.outputs(&z);
            b.finish()
        };

        // Evaluate with test vector 
        let garbler_input =    &[0, 0 ,0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0,
                                0, 0 ,0, 0, 0, 0,
                                0, 0 ,0, 0, 0, 0x40,
                                0, 0 ,0, 0, 0, 0x20,
                                0, 0, 0, 0, 0, 0x20];

        let res = circ.eval_plain(garbler_input, &[]).unwrap();
        let res_state_m: Vec<u16> = vec!(0x4D, 0xE0, 0xE9, 0xCB, 0xE8, 0x18, 
                                        0xBD, 0x9E, 0xD5, 0x6B, 0xC2, 0xCC, 
                                        0x90, 0x5C, 0x66, 0xC8, 0xC0, 0x62, 
                                        0x36, 0x38, 0x08, 0x8B, 0x69, 0x9C, 
                                        0x1C, 0xA9, 0xCF, 0x93, 0x25, 0xAE, 
                                        0xB5, 0xC9, 0x52, 0x16, 0xF7, 0x79); 
        
        assert_eq!(res,res_state_m);
    }


    #[test]
    fn forward_backward_permutation_80() {
        let Z: &[u16] = &[1, 2, 9, 9, 2];
        let ics = &[0, 1, 3, 6, 4];
        let x4_x_1 = Modulus::GF4 { p: 19 };
        const D: usize = 5; 

        // Build circuit
        let circ = {
            let mut b = CircuitBuilder::new();
            let x_vec = b.garbler_inputs(&[x4_x_1; D*D]); 
            let xx = b.photon_100(&x_vec).unwrap();
            let z = b.photon_custom_inv(&xx, D, ics, Z, true).unwrap();
            b.outputs(&z);
            b.finish()
        };

        // Evaluate with test vector (see test photon80_du)
        let garbler_input =    &[0, 0 ,0, 0, 4,
                                0, 0, 0, 0, 1,
                                0, 0 ,0, 0, 4,
                                0, 0 ,0, 0, 1,
                                0, 0 ,0, 1, 0];
        let res = circ.eval_plain(garbler_input, &[]).unwrap();
        assert_eq!(res,garbler_input);
    }

    #[test]
    fn backward_80() {
        let init_state: Vec<u16> = vec!(3, 6, 5, 6, 0xb,
            3, 2, 0xc, 5, 7,
            0xd, 9, 4, 0xc, 7,
            5, 0xb, 8, 0xe, 0,
            0xf, 9, 1, 7, 0xc);
        
        let Z: &[u16] = &[1, 2, 9, 9, 2];
        let ics = &[0, 1, 3, 6, 4];
        let mut f = Dummy::new();
        let x4_x_1 = Modulus::GF4 { p: 19 };
        let init_state_enc = f.encode_many(&init_state, &[x4_x_1; 25]).unwrap();
        let state = f.photon_custom_inv(&init_state_enc,5,ics,Z,true).unwrap();
        // println!("full: {}", res);
        let res_state = vec!(0, 0 ,0, 0, 4,
            0, 0, 0, 0, 1,
            0, 0 ,0, 0, 4,
            0, 0 ,0, 0, 1,
            0, 0 ,0, 1, 0);
  
        assert_eq!(res_state, state.into_iter().map(|w| w.val()).collect_vec());
    }


/// ---------- garble.rs tests --------------------

    // helper - checks that Streaming evaluation of a fancy function equals Dummy
    // evaluation of the same function
    fn streaming_test_GF4<FGB, FEV, FDU>(
        mut f_gb: FGB,
        mut f_ev: FEV,
        mut f_du: FDU,
        input_mod: &Modulus,
        d: usize,
    ) where
        FGB: FnMut(&mut Garbler<UnixChannel, AesRng>, &Vec<Wire>) -> Option<Vec<u16>> + Send + Sync,
        FEV: FnMut(&mut Evaluator<UnixChannel>, &Vec<Wire>) -> Option<Vec<u16>>,
        FDU: FnMut(&mut Dummy, &Vec<DummyVal>) -> Option<Vec<u16>>,
    {
        let mut rng = AesRng::new();
        let inputs = (0..d*d).map(|_| (rng.gen::<u8>()&(15)) as u16).collect_vec();

        // evaluate f_gb as a dummy
        let mut dummy = Dummy::new();
        let dinp = dummy.encode_many(&inputs, &vec![*input_mod; d*d]).unwrap();
        let should_be = f_du(&mut dummy, &dinp).unwrap();
        println!("inp: {:?} -> {:?}", inputs, should_be);

        let (sender, receiver) = unix_channel_pair();

        crossbeam::scope(|s| {
            s.spawn(move |_| {
                let mut gb = Garbler::new(sender, rng);
                let (gb_inp, ev_inp) = gb.encode_many_wires(&inputs, &vec![*input_mod; d*d]).unwrap();
                ev_inp.iter().for_each(|w| gb.send_wire(&w).unwrap());
                f_gb(&mut gb, &gb_inp);
            });

            let mut ev = Evaluator::new(receiver);
            let ev_inp = (0..d*d)
                .map(|_| ev.read_wire(input_mod).unwrap())
                .collect_vec();
            let result = f_ev(&mut ev, &ev_inp).unwrap();

            assert_eq!(result, should_be)
        })
        .unwrap();
    }

    #[test]
    fn photon100() {
        fn fancy_photon100<F: Fancy>(b: &mut F, x: &Vec<F::Item>) -> Option<Vec<u16>> {
            // let Z: &[u16] = &[1, 2, 9, 9, 2];
            // let ics = &[0, 1, 3, 6, 4];
            let z = b.photon_100(x).unwrap();
            b.outputs(&z).unwrap()
        }

        for _ in 0..16 {
            let q = Modulus::GF4 { p: 19 };
            streaming_test_GF4(
                move |b, x| fancy_photon100(b, x),
                move |b, x| fancy_photon100(b, x),
                move |b, x| fancy_photon100(b, x),
                &q,
                5
            );
        }
    }

/// ------------ mod.rs tests ------------------

    #[test]
    fn test_photon80() {
        let mut rng = rand::thread_rng();
        let p = Modulus::GF4 { p: 19 };
        let d = 5;
        let n = d*d;
        let input = (0..n).map(|_| rng.gen_u16() % 16).collect::<Vec<u16>>();
        println!("inp: {:?}", input);

        // let ics = [0, 1, 3, 6, 4];
        // let Z = [1, 2, 9, 9, 2];

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input =  dummy.encode_many(&input, &vec![p; d*d]).unwrap();
        let target_enc = dummy.photon_100(&dummy_input).unwrap();
        let target = dummy.outputs(&target_enc).unwrap().unwrap();
        println!("trgt: {:?}", target);

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                twopac::semihonest::Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_many(&input, &vec![p; d*d]).unwrap();
            let gb_o = gb.photon_100(&xs).unwrap();
            gb.outputs(&gb_o);
        });

        let rng = AesRng::new();
        let mut ev =
            twopac::semihonest::Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_many(&vec![p; d*d]).unwrap();
        let result = ev.outputs(&xs).unwrap().unwrap();
        println!("res: {:?}", result);
        assert_eq!(target, result);
    }

    #[test]
    fn test_photon192() {
        let mut rng = rand::thread_rng();
        let p = Modulus::GF4 { p: 19 };
        let d = 7;
        let n = d*d;
        let input = (0..n).map(|_| rng.gen_u16() % 16).collect::<Vec<u16>>();
        println!("inp: {:?}", input);

        // let ics = [0, 1, 2, 5, 3, 6, 4];
        // let Z = [1, 4, 6, 1, 1, 6, 4];

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input =  dummy.encode_many(&input, &vec![p; d*d]).unwrap();
        let target_enc = dummy.photon_196(&dummy_input).unwrap();
        let target = dummy.outputs(&target_enc).unwrap().unwrap();
        println!("trgt: {:?}", target);

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                twopac::semihonest::Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_many(&input, &vec![p; d*d]).unwrap();
            let gb_o = gb.photon_196(&xs).unwrap();
            gb.outputs(&gb_o);
        });

        let rng = AesRng::new();
        let mut ev =
            twopac::semihonest::Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_many(&vec![p; d*d]).unwrap();
        let result = ev.outputs(&xs).unwrap().unwrap();
        println!("res: {:?}", result);
        assert_eq!(target, result);
    }

    #[test]
    fn test_photon256() {
        let mut rng = rand::thread_rng();
        let p = Modulus::GF8 { p: 283 };
        let d = 6;
        let n = d*d;
        let input = (0..n).map(|_| rng.gen_u16() % 256).collect::<Vec<u16>>();
        println!("inp: {:?}", input);

        // let ics = [0, 1, 3, 7, 6, 4];
        // let Z = [2, 3, 1, 2, 1, 4];

        // Run dummy version.
        let mut dummy = Dummy::new();
        let dummy_input =  dummy.encode_many(&input, &vec![p; d*d]).unwrap();
        let target_enc = dummy.photon_256(&dummy_input).unwrap();
        let target = dummy.outputs(&target_enc).unwrap().unwrap();
        println!("trgt: {:?}", target);

        // Run 2PC version.
        let (sender, receiver) = unix_channel_pair();
        std::thread::spawn(move || {
            let rng = AesRng::new();
            let mut gb =
                twopac::semihonest::Garbler::<UnixChannel, AesRng, ChouOrlandiSender>::new(sender, rng).unwrap();
            let xs = gb.encode_many(&input, &vec![p; d*d]).unwrap();
            let gb_o = gb.photon_256(&xs).unwrap();
            gb.outputs(&gb_o);
        });

        let rng = AesRng::new();
        let mut ev =
            twopac::semihonest::Evaluator::<UnixChannel, AesRng, ChouOrlandiReceiver>::new(receiver, rng).unwrap();
        let xs = ev.receive_many(&vec![p; d*d]).unwrap();
        let result = ev.outputs(&xs).unwrap().unwrap();
        println!("res: {:?}", result);
        assert_eq!(target, result);
    }
}