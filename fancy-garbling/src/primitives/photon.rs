use crate::Fancy;

const RC: [u8; 12] = [1, 3, 7, 14, 13, 11, 6, 12, 9, 2, 5, 10];

struct PhotonState<F: Fancy, const D: usize, const S: usize> {
    state: Vec<Vec<Vec<F::Item>>>,
    ic: [u8; D],
    zi: [usize; D],
}

impl<F: Fancy, const D: usize, const S: usize> PhotonState<F, D, S> {
    pub fn new(state: Vec<Vec<Vec<F::Item>>>, ic: [u8; D], zi: [usize; D]) -> Self {
        Self::check_dim(&state);
        Self {state, ic, zi}
    }

    fn check_dim(s: &Vec<Vec<Vec<F::Item>>>)  {
        debug_assert_eq!(s.len(), D);
        for i in 0..D {
            debug_assert_eq!(s[i].len(), D);
            for j in 0..D {
                debug_assert_eq!(s[i][j].len(), S);
            }
        }
    }

    fn add_constant(&mut self, f: &mut F, r: usize) -> Result<(), F::Error> {
        for i in 0..D {
            let rc = RC[r] ^ self.ic[i];
            for j in 0..S {
                if ((rc >> j) & 0x1) > 0 {
                    self.state[i][0][j] = f.negate(&self.state[i][0][j])?;
                }
            }
        }
        Ok(())
    }

    fn sub_cells(&mut self, f: &mut F) -> Result<(), F::Error> {
        for x in 0..D {
            for y in 0..D {
                let cell = if S == 4 {
                    present_sbox(f, &self.state[x][y])?
                }else{
                    panic!("Bit-width {} not supported", S)
                };
                for (i,ci) in cell.into_iter().enumerate() {
                    self.state[x][y][i] = ci;
                }
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
        where C : Fn(&mut F, usize, &[F::Item]) -> Result<[F::Item; S], F::Error>
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
                    for i in 0..S {
                        res[i] = f.add(&res[i], &to_add[i])?;
                    }
                }
                new_state[D-1][y] = res.into();
            }
            self.state = new_state;
        }
        Ok(())
    }

    pub fn forward<C>(&mut self, f: &mut F, cmul: C) -> Result<(), F::Error>
    where C : Fn(&mut F, usize, &[F::Item]) -> Result<[F::Item; S], F::Error> {
        for r in 0..12 {
            self.add_constant(f, r)?;
            self.sub_cells(f)?;
            self.shift_rows();
            self.mix_columns_serial(f, &cmul)?;
        }
        Ok(())
    }

    pub fn into_wires(self) -> Vec<Vec<Vec<F::Item>>> {
        self.state
    }
}

fn present_sbox<F: Fancy>(f: &mut F, x: &[F::Item]) -> Result<Vec<F::Item>, F::Error> {
    debug_assert_eq!(4, x.len());
    let f0 = &x[1];
    let f1 = &x[3];
    let f2 = &x[0];
    let f3 = &x[2];

    let f3 = f.add(f3, f0)?;
    let tmp = f.and(f0, &f3)?;
    let f1 = f.add(f1, &tmp)?;
    let f3 = f.add(&f3, &f1)?;
    let f3 = f.negate(&f3)?;
    let f2 = f.negate(&f2)?;
    let tmp = f.and(&f1, &f3)?;
    let f0 = f.add(&f0, &tmp)?;
    let tmp = f.and(&f0, &f2)?;
    let f1 = f.add(&f1, &tmp)?;
    let f0 = f.add(&f0, &f2)?;
    let f2 = f.add(&f2, &f3)?;
    let f0 = f.add(&f0, &f1)?;
    let tmp = f.and(&f0, &f1)?;
    let f3 = f.add(&f3, &tmp)?;

    Ok(vec!(f2, f1, f3, f0))
}

fn cmul_mod_x4_x_1<F: Fancy>(f: &mut F, c: usize, cell: &[F::Item]) -> Result<[F::Item; 4], F::Error> {
    debug_assert_eq!(4, cell.len());
    match c {
        1 => Ok([cell[0].clone(), cell[1].clone(), cell[2].clone(), cell[3].clone()]),
        2 => {
            Ok([cell[3].clone(), f.add(&cell[0], &cell[3])?, cell[1].clone(), cell[2].clone()])
        },
        4 => {
          Ok([cell[2].clone(), f.add(&cell[2], &cell[3])?, f.add(&cell[0], &cell[3])?, cell[1].clone()])
        },
        5 => {
            let cell02 = f.add(&cell[0], &cell[2])?;
            let cell13 = f.add(&cell[1], &cell[3])?;
            let cell023 = f.add(&cell02, &cell[3])?;
            Ok([cell02, f.add(&cell13, &cell[2])?, cell023, cell13])
        },
        6 => {
            let cell23 = f.add(&cell[2], &cell[3])?;
            let cell02 = f.add(&cell[0], &cell[2])?;
            let cell13 = &f.add(&cell[1], &cell[3])?;
            let cell12 = f.add(&cell[1], &cell[2])?;
            Ok([cell23, cell02, f.add(&cell[0], &cell13)?, cell12])
        },
        8 => {
          Ok([cell[1].clone(), f.add(&cell[1], &cell[2])?, f.add(&cell[2], &cell[3])?, f.add(&cell[0], &cell[3])?])
        },
        9 => {
            Ok([f.add(&cell[0], &cell[1])?, cell[2].clone(), cell[3].clone(), cell[0].clone()])
        },
        11 => {
            let cell02 = f.add(&cell[0], &cell[2])?;
            let cell13 = f.add(&cell[1], &cell[3])?;
            let cell023 = f.add(&cell02, &cell[3])?;
            let cell013 = f.add(&cell13, &cell[0])?;
            Ok([cell013, cell023, cell13, cell02])
        }
        _ => panic!("cmul by {} not supported", c),
    }
}

pub trait PhotonFancyExt : Fancy
{
    fn photon_100(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error>;

    fn photon_144(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error>;

    fn photon_196(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error>;

    fn photon_256(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error>;
}

impl<F: Fancy> PhotonFancyExt for F {
    fn photon_100(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::new(input, [0,1,3,6,4], [1,2,9,9,2]);
        state.forward(self, cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }

    fn photon_144(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::new(input, [0,1,3,7,6,4], [1,2,8,5,8,2]);
        state.forward(self, cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }

    fn photon_196(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::new(input, [0,1,2,5,3,6,4], [1,4,6,1,1,6,4]);
        state.forward(self, cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }

    fn photon_256(&mut self, input: Vec<Vec<Vec<Self::Item>>>) -> Result<Vec<Vec<Vec<Self::Item>>>, Self::Error> {
        let mut state = PhotonState::new(input, [0,1,3,7,15,14,12,8], [2,4,2,11,2,8,5,6]);
        state.forward(self, cmul_mod_x4_x_1)?;
        Ok(state.into_wires())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Fancy, Modulus};
    use crate::circuit::{CircuitBuilder, CircuitRef};
    use crate::classic::garble;
    use crate::primitives::photon::{cmul_mod_x4_x_1, present_sbox};
    use super::PhotonFancyExt;

    fn fill_4bit<F, T, const D: usize>(bytes: &[u8], f: &mut F) -> Vec<Vec<Vec<T>>>
    where F: FnMut(u8) -> T {
        assert_eq!(bytes.len(), D*D);
        let mut v = Vec::with_capacity(D);
        let mut cnt = 0;
        for _ in 0..D {
            let mut row = Vec::with_capacity(D);
            for _ in 0..D {
                let x = bytes[cnt];
                cnt += 1;
                let cell: Vec<_> = (0..4).map(|i| f((x >> i) & 0x1))
                    .collect();
                row.push(cell);
            }
            v.push(row);
        }
        return v;
    }

    #[test]
    fn test_present_sbox() {
        const PRESENT_SBOX: [u16; 16] = [0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let circuit = {
            let mut b = CircuitBuilder::new();
            let inputs = b.garbler_inputs(&[Modulus::Zq {q:2}, Modulus::Zq {q:2}, Modulus::Zq {q:2}, Modulus::Zq {q:2}]);
            let outputs = present_sbox(&mut b, &inputs).unwrap();
            assert_eq!(b.outputs(&outputs).unwrap(), None);
            b.finish()
        };
        let (enc, gc) = garble(&circuit).unwrap();
        for (x,y) in PRESENT_SBOX.iter().enumerate() {
            let x = x as u16;
            let inputs = enc.encode_garbler_inputs(&[x & 0x1, (x >> 1) & 0x1, (x >> 2) & 0x1, (x >> 3) & 0x1]);
            let outputs = gc.eval(&circuit, &inputs, &[]).unwrap();
            assert_eq!(&outputs, &[y & 0x1, (y >> 1) & 0x1, (y >> 2) & 0x1, (y >> 3) & 0x1]);
        }
    }

    #[test]
    fn test_cmul_mod_x4_x_1() {
        const CMUL_1: [u16; 16] = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf];
        const CMUL_2: [u16; 16] = [0x0, 0x2, 0x4, 0x6, 0x8, 0xa, 0xc, 0xe, 0x3, 0x1, 0x7, 0x5, 0xb, 0x9, 0xf, 0xd];
        const CMUL_4: [u16; 16] = [0x0, 0x4, 0x8, 0xc, 0x3, 0x7, 0xb, 0xf, 0x6, 0x2, 0xe, 0xa, 0x5, 0x1, 0xd, 0x9];
        const CMUL_5: [u16; 16] = [0x0, 0x5, 0xa, 0xf, 0x7, 0x2, 0xd, 0x8, 0xe, 0xb, 0x4, 0x1, 0x9, 0xc, 0x3, 0x6];
        const CMUL_6: [u16; 16] = [0x0, 0x6, 0xc, 0xa, 0xb, 0xd, 0x7, 0x1, 0x5, 0x3, 0x9, 0xf, 0xe, 0x8, 0x2, 0x4];
        const CMUL_8: [u16; 16] = [0x0, 0x8, 0x3, 0xb, 0x6, 0xe, 0x5, 0xd, 0xc, 0x4, 0xf, 0x7, 0xa, 0x2, 0x9, 0x1];
        const CMUL_9: [u16; 16] = [0x0, 0x9, 0x1, 0x8, 0x2, 0xb, 0x3, 0xa, 0x4, 0xd, 0x5, 0xc, 0x6, 0xf, 0x7, 0xe];
        const CMUL_11: [u16; 16] = [0x0, 0xb, 0x5, 0xe, 0xa, 0x1, 0xf, 0x4, 0x7, 0xc, 0x2, 0x9, 0xd, 0x6, 0x8, 0x3];

        fn test_cmul(c: usize, expected: &[u16; 16]) {
            let circuit = {
                let mut b = CircuitBuilder::new();
                let inputs = b.garbler_inputs(&[Modulus::Zq {q:2}, Modulus::Zq {q:2}, Modulus::Zq {q:2}, Modulus::Zq {q:2}]);
                let outputs = cmul_mod_x4_x_1(&mut b, c, &inputs).unwrap();
                assert_eq!(b.outputs(&outputs).unwrap(), None);
                b.finish()
            };
            let (enc, gc) = garble(&circuit).unwrap();
            for (x,y) in expected.iter().enumerate() {
                let x = x as u16;
                let inputs = enc.encode_garbler_inputs(&[x & 0x1, (x >> 1) & 0x1, (x >> 2) & 0x1, (x >> 3) & 0x1]);
                let outputs = gc.eval(&circuit, &inputs, &[]).unwrap();
                assert_eq!(&outputs, &[y & 0x1, (y >> 1) & 0x1, (y >> 2) & 0x1, (y >> 3) & 0x1]);
            }
        }

        test_cmul(1, &CMUL_1);
        test_cmul(2, &CMUL_2);
        test_cmul(4, &CMUL_4);
        test_cmul(5, &CMUL_5);
        test_cmul(6, &CMUL_6);
        test_cmul(8, &CMUL_8);
        test_cmul(9, &CMUL_9);
        test_cmul(11, &CMUL_11);

    }

    fn test_photon_4bit<const D: usize, P>(photon: &mut P, input: &[u8], expected_output: &[u8])
    where P: FnMut(&mut CircuitBuilder, Vec<Vec<Vec<CircuitRef>>>) -> Result<Vec<Vec<Vec<CircuitRef>>>, <CircuitBuilder as Fancy>::Error> {
        let mut b = CircuitBuilder::new();
        let input_wires = fill_4bit::<_, _, D>(input, &mut |i| b.constant(i as u16, &Modulus::Zq {q: 2}).unwrap());
        let output_wires: Vec<_> = photon(&mut b, input_wires).unwrap()
            .into_iter()
            .flatten()
            .flatten()
            .collect();
        assert_eq!(None, b.outputs(&output_wires).unwrap());
        let circuit = b.finish();
        let (_, gc) = garble(&circuit).unwrap();
        let output = gc.eval(&circuit, &[], &[]).unwrap();
        let output_u8: Vec<_> = output.into_iter().map(|i| {
            assert!(i < 2);
            i as u8
        })
            .collect();
        let expected_output: Vec<_> = fill_4bit::<_,_,D>(expected_output, &mut |i| i)
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
        test_photon_4bit::<5, _>(&mut CircuitBuilder::photon_100, &INPUT, &OUTPUT);
    }

    #[test]
    fn test_photon_144() {
        const INPUT: [u8; 36] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,1,0,1,0];
        const OUTPUT: [u8; 36] = [0x9,0x5,0xf,0xc,0x3,0xc,0xe,0x2,0x2,0xa,0x2,0xa,0x6,0x3,0x2,0xd,0x6,0xf,0xe,0xb,0x4,0xe,0x0,0xb,0x6,0x2,0x5,0x9,0x2,0xd,0x8,0xd,0x0,0x3,0x2,0x9];
        test_photon_4bit::<6,_>(&mut CircuitBuilder::photon_144, &INPUT, &OUTPUT);
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
        test_photon_4bit::<7,_>(&mut CircuitBuilder::photon_196, &INPUT, &OUTPUT);
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
        test_photon_4bit::<8,_>(&mut CircuitBuilder::photon_256, &INPUT, &OUTPUT);
    }
}