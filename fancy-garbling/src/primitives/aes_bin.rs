use crate::{BinaryBundle, BinaryGadgets, Fancy, Modulus};
use crate::primitives::utils;
use crate::primitives::utils::{xor_bundle};

const SHIFT_ROWS_PERMUTATION: [usize; 16] = [0,5,10,15,4,9,14,3,8,13,2,7,12,1,6,11];
const AES_RCS: [u16; 11] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36, 0x6c];

struct AesState<F: Fancy> {
    state: Vec<BinaryBundle<F::Item>>,
    key_schedule: Vec<BinaryBundle<F::Item>>,
}

fn fill_state<T: Clone>(x: &[BinaryBundle<T>]) -> Vec<BinaryBundle<T>> {
    debug_assert_eq!(x.len(), 16);
    return x.to_vec();
}

impl<F: Fancy> AesState<F> {
    pub fn new(state: &[BinaryBundle<F::Item>], key: &[BinaryBundle<F::Item>]) -> Self {
        Self {
            state: fill_state(state),
            key_schedule: fill_state(key),
        }
    }

    // fn sub_bytes(&mut self, f: &mut F) -> Result<(),F::Error> {
    //     for i in 0..16 {
    //         self.state[i] = aes_sbox(f, self.state[i]?);
    //     }
    //     Ok(())
    // }

    fn shift_rows(&mut self) {
        self.state = (0..16)
            .map(|i| self.state[SHIFT_ROWS_PERMUTATION[i]].clone())
            .collect();
    }

    fn mix_columns(&mut self, f: &mut F) -> Result<(), F::Error> {
        let doubles = self.state.iter()
            .map(|x| {
                // let cell = x.wires();
                utils::cmul_mod_x8_x4_x3_x_1(f, 2, x)
                    // .map(|bits| BinaryBundle::new(bits.to_vec()))
            })
            .collect::<Result<Vec<_>,_>>()?;
        let mut new_state = Vec::with_capacity(16);

        for x in 0..4 {
            let i = 4*x;
            new_state.push(xor_bundle(f,[&doubles[i], &doubles[i+1], &self.state[i+1], &self.state[i+2], &self.state[i+3]], 8)?);
            new_state.push(xor_bundle(f, [&self.state[i], &doubles[i+1], &doubles[i+2], &self.state[i+2], &self.state[i+3]], 8)?);
            new_state.push(xor_bundle(f, [&self.state[i], &self.state[i+1], &doubles[i+2], &doubles[i+3], &self.state[i+3]], 8)?);
            new_state.push(xor_bundle(f, [&doubles[i], &self.state[i], &self.state[i+1], &self.state[i+2], &doubles[i+3]], 8)?);
        }
        self.state = new_state;
        Ok(())
    }

    fn update_key(&mut self, f: &mut F, rc: &BinaryBundle<F::Item>) -> Result<(), F::Error> {
        let mut new_key = Vec::with_capacity(16);
        for i in 0..4 {
            let x = &self.key_schedule[12 + (i+1)%4];
            let mut x = utils::aes_sbox(f, x)?;
            if i == 0 {
                x = f.bin_xor(&x, rc)?;
            }
            new_key.push(f.bin_xor(&x, &self.key_schedule[i])?);
        }
        for i in 4..16 {
            new_key.push(f.bin_xor(&new_key[i-4], &self.key_schedule[i])?);
        }
        self.key_schedule = new_key;
        Ok(())
    }

    fn add_round_key(&mut self, f: &mut F) -> Result<(), F::Error> {
        for i in 0..16 {
            self.state[i] = f.bin_xor(&self.state[i], &self.key_schedule[i])?;
        }
        Ok(())
    }

    pub fn into_state(self) -> Vec<BinaryBundle<F::Item>> {
        self.state
    }

    pub fn aes128_forward(&mut self, f: &mut F, rcs: &[BinaryBundle<F::Item>]) -> Result<(), F::Error> {
        debug_assert_eq!(rcs.len(), 11);
        self.add_round_key(f)?;
        for i in 0..9 {
            utils::sbox_layer_bin(f, &mut self.state, utils::aes_sbox)?;
            // self.sub_bytes(f)?;
            self.shift_rows();
            self.mix_columns(f)?;
            self.update_key(f, &rcs[i])?;
            self.add_round_key(f)?;
        }
        utils::sbox_layer_bin(f, &mut self.state, utils::aes_sbox)?;
        self.shift_rows();
        self.update_key(f, &rcs[9])?;
        self.add_round_key(f)?;
        Ok(())
    }
}

pub trait AesBinFancyExt: Fancy {
    fn aes128_bin_forward(&mut self, state: &[BinaryBundle<Self::Item>], key: &[BinaryBundle<Self::Item>]) -> Result<Vec<BinaryBundle<Self::Item>>, Self::Error>;
}

impl<F: Fancy> AesBinFancyExt for F {
    fn aes128_bin_forward(&mut self, state: &[BinaryBundle<Self::Item>], key: &[BinaryBundle<Self::Item>]) -> Result<Vec<BinaryBundle<Self::Item>>, Self::Error> {
        let mut state = AesState::<F>::new(state, key);
        let rcs = AES_RCS.iter().map(|rc| self.bin_constant_bundle(*rc as u128, 8))
            .collect::<Result<Vec<_>, _>>()?;
        state.aes128_forward(self, &rcs)?;
        Ok(state.into_state())
    }
}

#[cfg(test)]
mod tests {
    use crate::aes_bin::AesBinFancyExt;
    use crate::circuit::CircuitBuilder;
    use crate::classic::garble;
    use crate::{BinaryGadgets, Fancy};

    #[test]
    fn test_aes128() {
        for (input, key, expected) in [
            ([0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d, 0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07, 0x34], [0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c], vec![0x39u16, 0x25, 0x84, 0x1d, 0x02, 0xdc, 0x09, 0xfb, 0xdc, 0x11, 0x85, 0x97, 0x19, 0x6a, 0x0b, 0x32]),
            ([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff], [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f], vec![0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5, 0x5a]),
        ] {
            let mut b = CircuitBuilder::new();
            let input_wires = input.iter().map(|x| b.bin_constant_bundle(*x, 8))
                .collect::<Result<Vec<_>,_>>()
                .unwrap();
            let key_wires = key.iter().map(|x| b.bin_constant_bundle(*x, 8))
                .collect::<Result<Vec<_>,_>>()
                .unwrap();
            let output_wires = b.aes128_bin_forward(&input_wires, &key_wires).unwrap();
            assert_eq!(None, b.bin_outputs(&output_wires).unwrap());

            let circuit = b.finish();
            let (_, gc) = garble(&circuit).unwrap();
            let outputs = gc.eval(&circuit, &[], &[]).unwrap();

            let expected = expected.into_iter().map(|x| (0..8).map(move |i| (x >> i) & 0x1 as u16))
                .flatten()
                .collect::<Vec<_>>();
            assert_eq!(outputs, expected);
        }
    }
}