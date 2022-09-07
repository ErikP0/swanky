use itertools::Itertools;
use crate::{Fancy, Modulus};
use crate::primitives::utils;
const PICCOLO_MODULUS: Modulus = Modulus::X4_X_1;
const PICCOLO_SBOX: [u16; 16] = [0xe, 0x4, 0xb, 0x2, 0x3, 0x8, 0x0, 0x9, 0x1, 0xa, 0x7, 0xf, 0x6, 0xc, 0x5, 0xd];
const PICCOLO_DIFF_MATRIX: [u16; 16] = [2,3,1,1,1,2,3,1,1,1,2,3,3,1,1,2];
const PICCOLO_KS128_PERMUTATION: [usize; 32] = [8, 9, 10, 11, 4, 5, 6, 7, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23];
const PICCOLO_RC: [u32; 31] = [
    0x6d45ad8a, 0x7543a189, 0x7d41a588, 0x454fb98f, 0x4d4dbd8e,
    0x554bb18d, 0x5d49b58c, 0x25578983, 0x2d558d82, 0x35538181,
    0x3d518580, 0x055f9987, 0x0d5d9d86, 0x155b9185, 0x1d599584,
    0xe567e99b, 0xed65ed9a, 0xf563e199, 0xfd61e598, 0xc56ff99f,
    0xcd6dfd9e, 0xd56bf19d, 0xdd69f59c, 0xa577c993, 0xad75cd92,
    0xb573c191, 0xbd71c590, 0x857fd997, 0x8d7ddd96, 0x957bd195,
    0x9d79d594
];

struct Piccolo<F: Fancy> {
    x0: Vec<F::Item>,
    x1: Vec<F::Item>,
    x2: Vec<F::Item>,
    x3: Vec<F::Item>,
    key: Vec<F::Item>,
}

impl<F: Fancy> Piccolo<F> {

    pub fn new(state: &[F::Item], key: &[F::Item]) -> Self {
        assert_eq!(state.len(), 16);
        assert_eq!(key.len(), 32);
        Self {
            x0: state[0..4].to_vec(),
            x1: state[4..8].to_vec(),
            x2: state[8..12].to_vec(),
            x3: state[12..16].to_vec(),
            key: key.to_vec(),
        }
    }

    pub fn into_wires(mut self) -> Vec<F::Item> {
        let mut res = Vec::with_capacity(16);
        res.append(&mut self.x0);
        res.append(&mut self.x1);
        res.append(&mut self.x2);
        res.append(&mut self.x3);
        return res;
    }

    fn f(f: &mut F, x: &[F::Item]) -> Result<Vec<F::Item>,F::Error> {
        let mut x = x.to_vec();
        utils::sbox_layer_proj(f, &mut x, &PICCOLO_MODULUS, &PICCOLO_SBOX)?;
        let mut x = utils::matrix_vec_mul(f, &PICCOLO_DIFF_MATRIX, &x, &PICCOLO_MODULUS)?;
        utils::sbox_layer_proj(f, &mut x, &PICCOLO_MODULUS, &PICCOLO_SBOX)?;
        Ok(x)
    }

    fn round_permutation(&mut self) {
        let x0 = self.x1[0..2].iter().chain(&self.x3[2..4]).map(|c| c.clone()).collect();
        let x1 = self.x2[0..2].iter().chain(&self.x0[2..4]).map(|c| c.clone()).collect();
        let x2 = self.x3[0..2].iter().chain(&self.x1[2..4]).map(|c| c.clone()).collect();
        let x3 = self.x0[0..2].iter().chain(&self.x2[2..4]).map(|c| c.clone()).collect();
        self.x0 = x0;
        self.x1 = x1;
        self.x2 = x2;
        self.x3 = x3;
    }

    fn forward(&mut self, f: &mut F) -> Result<(), F::Error> {
        // wk0 = k0L|k1R = key[0..2]|key[6..8]
        utils::add_roundkey_layer(f, &mut self.x0[0..2], &self.key[0..2])?;
        utils::add_roundkey_layer(f, &mut self.x0[2..4], &self.key[6..8])?;
        // wk1 = k1L|k0R = key[4..6]|key[2..4]
        utils::add_roundkey_layer(f, &mut self.x2[0..2], &self.key[4..6])?;
        utils::add_roundkey_layer(f, &mut self.x2[2..4], &self.key[2..4])?;

        // wk2 = k4L|k7R
        let wk2: Vec<_> = self.key[16..18].iter().chain(&self.key[30..32]).map(|c| c.clone()).collect();
        // wk3 = k7L|k4R
        let wk3: Vec<_> = self.key[28..30].iter().chain(&self.key[18..20]).map(|c| c.clone()).collect();

        for i in 0..31 {

            if (2*i+2) % 8 == 0 || (2*i+3) % 8 == 0{
                self.key = utils::permute_state(&self.key, &PICCOLO_KS128_PERMUTATION);
            }
            let rki = (2*i+2) % 8;
            let rkip1 = (2*i+3) % 8;
            let rk2i = &self.key[4*rki..4*rki+4];
            let rc2i = (PICCOLO_RC[i] & 0xffff0000) >> 16;
            let rc2i = (0..4).map(|i| f.constant(((rc2i >> 4 * i) & 0xf) as u16, &PICCOLO_MODULUS)).collect::<Result<Vec<_>, _>>()?;

            let rk2ip1 = &self.key[4*rkip1..4*rkip1+4];
            let rc2ip1 = (PICCOLO_RC[i] & 0xffff) >> 16;
            let rc2ip1 = (0..4).map(|i| f.constant(((rc2ip1 >> 4 * i) & 0xf) as u16, &PICCOLO_MODULUS)).collect::<Result<Vec<_>, _>>()?;

            let f_x0 = Self::f(f, &self.x0)?;
            utils::add_roundkey_layer(f, &mut self.x1, &f_x0)?;
            assert_eq!(f_x0.len(), rk2i.len());
            assert_eq!(f_x0.len(), rk2i.len());
            utils::add_roundkey_layer(f, &mut self.x1, &rk2i)?;
            utils::add_roundkey_layer(f, &mut self.x1, &rc2i)?;

            let f_x2 = Self::f(f, &self.x2)?;
            utils::add_roundkey_layer(f, &mut self.x3, &f_x2)?;
            assert_eq!(f_x2.len(), rk2ip1.len());
            assert_eq!(f_x2.len(), rk2ip1.len());
            utils::add_roundkey_layer(f, &mut self.x3, &rk2ip1)?;
            utils::add_roundkey_layer(f, &mut self.x3, &rc2ip1)?;

            if i < 30 {
                self.round_permutation();
            }
        }

        utils::add_roundkey_layer(f, &mut self.x0, &wk2)?;
        utils::add_roundkey_layer(f, &mut self.x2, &wk3)?;

        Ok(())
    }
}

pub trait PiccoloFancyExt: Fancy {
    fn piccolo128(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error>;
}

impl<F: Fancy> PiccoloFancyExt for F {
    fn piccolo128(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error> {
        let mut piccolo = Piccolo::new(state, key);
        piccolo.forward(self)?;
        Ok(piccolo.into_wires())
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::dummy::Dummy;
    use crate::Fancy;
    use crate::piccolo::{PICCOLO_MODULUS, PiccoloFancyExt};

    #[test]
    fn test_piccolo128() {
        let key = [0x0, 0x0, 0x1, 0x1, 0x2, 0x2, 0x3, 0x3, 0x4, 0x4, 0x5, 0x5, 0x6, 0x6, 0x7, 0x7, 0x8, 0x8, 0x9, 0x9, 0xa, 0xa, 0xb, 0xb, 0xc, 0xc, 0xd, 0xd, 0xe, 0xe, 0xf, 0xf];
        let state = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf];
        let expected = vec![0x5, 0xe, 0xc, 0x4, 0x2, 0xc, 0xe, 0xa, 0x6, 0x5, 0x7, 0xb, 0x8, 0x9, 0xf, 0xf];
        let mut b = Dummy::new();
        let key_input = key.iter().map(|c| b.constant(*c, &PICCOLO_MODULUS).unwrap()).collect_vec();
        let state_input = state.iter().map(|c| b.constant(*c, &PICCOLO_MODULUS).unwrap()).collect_vec();

        let output = b.piccolo128(&state_input, &key_input).unwrap();
        let output = b.outputs(&output).unwrap().unwrap();
        assert_eq!(output, expected);
    }
}