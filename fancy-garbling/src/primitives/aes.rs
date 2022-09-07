use lazy_static::lazy_static;
use crate::{Fancy, Modulus};
use crate::primitives::utils::sum;

const STATE_SIZE: usize = 4;
const AES_MODULUS: Modulus = Modulus::GF8 { p: 283 };
const AES_MIX_CLOUMNS: [[u8; STATE_SIZE]; STATE_SIZE] = [
    [2,3,1,1],
    [1,2,3,1],
    [1,1,2,3],
    [3,1,1,2],
];
const SHIFT_ROWS_PERMUTATION: [usize; 16] = [0,5,10,15,4,9,14,3,8,13,2,7,12,1,6,11];
const AES_RCS: [u16; 11] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36, 0x6c];
lazy_static! {
    static ref SBOX_AES: Vec<u16> = vec![
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
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
        0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16
    ];
}

struct AesState<F: Fancy> {
    state: Vec<F::Item>,
    key_schedule: Vec<F::Item>,
}

fn fill_array<T: Clone>(state: &[T]) -> [[T; STATE_SIZE]; STATE_SIZE] {
    debug_assert_eq!(STATE_SIZE*STATE_SIZE, state.len());
    [
        [state[0].clone(), state[4].clone(), state[8].clone(), state[12].clone()],
        [state[1].clone(), state[5].clone(), state[9].clone(), state[13].clone()],
        [state[2].clone(), state[6].clone(), state[10].clone(), state[14].clone()],
        [state[3].clone(), state[7].clone(), state[11].clone(), state[15].clone()],
    ]
}

impl<F: Fancy> AesState<F> {
    pub fn new(state: &[F::Item], key: &[F::Item]) -> Self {
        debug_assert_eq!(state.len(), 16);
        debug_assert_eq!(key.len(), 16);
        Self {
            state: state.to_vec(),
            key_schedule: key.to_vec(),
        }
    }
    
    fn sub_bytes(&mut self, f: &mut F) -> Result<(),F::Error> {
        for i in 0..16 {
            self.state[i] = f.proj(&self.state[i], &AES_MODULUS, Some(SBOX_AES.clone()))?;
        }
        Ok(())
    }

    fn shift_rows(&mut self) {
        self.state = (0..16)
            .map(|i| self.state[SHIFT_ROWS_PERMUTATION[i]].clone())
            .collect();
    }

    fn mix_columns(&mut self, f: &mut F) -> Result<(), F::Error> {
        let doubles = self.state.iter()
            .map(|x| f.cmul(x, 2))
            .collect::<Result<Vec<_>,_>>()?;
        let mut new_state = Vec::with_capacity(16);

        for x in 0..4 {
            let i = 4*x;
            new_state.push(sum(f,[&doubles[i], &doubles[i+1], &self.state[i+1], &self.state[i+2], &self.state[i+3]], &AES_MODULUS)?);
            new_state.push(sum(f, [&self.state[i], &doubles[i+1], &doubles[i+2], &self.state[i+2], &self.state[i+3]], &AES_MODULUS)?);
            new_state.push(sum(f, [&self.state[i], &self.state[i+1], &doubles[i+2], &doubles[i+3], &self.state[i+3]], &AES_MODULUS)?);
            new_state.push(sum(f, [&doubles[i], &self.state[i], &self.state[i+1], &self.state[i+2], &doubles[i+3]], &AES_MODULUS)?);
        }
        self.state = new_state;
        Ok(())
    }
    
    fn update_key(&mut self, f: &mut F, rc: &F::Item) -> Result<(), F::Error> {
        let mut new_key = Vec::with_capacity(16);
        for i in 0..4 {
            let x = &self.key_schedule[12 + (i+1)%4];
            let mut x = f.proj(x, &AES_MODULUS, Some(SBOX_AES.clone()))?;
            if i == 0 {
                x = f.add(&x, rc)?;
            }
            new_key.push(f.add(&x, &self.key_schedule[i])?);
        }
        for i in 4..16 {
            new_key.push(f.add(&new_key[i-4], &self.key_schedule[i])?);
        }
        self.key_schedule = new_key;
        Ok(())
    }

    fn add_round_key(&mut self, f: &mut F) -> Result<(), F::Error> {
        for i in 0..16 {
            self.state[i] = f.add(&self.state[i], &self.key_schedule[i])?;
        }
        Ok(())
    }

    pub fn into_state(self) -> Vec<F::Item> {
        self.state
    }

    pub fn aes128_forward(&mut self, f: &mut F, rcs: &[F::Item]) -> Result<(), F::Error> {
        debug_assert_eq!(rcs.len(), 11);
        self.add_round_key(f)?;
        for i in 0..9 {
            // f.outputs(&self.state)?;
            self.sub_bytes(f)?;
            self.shift_rows();
            self.mix_columns(f)?;
            self.update_key(f, &rcs[i])?;
            self.add_round_key(f)?;
        }
        // f.outputs(&self.state)?;
        self.sub_bytes(f)?;
        self.shift_rows();
        self.update_key(f, &rcs[9])?;
        self.add_round_key(f)?;
        // f.outputs(&self.state)?;
        Ok(())
    }
}

pub trait AesFancyExt: Fancy {
    fn aes128_forward(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error>;
}

impl<F: Fancy> AesFancyExt for F {
    fn aes128_forward(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error> {
        let mut state = AesState::<F>::new(state, key);
        let rcs = AES_RCS.iter().map(|rc| self.constant(*rc, &AES_MODULUS))
            .collect::<Result<Vec<_>, _>>()?;
        state.aes128_forward(self, &rcs)?;
        Ok(state.into_state())
    }
}

#[cfg(test)]
mod tests {
    use crate::aes::{AES_MODULUS, AesFancyExt};
    use crate::circuit::CircuitBuilder;
    use crate::classic::garble;
    use crate::Fancy;

    #[test]
    fn test_aes128() {
        for (input, key, expected) in [
            ([0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d, 0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07, 0x34], [0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c], vec![0x39u16, 0x25, 0x84, 0x1d, 0x02, 0xdc, 0x09, 0xfb, 0xdc, 0x11, 0x85, 0x97, 0x19, 0x6a, 0x0b, 0x32]),
            ([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff], [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f], vec![0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5, 0x5a]),
        ] {
            let mut b = CircuitBuilder::new();
            let input_wires = input.iter().map(|x| b.constant(*x, &AES_MODULUS))
                .collect::<Result<Vec<_>,_>>()
                .unwrap();
            let key_wires = key.iter().map(|x| b.constant(*x, &AES_MODULUS))
                .collect::<Result<Vec<_>,_>>()
                .unwrap();
            let output_wires = b.aes128_forward(&input_wires, &key_wires).unwrap();
            assert_eq!(None, b.outputs(&output_wires).unwrap());

            let circuit = b.finish();
            let (_, gc) = garble(&circuit).unwrap();
            let outputs = gc.eval(&circuit, &[], &[]).unwrap();

            assert_eq!(outputs, expected);
        }

    }
}