use crate::{BinaryBundle, BinaryGadgets, Fancy};
use crate::primitives::utils;
use crate::primitives::utils::{aes_sbox, cmul_mod_x8_x4_x3_x_1, present_sbox};

const RC: [u8; 12] = [1, 3, 7, 14, 13, 11, 6, 12, 9, 2, 5, 10];

struct PhotonState<F: Fancy, const D: usize, const S: usize> {
    state: Vec<Vec<BinaryBundle<F::Item>>>,
    ic: [u8; D],
    zi: [usize; D],
}

impl<F: Fancy, const D: usize, const S: usize> PhotonState<F, D, S> {
    pub fn new(state: Vec<Vec<BinaryBundle<F::Item>>>, ic: [u8; D], zi: [usize; D]) -> Self {
        Self::check_dim(&state);
        Self {state, ic, zi}
    }

    fn check_dim(s: &Vec<Vec<BinaryBundle<F::Item>>>)  {
        debug_assert_eq!(s.len(), D);
        for i in 0..D {
            debug_assert_eq!(s[i].len(), D);
            for j in 0..D {
                debug_assert_eq!(s[i][j].size(), S);
            }
        }
    }

    fn add_constant(&mut self, f: &mut F, r: usize) -> Result<(), F::Error> {
        for i in 0..D {
            let rc = f.bin_constant_bundle((RC[r] ^ self.ic[i]) as u128, S)?;
            self.state[i][0] = f.bin_xor(&self.state[i][0], &rc)?;
            // for j in 0..S {
            //     if ((rc >> j) & 0x1) > 0 {
            //         [j] = f.negate(&self.state[i][0][j])?;
            //     }
            // }
        }
        Ok(())
    }

    fn sub_cells(&mut self, f: &mut F) -> Result<(), F::Error> {
        for x in 0..D {
            for y in 0..D {
                let cell = if S == 4 {
                    present_sbox(f, &self.state[x][y])?
                }else if S == 8 {
                    aes_sbox(f, &self.state[x][y])?
                }else{
                    panic!("Bit-width {} not supported", S)
                };
                self.state[x][y] = cell;
                // for (i,ci) in cell.into_iter().enumerate() {
                //     [i] = ci;
                // }
            }
        }
        Ok(())
    }

    fn shift_rows(&mut self) {
        let mut new_state = self.state.clone();
        for x in 1..D {
            for y in 0..D {
                new_state[x][y] = self.state[x][(y+x) % D].clone();
            }
        }
        self.state = new_state;
    }

    fn mix_columns_serial<C>(&mut self, f: &mut F, cmul: C) -> Result<(), F::Error>
        where C : Fn(&mut F, usize, &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error>
    {
        for _ in 0..D {
            let mut new_state = self.state.clone();
            for x in 0..(D-1) {
                new_state[x] = self.state[x+1].clone();
            }
            for y in 0..D {
                let mut res = cmul(f, self.zi[0], &self.state[0][y])?;
                for k in 1..D {
                    let to_add = cmul(f, self.zi[k], &self.state[k][y])?;
                    res = f.bin_xor(&res, &to_add)?;
                    // for i in 0..S {
                    //
                    // }
                }
                new_state[D-1][y] = res;
            }
            self.state = new_state;
        }
        Ok(())
    }

    pub fn forward<C>(&mut self, f: &mut F, cmul: C) -> Result<(), F::Error>
    where C : Fn(&mut F, usize, &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error> {
        for r in 0..12 {
            self.add_constant(f, r)?;
            self.sub_cells(f)?;
            self.shift_rows();
            self.mix_columns_serial(f, &cmul)?;
        }
        Ok(())
    }

    pub fn into_wires(self) -> Vec<Vec<BinaryBundle<F::Item>>> {
        self.state
    }
}

pub trait PhotonFancyExt : Fancy
{
    fn photon_100(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error>;

    fn photon_144(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error>;

    fn photon_196(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error>;

    fn photon_256(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error>;

    fn photon_288(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error>;
}

impl<F: Fancy> PhotonFancyExt for F {
    fn photon_100(&mut self, input: Vec<Vec<BinaryBundle<F::Item>>>) -> Result<Vec<Vec<BinaryBundle<F::Item>>>, Self::Error> {
        let mut state = PhotonState::<F,5,4>::new(input, [0,1,3,6,4], [1,2,9,9,2]);
        state.forward(self, utils::cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }

    fn photon_144(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::<F,6,4>::new(input, [0,1,3,7,6,4], [1,2,8,5,8,2]);
        state.forward(self, utils::cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }

    fn photon_196(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::<F,7,4>::new(input, [0,1,2,5,3,6,4], [1,4,6,1,1,6,4]);
        state.forward(self, utils::cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }

    fn photon_256(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::<F,8,4>::new(input, [0,1,3,7,15,14,12,8], [2,4,2,11,2,8,5,6]);
        state.forward(self, utils::cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }

    fn photon_288(&mut self, input: Vec<Vec<BinaryBundle<Self::Item>>>) -> Result<Vec<Vec<BinaryBundle<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::<F,6,8>::new(input, [0,1,3,7,6,4], [2,3,1,2,1,4]);
        state.forward(self, cmul_mod_x8_x4_x3_x_1)?;
        Ok(state.into_wires())
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::{BinaryBundle, BinaryGadgets, Fancy, Modulus};
    use crate::circuit::{CircuitBuilder, CircuitRef};
    use crate::classic::garble;
    use crate::primitives::photon_bin::{present_sbox};
    use super::PhotonFancyExt;

    fn fill_nbit<F, T, const D: usize>(bytes: &[u8], f: &mut F) -> Vec<Vec<T>>
    where F: FnMut(u8) -> T {
        assert_eq!(bytes.len(), D*D);
        let mut v = Vec::with_capacity(D);
        let mut cnt = 0;
        for _ in 0..D {
            let mut row = Vec::with_capacity(D);
            for _ in 0..D {
                let x = bytes[cnt];
                cnt += 1;
                row.push(f(x));
            }
            v.push(row);
        }
        return v;
    }

    fn test_photon_nbit<const D: usize, P>(photon: &mut P, input: &[u8], expected_output: &[u8], n: usize)
    where P: FnMut(&mut CircuitBuilder, Vec<Vec<BinaryBundle<CircuitRef>>>) -> Result<Vec<Vec<BinaryBundle<CircuitRef>>>, <CircuitBuilder as Fancy>::Error> {
        let mut b = CircuitBuilder::new();
        let input_wires = fill_nbit::<_, _, D>(input, &mut |i| b.bin_constant_bundle(i as u128, n).unwrap());
        let output_wires: Vec<_> = photon(&mut b, input_wires).unwrap()
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(None, b.bin_outputs(&output_wires).unwrap());
        let circuit = b.finish();
        let (_, gc) = garble(&circuit).unwrap();
        let output = gc.eval(&circuit, &[], &[]).unwrap();
        let output_u8: Vec<_> = output.into_iter().map(|i| {
            assert!(i < 2);
            i as u8
        })
            .collect();
        let expected_output: Vec<_> = fill_nbit::<_,_,D>(expected_output, &mut |x |(0..n).map(|i| (x >> i) & 0x1).collect_vec())
            .into_iter()
            .flatten()
            .flatten()
            .collect();
        assert_eq!(expected_output, output_u8);
    }

    #[test]
    fn test_photon_100() {
        const INPUT: [u8; 25] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,4,1,4,1,0];
        const OUTPUT: [u8; 25] = [0x3,0x3,0xd,0x5,0xf,0x6,0x2,0x9,0xb,0x9,0x5,0xc,0x4,0x8,0x1,0x6,0x5,0xc,0xe,0x7,0xb,0x7,0x7,0x0,0xc];
        test_photon_nbit::<5, _>(&mut CircuitBuilder::photon_100, &INPUT, &OUTPUT, 4);
    }

    #[test]
    fn test_photon_144() {
        const INPUT: [u8; 36] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,1,0,1,0];
        const OUTPUT: [u8; 36] = [0x9,0x5,0xf,0xc,0x3,0xc,0xe,0x2,0x2,0xa,0x2,0xa,0x6,0x3,0x2,0xd,0x6,0xf,0xe,0xb,0x4,0xe,0x0,0xb,0x6,0x2,0x5,0x9,0x2,0xd,0x8,0xd,0x0,0x3,0x2,0x9];
        test_photon_nbit::<6,_>(&mut CircuitBuilder::photon_144, &INPUT, &OUTPUT, 4);
    }

    #[test]
    fn test_photon_196() {
        const INPUT: [u8; 49] = [0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,0,0,0,0,0,0, 0,2,8,2,4,2,4];
        const OUTPUT: [u8; 49] = [
            0x1,0xf,0x0,0xd,0x4,0xa,0x1,
            0xd,0xd,0x0,0xa,0x3,0x1,0xd,
            0xe,0xc,0xf,0x5,0xb,0x6,0x9,
            0xb,0x6,0x6,0xe,0x0,0xc,0x8,
            0xf,0x6,0x4,0x4,0xc,0xe,0xe,
            0xe,0x9,0x0,0x2,0x0,0xf,0x4,
            0x3,0xa,0x9,0xd,0xe,0x7,0x4
        ];
        test_photon_nbit::<7,_>(&mut CircuitBuilder::photon_196, &INPUT, &OUTPUT, 4);
    }

    #[test]
    fn test_photon_256() {
        const INPUT: [u8; 64] = [0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,3,8,2,0,2,0];
        const OUTPUT: [u8; 64] = [
            0x1, 0x7, 0x3, 0x0, 0x4, 0x2, 0x4, 0x2,
            0x9, 0xc, 0xf, 0x2, 0x6, 0xe, 0x1, 0x0,
            0x8, 0xd, 0x3, 0xd, 0x9, 0xc, 0xf, 0x9,
            0x0, 0x0, 0xe, 0x2, 0x7, 0xb, 0xd, 0xc,
            0xc, 0x6, 0x2, 0x9, 0xb, 0x3, 0xd, 0x1,
            0xa, 0xf, 0x4, 0x1, 0xf, 0x1, 0xc, 0xb,
            0x7, 0x4, 0x8, 0x3, 0xf, 0xc, 0xc, 0x0,
            0x8, 0x9, 0x1, 0x6, 0xb, 0x8, 0x2, 0xc
        ];
        test_photon_nbit::<8,_>(&mut CircuitBuilder::photon_256, &INPUT, &OUTPUT, 4);
    }

    #[test]
    fn test_photon_288() {
        const INPUT: [u8; 36] = [0,0,0,0,0,0, 0,0,0,0,0,0, 0,0,0,0,0,0, 0,0,0,0,0,0, 0,0,0,0,0,0, 0, 0, 0, 0x40, 0x20, 0x20];
        const OUTPUT: [u8; 36] = [
            0x4D, 0xBD, 0x90, 0x36, 0x1C, 0xB5,
            0xE0, 0x9E, 0x5C, 0x38, 0xA9, 0xC9,
            0xE9, 0xD5, 0x66, 0x08, 0xCF, 0x52,
            0xCB, 0x6B, 0xC8, 0x8B, 0x93, 0x16,
            0xE8, 0xC2, 0xC0, 0x69, 0x25, 0xF7,
            0x18, 0xCC, 0x62, 0x9C, 0xAE, 0x79
        ];
        test_photon_nbit::<6,_>(&mut CircuitBuilder::photon_288, &INPUT, &OUTPUT, 8);
    }
}