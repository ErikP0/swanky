use crate::{BinaryBundle, Fancy, FancyError, HasModulus, Modulus};
use crate::primitives::utils;
use crate::primitives::utils::SBOX_PRESENT;

struct Led<F: Fancy> {
    state: Vec<F::Item>,
    key: Vec<F::Item>,
}

struct LedBin<F: Fancy> {
    state: Vec<BinaryBundle<F::Item>>,
    key: Vec<BinaryBundle<F::Item>>,
}

const LED_MODULUS: Modulus = Modulus::X4_X_1;
const KS64: [u16; 4] = [64 >> 4, (64 >> 4) ^ 1, (64 & 0xf) ^ 2, (64 & 0xf) ^ 3];
const KS128: [u16; 4] = [128 >> 4, (128 >> 4) ^ 1, (128 & 0xf) ^ 2, (128 & 0xf) ^ 3];
const ROUND_CONSTANTS: [u8; 48] = [
    0x01, 0x03, 0x07, 0x0F, 0x1F, 0x3E, 0x3D, 0x3B, 0x37, 0x2F, 0x1E, 0x3C, 0x39, 0x33, 0x27, 0x0E, 0x1D, 0x3A, 0x35, 0x2B, 0x16, 0x2C, 0x18, 0x30,
    0x21, 0x02, 0x05, 0x0B, 0x17, 0x2E, 0x1C, 0x38, 0x31, 0x23, 0x06, 0x0D, 0x1B, 0x36, 0x2D, 0x1A, 0x34, 0x29, 0x12, 0x24, 0x08, 0x11, 0x22, 0x04,
];
const SHIFT_ROWS_PERMUTATION: [usize; 16] = [0,5,10,15,4,9,14,3,8,13,2,7,12,1,6,11];

fn fill_column_first<T: Clone>(input: &[T], size: usize) -> Vec<T> {
    let mut res = Vec::with_capacity(size * size);
    for i in 0..size {
        for j in 0..size {
            res.push(input[i+size*j].clone());
        }
    }
    return res;
}

fn unfill_column_first<T: Clone>(input: &[T], size: usize) -> Vec<T> {
    let mut res = Vec::with_capacity(size * size);
    for i in 0..size {
        for j in 0..size {
            res.push(input[i+size*j].clone());
        }
    }
    return res;
}

impl<F: Fancy> Led<F> {
    pub fn new(state: &[F::Item], key: &[F::Item]) -> Self {
        debug_assert_eq!(state.len(), 16);
        debug_assert!(key.len() == 16 || key.len() == 32);
        let filled_key = if key.len() == 16 {
            fill_column_first(key, 4)
        }else{
            let mut left = fill_column_first(&key[0..16], 4);
            let mut right = fill_column_first(&key[16..32], 4);
            left.append(&mut right);
            left
        };
        Self {
            state: fill_column_first(state, 4),
            key: filled_key,
        }
    }

    pub fn forward(&mut self, f: &mut F) -> Result<(), F::Error>  {
        let (s,ks) = if self.key.len() == 16 {
            (8, KS64)
        }else{
            (12, KS128)
        };
        for i in 0..s {
            let rk = &self.key[(16*i % self.key.len())..(16*i % self.key.len() + 16)];
            utils::add_roundkey_layer(f, &mut self.state, rk)?;
            for r in 0..4 {
                // add constants
                let rc = ROUND_CONSTANTS[4*i + r] as u16;
                utils::add_constants_layer(f, &mut self.state, &[ks[0], ks[1], ks[2], ks[3], rc >> 3, rc & 0b111, rc >> 3, rc & 0b111], &LED_MODULUS)?;
                utils::sbox_layer_proj(f, &mut self.state, &LED_MODULUS, &SBOX_PRESENT)?;
                self.state = utils::permute_state(&self.state, &SHIFT_ROWS_PERMUTATION);
                utils::mix_columns_mds(f, &mut self.state, 4, &[4,1,2,2], 4)?;
            }
        }
        let rk = &self.key[(16*s % self.key.len())..(16*s % self.key.len() + 16)];
        utils::add_roundkey_layer(f, &mut self.state, rk)?;
        Ok(())
    }

    pub fn into_state(self) -> Vec<F::Item> {
        unfill_column_first(&self.state, 4)
    }
}

impl<F: Fancy> LedBin<F> {
    pub fn new(state: &[BinaryBundle<F::Item>], key: &[BinaryBundle<F::Item>]) -> Self {
        debug_assert_eq!(state.len(), 16);
        debug_assert!(key.len() == 16 || key.len() == 32);
        let filled_key = if key.len() == 16 {
            fill_column_first(key, 4)
        }else{
            let mut left = fill_column_first(&key[0..16], 4);
            let mut right = fill_column_first(&key[16..32], 4);
            left.append(&mut right);
            left
        };
        Self {
            state: fill_column_first(state, 4),
            key: filled_key,
        }
    }

    pub fn forward(&mut self, f: &mut F) -> Result<(), F::Error>  {
        let (s,ks) = if self.key.len() == 16 {
            (8, KS64)
        }else{
            (12, KS128)
        };
        for i in 0..s {
            let rk = &self.key[(16*i % self.key.len())..(16*i % self.key.len() + 16)];
            utils::add_roundkey_layer_bin(f, &mut self.state, rk)?;
            for r in 0..4 {
                // add constants
                let rc = ROUND_CONSTANTS[4*i + r] as u16;
                utils::add_constants_layer_bin(f, &mut self.state, &[ks[0], ks[1], ks[2], ks[3], rc >> 3, rc & 0b111, rc >> 3, rc & 0b111])?;
                utils::sbox_layer_bin(f, &mut self.state, utils::present_sbox)?;
                self.state = utils::permute_state(&self.state, &SHIFT_ROWS_PERMUTATION);
                utils::mix_columns_mds_bin(f, &mut self.state, 4, &[4,1,2,2], 4, utils::cmul_mod_x4_x_1)?;
            }
        }
        let rk = &self.key[(16*s % self.key.len())..(16*s % self.key.len() + 16)];
        utils::add_roundkey_layer_bin(f, &mut self.state, rk)?;
        Ok(())
    }

    pub fn into_state(self) -> Vec<BinaryBundle<F::Item>> {
        unfill_column_first(&self.state, 4)
    }
}

fn check_modulus<F: Fancy>(wires: &[F::Item]) -> Result<(), FancyError> {
    for w in wires {
        if w.modulus() != LED_MODULUS {
            return Err(FancyError::InvalidArgMod {needed: LED_MODULUS, got: w.modulus()});
        }
    }
    Ok(())
}

pub trait LedFancyExt : Fancy + Sized {
    fn led64(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error>;

    fn led128(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error>;

    fn led64_bin(&mut self, state: &[BinaryBundle<Self::Item>], key: &[BinaryBundle<Self::Item>]) -> Result<Vec<BinaryBundle<Self::Item>>, Self::Error>;
}

impl<F: Fancy> LedFancyExt for F {
    fn led64(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error> {
        debug_assert_eq!(state.len(), 16);
        debug_assert_eq!(key.len(), 16);
        check_modulus::<Self>(state)?;
        check_modulus::<Self>(key)?;
        let mut led = Led::new(state, key);
        led.forward(self)?;
        Ok(led.into_state())
    }

    fn led128(&mut self, state: &[Self::Item], key: &[Self::Item]) -> Result<Vec<Self::Item>, Self::Error> {
        debug_assert_eq!(state.len(), 16);
        debug_assert_eq!(key.len(), 32);
        check_modulus::<Self>(state)?;
        check_modulus::<Self>(key)?;
        let mut led = Led::new(state, key);
        led.forward(self)?;
        Ok(led.into_state())
    }

    fn led64_bin(&mut self, state: &[BinaryBundle<Self::Item>], key: &[BinaryBundle<Self::Item>]) -> Result<Vec<BinaryBundle<Self::Item>>, Self::Error> {
        debug_assert_eq!(state.len(), 16);
        debug_assert_eq!(key.len(), 16);
        let mut led = LedBin::new(state, key);
        led.forward(self)?;
        Ok(led.into_state())
    }
}

#[cfg(test)]
mod tests {
    use crate::circuit::CircuitBuilder;
    use crate::classic::garble;
    use crate::Fancy;
    use crate::led::{LED_MODULUS, LedFancyExt, unfill_column_first};

    #[test]
    fn test_led64() {
        for (state,key,expected) in [
            ([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], vec![0x3,0x9,0xc,0x2,0x4,0x0,0x1,0x0,0x0,0x3,0xa,0x0,0xc,0x7,0x9,0x8]),
            ([0xa,0x7,0xf,0x1,0xd,0x9,0x2,0xa,0x8,0x2,0xc,0x8,0xd,0x8,0xf,0xe], [0x4,0x3,0x4,0xd,0x9,0x8,0x5,0x5,0x8,0xc,0xe,0x2,0xb,0x3,0x4,0x7], vec![0x9,0x1,0x7,0xe,0x9,0x0,0x6,0x9,0x4,0x1,0x5,0x5,0x4,0xf,0x9,0x7]),
            ([0x5,0x8,0xf,0x5,0x6,0xb,0xd,0x6,0x8,0x8,0x0,0x7,0x9,0x9,0x9,0x2], [0x4,0x8,0x3,0x3,0x6,0x2,0x4,0x1,0xf,0x3,0x0,0xd,0x2,0x3,0xe,0x5], vec![0x3,0x3,0x7,0x4,0x8,0xa,0x8,0xa,0xf,0xa,0xc,0x4,0x8,0x8,0x7,0xd]),
        ] {
            let mut b = CircuitBuilder::new();
            let input_state = state.iter().map(|c| b.constant(*c, &LED_MODULUS))
                .collect::<Result<Vec<_>,_>>().unwrap();
            let input_key = key.iter().map(|c| b.constant(*c, &LED_MODULUS))
                .collect::<Result<Vec<_>,_>>().unwrap();
            let output = b.led64(&input_state, &input_key).unwrap();
            assert_eq!(None,b.outputs(&output).unwrap());
            let circuit = b.finish();

            let (_, gc) = garble(&circuit).unwrap();
            let outputs = gc.eval(&circuit, &[], &[]).unwrap();
            assert_eq!(expected, outputs);
        }
    }

    #[test]
    fn test_led128() {
        for (state,key,expected) in [
            ([0x8,0x5,0x8,0x8,0x8,0x2,0x6,0xa,0x4,0x1,0x9,0xd,0x5,0x8,0x3,0x1], [0x1,0xc,0x1,0x7,0x8,0x4,0xb,0x5,0x4,0x8,0x4,0xe,0xe,0xc,0xd,0xb,0x3,0x9,0x3,0xf,0x6,0xa,0x0,0xa,0xc,0xa,0x1,0x1,0xb,0x9,0x1,0xd], vec![0x3,0xe,0x5,0x4,0xe,0x8,0x3,0x8,0x0,0xc,0xf,0x8,0xb,0xa,0x7,0xf]),
            ([0xf,0x0,0x8,0x6,0x6,0xb,0x5,0x0,0x0,0xb,0x8,0xd,0xe,0xe,0x5,0x0], [0x1,0xf,0xd,0x7,0xe,0xb,0x9,0xb,0xc,0xe,0x0,0x9,0xa,0x1,0x7,0xd,0x7,0x4,0x1,0x2,0x4,0xb,0x4,0x6,0x0,0x5,0xa,0xd,0xf,0xc,0x0,0x7], vec![0xc,0x0,0xd,0xe,0x2,0xc,0xe,0x8,0x3,0xa,0xc,0x9,0x4,0x5,0x0,0x0]),
        ] {
            let mut b = CircuitBuilder::new();
            let input_state = state.iter().map(|c| b.constant(*c, &LED_MODULUS))
                .collect::<Result<Vec<_>,_>>().unwrap();
            let input_key = key.iter().map(|c| b.constant(*c, &LED_MODULUS))
                .collect::<Result<Vec<_>,_>>().unwrap();
            let output = b.led128(&input_state, &input_key).unwrap();
            assert_eq!(None,b.outputs(&output).unwrap());
            let circuit = b.finish();

            let (_, gc) = garble(&circuit).unwrap();
            let outputs = gc.eval(&circuit, &[], &[]).unwrap();
            assert_eq!(expected, outputs);
        }
    }
}