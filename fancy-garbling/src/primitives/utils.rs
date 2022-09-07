use crate::{BinaryBundle, BinaryGadgets, BundleGadgets, Fancy, HasModulus, Modulus};
use crate::primitives::utils;

pub const SBOX_PRESENT: &[u16] =  &[0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];

pub fn aes_sbox<F: Fancy>(f: &mut F, x: &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error> {
    debug_assert_eq!(x.size(), 8);
    const U: [[u8; 8]; 22] = [
        [1, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 1, 1, 0],
        [1, 0, 0, 0, 0, 1, 1, 1],
        [1, 1, 1, 0, 0, 1, 1, 1],
        [1, 0, 0, 0, 1, 1, 1, 0],
        [1, 1, 0, 0, 0, 1, 1, 0],
        [1, 1, 0, 1, 1, 0, 0, 1],
        [1, 1, 1, 1, 0, 0, 1, 0],
        [0, 0, 1, 0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1, 0, 0, 1],
        [0, 1, 0, 1, 1, 1, 1, 1],
        [0, 1, 1, 1, 0, 0, 1, 0],
        [0, 1, 1, 0, 1, 0, 0, 1],
        [0, 1, 0, 0, 0, 0, 0, 1],
        [0, 0, 1, 0, 1, 0, 0, 0],
        [0, 1, 0, 1, 1, 0, 0, 1],
        [0, 1, 1, 1, 0, 1, 0, 0],
        [0, 0, 1, 0, 1, 1, 0, 1],
        [0, 1, 1, 1, 0, 1, 0, 1],
        [0, 1, 1, 1, 1, 1, 1, 0],
        [0, 1, 1, 1, 1, 0, 1, 1],
        [0, 0, 1, 1, 0, 1, 0, 1],
    ];
    const B: [[u8; 18]; 8] = [
        [0, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0],
        [1, 1, 0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0],
        [1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0, 1],
        [1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0],
        [0, 1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0],
        [1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0],
        [0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 1, 1, 0],
        [1, 0, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 1, 0]
    ];
    const N: [u8; 8] = [1, 1, 0, 0, 0, 1, 1, 0];
    fn mat_mul<F: Fancy, const D1: usize, const D2: usize>(f: &mut F, m : &[[u8; D1]; D2], x: &[F::Item]) -> Result<Vec<F::Item>, F::Error> {
        let mut res = Vec::with_capacity(x.len());
        for mi in m {
            let mut resi = None;
            for (j,mij) in mi.iter().enumerate() {
                if *mij > 0 {
                    resi = match resi {
                        Some(r) => Some(f.add(&r, &x[j])?),
                        None => Some(x[j].clone())
                    }
                }
            }
            res.push(resi.expect("Matrix row didn't contain a one"));
        }
        Ok(res)
    }

    let y = mat_mul(f, &U, &x.wires())?;
    let t2 = f.and(&y[12], &y[15])?;
    let t3 = f.and(&y[3], &y[6])?;
    let t4 = f.add(&t2, &t3)?;
    let t5 = f.and(&y[4], &y[0])?;
    let t6 = f.add(&t2, &t5)?;
    let t7 = f.and(&y[13], &y[16])?;
    let t8 = f.and(&y[5], &y[1])?;
    let t9 = f.add(&t7, &t8)?;
    let t10 = f.and(&y[2], &y[7])?;
    let t11 = f.add(&t7, &t10)?;
    let t12 = f.and(&y[9], &y[11])?;
    let t13 = f.and(&y[14], &y[17])?;
    let t14 = f.add(&t12, &t13)?;
    let t15 = f.and(&y[8], &y[10])?;
    let t16 = f.add(&t12, &t15)?;
    let t17 = f.add(&t4, &t14)?;
    let t18 = f.add(&t6, &t16)?;
    let t19 = f.add(&t9, &t14)?;
    let t20 = f.add(&t11, &t16)?;
    let t21 = f.add(&t17, &y[20])?;
    let t22 = f.add(&t18, &y[19])?;
    let t23 = f.add(&t19, &y[21])?;
    let t24 = f.add(&t20, &y[18])?;
    let t25 = f.add(&t21, &t22)?;
    let t26 = f.and(&t21, &t23)?;
    let t27 = f.add(&t24, &t26)?;
    let t28 = f.and(&t25, &t27)?;
    let t29 = f.add(&t28, &t22)?;
    let t30 = f.add(&t23, &t24)?;
    let t31 = f.add(&t22, &t26)?;
    let t32 = f.and(&t31, &t30)?;
    let t33 = f.add(&t32, &t24)?;
    let t34 = f.add(&t23, &t33)?;
    let t35 = f.add(&t27, &t33)?;
    let t36 = f.and(&t24, &t35)?;
    let t37 = f.add(&t36, &t34)?;
    let t38 = f.add(&t27, &t36)?;
    let t39 = f.and(&t29, &t38)?;
    let t40 = f.add(&t25, &t39)?;
    let t41 = f.add(&t40, &t37)?;
    let t42 = f.add(&t29, &t33)?;
    let t43 = f.add(&t29, &t40)?;
    let t44 = f.add(&t33, &t37)?;
    let t45 = f.add(&t42, &t41)?;
    let z0 = f.and(&t44, &y[15])?;
    let z1 = f.and(&t37, &y[6])?;
    let z2 = f.and(&t33, &y[0])?;
    let z3 = f.and(&t43, &y[16])?;
    let z4 = f.and(&t40, &y[1])?;
    let z5 = f.and(&t29, &y[7])?;
    let z6 = f.and(&t42, &y[11])?;
    let z7 = f.and(&t45, &y[17])?;
    let z8 = f.and(&t41, &y[10])?;
    let z9 = f.and(&t44, &y[12])?;
    let z10 = f.and(&t37, &y[3])?;
    let z11 = f.and(&t33, &y[4])?;
    let z12 = f.and(&t43, &y[13])?;
    let z13 = f.and(&t40, &y[5])?;
    let z14 = f.and(&t29, &y[2])?;
    let z15 = f.and(&t42, &y[9])?;
    let z16 = f.and(&t45, &y[14])?;
    let z17 = f.and(&t41, &y[8])?;
    let z = [z0, z1, z2, z3, z4, z5, z6, z7, z8, z9, z10, z11, z12, z13, z14, z15, z16, z17];
    let s = mat_mul(f, &B, &z)?;
    s.into_iter().rev().zip(&N)
        .map(|(si, ni)| {
            if *ni == 0 {
                Ok(si)
            }else{
                f.negate(&si)
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|wires| BinaryBundle::new(wires))
}

pub fn present_sbox<F: Fancy>(f: &mut F, x: &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error> {
    debug_assert_eq!(4, x.size());
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

    Ok(BinaryBundle::new(vec!(f2, f1, f3, f0)))
}

pub fn sum<'a, F: Fancy, I: IntoIterator<Item=&'a F::Item>>(f: &mut F, it: I, modulus: &Modulus) -> Result<F::Item,F::Error>
    where <F as Fancy>::Item: 'a {
    let mut it = it.into_iter();
    match it.next() {
        None => Ok(f.constant(0, modulus)?),
        Some(first) => {
            let mut res = first.clone();
            for rest in it {
                res = f.add(&res, rest)?;
            }
            Ok(res)
        }
    }
}

pub fn xor_bundle<'a, F: Fancy, I: IntoIterator<Item=&'a BinaryBundle<F::Item>>>(f: &mut F, it: I, bundle_size: usize) -> Result<BinaryBundle<F::Item>,F::Error>
    where <F as Fancy>::Item: 'a {
    let mut it = it.into_iter();
    match it.next() {
        None => Ok(f.bin_constant_bundle(0, bundle_size)?),
        Some(first) => {
            let mut res = first.clone();
            for rest in it {
                res = f.bin_xor(&res, rest)?;
            }
            Ok(res)
        }
    }
}

pub fn cmul_mod_x4_x_1<F: Fancy>(f: &mut F, c: usize, cell: &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error> {
    debug_assert_eq!(4, cell.size());
    match c {
        1 => Ok(cell.clone()),
        2 => {
            Ok(BinaryBundle::new(vec![cell[3].clone(), f.add(&cell[0], &cell[3])?, cell[1].clone(), cell[2].clone()]))
        },
        4 => {
            Ok(BinaryBundle::new(vec![cell[2].clone(), f.add(&cell[2], &cell[3])?, f.add(&cell[0], &cell[3])?, cell[1].clone()]))
        },
        5 => {
            let cell02 = f.add(&cell[0], &cell[2])?;
            let cell13 = f.add(&cell[1], &cell[3])?;
            let cell023 = f.add(&cell02, &cell[3])?;
            Ok(BinaryBundle::new(vec![cell02, f.add(&cell13, &cell[2])?, cell023, cell13]))
        },
        6 => {
            let cell23 = f.add(&cell[2], &cell[3])?;
            let cell02 = f.add(&cell[0], &cell[2])?;
            let cell13 = &f.add(&cell[1], &cell[3])?;
            let cell12 = f.add(&cell[1], &cell[2])?;
            Ok(BinaryBundle::new(vec![cell23, cell02, f.add(&cell[0], &cell13)?, cell12]))
        },
        8 => {
            Ok(BinaryBundle::new(vec![cell[1].clone(), f.add(&cell[1], &cell[2])?, f.add(&cell[2], &cell[3])?, f.add(&cell[0], &cell[3])?]))
        },
        9 => {
            Ok(BinaryBundle::new(vec![f.add(&cell[0], &cell[1])?, cell[2].clone(), cell[3].clone(), cell[0].clone()]))
        },
        11 => {
            let cell02 = f.add(&cell[0], &cell[2])?;
            let cell13 = f.add(&cell[1], &cell[3])?;
            let cell023 = f.add(&cell02, &cell[3])?;
            let cell013 = f.add(&cell13, &cell[0])?;
            Ok(BinaryBundle::new(vec![cell013, cell023, cell13, cell02]))
        },
        _ => panic!("cmul by {} not supported", c),
    }
}

pub fn cmul_mod_x8_x4_x3_x_1<F: Fancy>(f: &mut F, c: usize, cell: &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error> {
    match c {
        1 => Ok(cell.clone()),
        2 => {
            let cell37 = f.add(&cell[3], &cell[7])?;
            let cell27 = f.add(&cell[2], &cell[7])?;
            let cell07 = f.add(&cell[0], &cell[7])?;
            Ok(BinaryBundle::new(vec![cell[7].clone(), cell07, cell[1].clone(), cell27, cell37, cell[4].clone(), cell[5].clone(), cell[6].clone()]))
        },
        3 => {
            let cell67 = f.add(&cell[6], &cell[7])?;
            let cell56 = f.add(&cell[5], &cell[6])?;
            let cell45 = f.add(&cell[4], &cell[5])?;
            let cell37 = f.add(&cell[3], &cell[7])?;
            let cell347 = f.add(&cell37, &cell[4])?;
            let cell237 = f.add(&cell[2], &cell37)?;
            let cell12 = f.add(&cell[1], &cell[2])?;
            let cell07 = f.add(&cell[0], &cell[7])?;
            let cell017 = f.add(&cell07, &cell[1])?;
            Ok(BinaryBundle::new(vec![cell07, cell017, cell12, cell237, cell347, cell45, cell56, cell67]))
        },
        4 => {
            let cell37 = f.add(&cell[3], &cell[7])?;
            let cell67 = f.add(&cell[6], &cell[7])?;
            let cell267 = f.add(&cell[2], &cell67)?;
            let cell16 = f.add(&cell[1], &cell[6])?;
            let cell07 = f.add(&cell[0], &cell[7])?;
            Ok(BinaryBundle::new(vec![cell[6].clone(), cell67, cell07, cell16, cell267, cell37, cell[4].clone(), cell[5].clone()]))
        }
        _ => panic!("cmul by {} not supported", c),
    }
}

pub fn sbox_layer_proj<F: Fancy>(f: &mut F, state: &mut Vec<F::Item>, modulus: &Modulus, tt: &[u16]) -> Result<(), F::Error> {
    for cell in state.iter_mut() {
        *cell = f.proj(cell, modulus, Some(tt.to_vec()))?;
    }
    Ok(())
}

pub fn sbox_layer_bin<F: Fancy, S>(f: &mut F, state: &mut Vec<BinaryBundle<F::Item>>, sbox: S) -> Result<(), F::Error>
where S: Fn(&mut F, &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error> {
    for cell in state.iter_mut() {
        *cell = sbox(f, cell)?
    }
    Ok(())
}

pub fn add_roundkey_layer<F: Fancy>(f: &mut F, state: &mut [F::Item], roundkey: &[F::Item]) -> Result<(), F::Error> {
    debug_assert_eq!(state.len(), roundkey.len());
    for (cell, rk) in state.iter_mut().zip(roundkey) {
        *cell = f.add(cell, rk)?;
    }
    Ok(())
}

pub fn add_roundkey_layer_bin<F: Fancy>(f: &mut F, state: &mut [BinaryBundle<F::Item>], roundkey: &[BinaryBundle<F::Item>]) -> Result<(), F::Error> {
    debug_assert_eq!(state.len(), roundkey.len());
    for (cell, rk) in state.iter_mut().zip(roundkey) {
        *cell = f.bin_xor(cell, rk)?;
    }
    Ok(())
}

pub fn add_constants_layer<F: Fancy>(f: &mut F, state: &mut Vec<F::Item>, constants: &[u16], modulus: &Modulus) -> Result<(), F::Error> {
    debug_assert!(state.len() >= constants.len());
    for (cell, rc) in state.iter_mut().zip(constants) {
        let rc = f.constant(*rc, modulus)?;
        *cell = f.add(cell, &rc)?;
    }
    Ok(())
}

pub fn add_constants_layer_bin<F: Fancy>(f: &mut F, state: &mut Vec<BinaryBundle<F::Item>>, constants: &[u16]) -> Result<(), F::Error> {
    debug_assert!(state.len() >= constants.len());
    for (cell, rc) in state.iter_mut().zip(constants) {
        let rc = f.bin_constant_bundle(*rc as u128, cell.size())?;
        *cell = f.bin_xor(cell, &rc)?;
    }
    Ok(())
}

pub fn permute_state<T: Clone>(state: &Vec<T>, permutation: &[usize]) -> Vec<T> {
    debug_assert_eq!(state.len(), permutation.len());
    (0..state.len()).map(|i| state[permutation[i]].clone())
        .collect()
}

///
/// state: column-first nxn matrix
fn mix_columns_mds_alg<F: Fancy, T: Clone, C, A>(f: &mut F, state: &mut Vec<T>, n: usize, mds_last_row: &[u16], pow: usize, cmul: C, add: A) -> Result<(), F::Error>
where C: Fn(&mut F, &T, u16) -> Result<T, F::Error>,
A: Fn(&mut F, &T, &T) -> Result<T, F::Error>,
{
    let mut last_row: Vec<T> = Vec::with_capacity(n);

    let mut el: T;

    let perm = (0..n*n).map(|i| {
        if (i+1) % n == 0 {
            i-(n-1)
        }else{
            i+1
        }
    }).collect::<Vec<_>>();
    for _ in 0..pow {
        for i in 0..n {
            let mut sum =
                if mds_last_row[0] != 1 {
                    cmul(f, &state[n*i], mds_last_row[0])?
                }else{
                    state[n*i].clone()
                };
            for j in 1..n {
                if mds_last_row[j] != 1 {
                    el = cmul(f, &state[n*i+j],mds_last_row[j])?;
                } else {
                    el = state[n*i+j].clone();
                }
                sum = add(f, &sum, &el)?;
            }
            last_row.push(sum);
        }
        *state = utils::permute_state(state, &perm);
        for i in 0..n {
            state[i*n+n-1] = last_row[i].clone();
        }
        last_row.clear();
    }

    Ok(())
}

///
/// state: column-first nxn matrix
pub fn mix_columns_mds<F: Fancy>(f: &mut F, state: &mut Vec<F::Item>, n: usize, mds_last_row: &[u16], pow: usize) -> Result<(), F::Error> {
    debug_assert!(state.iter().all(|x| x.modulus().is_field() || x.modulus().value() > 2));
    mix_columns_mds_alg(f, state, n, mds_last_row, pow, <F as Fancy>::cmul, <F as Fancy>::add)
    // let mut last_row: Vec<F::Item> = Vec::with_capacity(n);
    //
    // let mut el: F::Item;
    //
    // let perm = (0..n*n).map(|i| {
    //     if (i+1) % n == 0 {
    //         i-(n-1)
    //     }else{
    //         i+1
    //     }
    // }).collect::<Vec<_>>();
    // for _ in 0..pow {
    //     for i in 0..n {
    //         let mut sum =
    //             if mds_last_row[0] != 1 {
    //                 f.cmul(&state[n*i], mds_last_row[0])?
    //             }else{
    //                 state[n*i].clone()
    //             };
    //         for j in 1..n {
    //             if mds_last_row[j] != 1 {
    //                 el = f.cmul(&state[n*i+j],mds_last_row[j])?;
    //             } else {
    //                 el = state[n*i+j].clone();
    //             }
    //             sum = f.add(&sum, &el)?;
    //         }
    //         last_row.push(sum);
    //     }
    //     *state = utils::permute_state(state, &perm);
    //     for i in 0..n {
    //         state[i*n+n-1] = last_row[i].clone();
    //     }
    //     last_row.clear();
    // }
    //
    // Ok(())
}

pub fn mix_columns_mds_bin<F: Fancy, C>(f: &mut F, state: &mut Vec<BinaryBundle<F::Item>>, n: usize, mds_last_row: &[u16], pow: usize, cmul: C) -> Result<(), F::Error>
where C: Fn(&mut F, usize, &BinaryBundle<F::Item>) -> Result<BinaryBundle<F::Item>, F::Error>
{
    mix_columns_mds_alg(f, state, n, mds_last_row, pow, |f,cell,c| cmul(f,c as usize,cell), <F as BinaryGadgets>::bin_xor)
}

///
/// matrix: row-first matrix
pub fn matrix_vec_mul<F: Fancy>(f: &mut F, matrix: &[u16], vector: &[F::Item], modulus: &Modulus) -> Result<Vec<F::Item>, F::Error> {
    let n = vector.len();
    debug_assert_eq!(matrix.len(), n*n);
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let to_sum = vector.iter().zip(&matrix[n*i..n*i+n]).map(|(x, c)| {
            match c {
                0 => None,
                1 => Some(Ok(x.clone())),
                _ => Some(f.cmul(x, *c))
            }
        })
            .filter_map(|x| x)
            .collect::<Result<Vec<_>,_>>()?;
        result.push(sum(f, &to_sum, modulus)?)
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::circuit::CircuitBuilder;
    use crate::{BinaryGadgets, Fancy, Modulus};
    use crate::classic::garble;
    use crate::dummy::Dummy;
    use crate::primitives::utils::{aes_sbox, cmul_mod_x4_x_1, cmul_mod_x8_x4_x3_x_1, mix_columns_mds, present_sbox};

    #[test]
    fn test_aes_sbox() {
        const AES_SBOX: [u8; 256] = [
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
        let circuit = {
            let mut b = CircuitBuilder::new();
            let inputs = b.bin_garbler_input(8);
            let outputs = aes_sbox(&mut b, &inputs).unwrap();
            assert_eq!(b.bin_outputs(&[outputs]).unwrap(), None);
            b.finish()
        };
        let (enc, gc) = garble(&circuit).unwrap();
        for (x,y) in AES_SBOX.iter().enumerate() {
            let x_bits: Vec<u16> = (0..8).map(|i| (x as u16 >> i) & 0x1).collect();
            let y_bits: Vec<u16> = (0..8).map(|i| (*y as u16 >> i) & 0x1).collect();
            let inputs = enc.encode_garbler_inputs(&x_bits);
            let outputs = gc.eval(&circuit, &inputs, &[]).unwrap();
            assert_eq!(&outputs, &y_bits);
        }
    }

    #[test]
    fn test_present_sbox() {
        const PRESENT_SBOX: [u16; 16] = [0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2];
        let circuit = {
            let mut b = CircuitBuilder::new();
            let inputs = b.bin_garbler_input(4);
            let outputs = present_sbox(&mut b, &inputs).unwrap();

            assert_eq!(b.bin_outputs(&[outputs]).unwrap(), None);
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
                let inputs = b.bin_garbler_input(4);
                let outputs = cmul_mod_x4_x_1(&mut b, c, &inputs).unwrap();
                assert_eq!(b.bin_outputs(&[outputs]).unwrap(), None);
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

    #[test]
    fn test_cmul_mod_x8_x4_x3_x_1() {
        const CMUL_1: [u16; 256] = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf, 0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf, 0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff];
        const CMUL_2: [u16; 256] = [0x0, 0x2, 0x4, 0x6, 0x8, 0xa, 0xc, 0xe, 0x10, 0x12, 0x14, 0x16, 0x18, 0x1a, 0x1c, 0x1e, 0x20, 0x22, 0x24, 0x26, 0x28, 0x2a, 0x2c, 0x2e, 0x30, 0x32, 0x34, 0x36, 0x38, 0x3a, 0x3c, 0x3e, 0x40, 0x42, 0x44, 0x46, 0x48, 0x4a, 0x4c, 0x4e, 0x50, 0x52, 0x54, 0x56, 0x58, 0x5a, 0x5c, 0x5e, 0x60, 0x62, 0x64, 0x66, 0x68, 0x6a, 0x6c, 0x6e, 0x70, 0x72, 0x74, 0x76, 0x78, 0x7a, 0x7c, 0x7e, 0x80, 0x82, 0x84, 0x86, 0x88, 0x8a, 0x8c, 0x8e, 0x90, 0x92, 0x94, 0x96, 0x98, 0x9a, 0x9c, 0x9e, 0xa0, 0xa2, 0xa4, 0xa6, 0xa8, 0xaa, 0xac, 0xae, 0xb0, 0xb2, 0xb4, 0xb6, 0xb8, 0xba, 0xbc, 0xbe, 0xc0, 0xc2, 0xc4, 0xc6, 0xc8, 0xca, 0xcc, 0xce, 0xd0, 0xd2, 0xd4, 0xd6, 0xd8, 0xda, 0xdc, 0xde, 0xe0, 0xe2, 0xe4, 0xe6, 0xe8, 0xea, 0xec, 0xee, 0xf0, 0xf2, 0xf4, 0xf6, 0xf8, 0xfa, 0xfc, 0xfe, 0x1b, 0x19, 0x1f, 0x1d, 0x13, 0x11, 0x17, 0x15, 0xb, 0x9, 0xf, 0xd, 0x3, 0x1, 0x7, 0x5, 0x3b, 0x39, 0x3f, 0x3d, 0x33, 0x31, 0x37, 0x35, 0x2b, 0x29, 0x2f, 0x2d, 0x23, 0x21, 0x27, 0x25, 0x5b, 0x59, 0x5f, 0x5d, 0x53, 0x51, 0x57, 0x55, 0x4b, 0x49, 0x4f, 0x4d, 0x43, 0x41, 0x47, 0x45, 0x7b, 0x79, 0x7f, 0x7d, 0x73, 0x71, 0x77, 0x75, 0x6b, 0x69, 0x6f, 0x6d, 0x63, 0x61, 0x67, 0x65, 0x9b, 0x99, 0x9f, 0x9d, 0x93, 0x91, 0x97, 0x95, 0x8b, 0x89, 0x8f, 0x8d, 0x83, 0x81, 0x87, 0x85, 0xbb, 0xb9, 0xbf, 0xbd, 0xb3, 0xb1, 0xb7, 0xb5, 0xab, 0xa9, 0xaf, 0xad, 0xa3, 0xa1, 0xa7, 0xa5, 0xdb, 0xd9, 0xdf, 0xdd, 0xd3, 0xd1, 0xd7, 0xd5, 0xcb, 0xc9, 0xcf, 0xcd, 0xc3, 0xc1, 0xc7, 0xc5, 0xfb, 0xf9, 0xff, 0xfd, 0xf3, 0xf1, 0xf7, 0xf5, 0xeb, 0xe9, 0xef, 0xed, 0xe3, 0xe1, 0xe7, 0xe5];
        const CMUL_3: [u16; 256] = [0x0, 0x3, 0x6, 0x5, 0xc, 0xf, 0xa, 0x9, 0x18, 0x1b, 0x1e, 0x1d, 0x14, 0x17, 0x12, 0x11, 0x30, 0x33, 0x36, 0x35, 0x3c, 0x3f, 0x3a, 0x39, 0x28, 0x2b, 0x2e, 0x2d, 0x24, 0x27, 0x22, 0x21, 0x60, 0x63, 0x66, 0x65, 0x6c, 0x6f, 0x6a, 0x69, 0x78, 0x7b, 0x7e, 0x7d, 0x74, 0x77, 0x72, 0x71, 0x50, 0x53, 0x56, 0x55, 0x5c, 0x5f, 0x5a, 0x59, 0x48, 0x4b, 0x4e, 0x4d, 0x44, 0x47, 0x42, 0x41, 0xc0, 0xc3, 0xc6, 0xc5, 0xcc, 0xcf, 0xca, 0xc9, 0xd8, 0xdb, 0xde, 0xdd, 0xd4, 0xd7, 0xd2, 0xd1, 0xf0, 0xf3, 0xf6, 0xf5, 0xfc, 0xff, 0xfa, 0xf9, 0xe8, 0xeb, 0xee, 0xed, 0xe4, 0xe7, 0xe2, 0xe1, 0xa0, 0xa3, 0xa6, 0xa5, 0xac, 0xaf, 0xaa, 0xa9, 0xb8, 0xbb, 0xbe, 0xbd, 0xb4, 0xb7, 0xb2, 0xb1, 0x90, 0x93, 0x96, 0x95, 0x9c, 0x9f, 0x9a, 0x99, 0x88, 0x8b, 0x8e, 0x8d, 0x84, 0x87, 0x82, 0x81, 0x9b, 0x98, 0x9d, 0x9e, 0x97, 0x94, 0x91, 0x92, 0x83, 0x80, 0x85, 0x86, 0x8f, 0x8c, 0x89, 0x8a, 0xab, 0xa8, 0xad, 0xae, 0xa7, 0xa4, 0xa1, 0xa2, 0xb3, 0xb0, 0xb5, 0xb6, 0xbf, 0xbc, 0xb9, 0xba, 0xfb, 0xf8, 0xfd, 0xfe, 0xf7, 0xf4, 0xf1, 0xf2, 0xe3, 0xe0, 0xe5, 0xe6, 0xef, 0xec, 0xe9, 0xea, 0xcb, 0xc8, 0xcd, 0xce, 0xc7, 0xc4, 0xc1, 0xc2, 0xd3, 0xd0, 0xd5, 0xd6, 0xdf, 0xdc, 0xd9, 0xda, 0x5b, 0x58, 0x5d, 0x5e, 0x57, 0x54, 0x51, 0x52, 0x43, 0x40, 0x45, 0x46, 0x4f, 0x4c, 0x49, 0x4a, 0x6b, 0x68, 0x6d, 0x6e, 0x67, 0x64, 0x61, 0x62, 0x73, 0x70, 0x75, 0x76, 0x7f, 0x7c, 0x79, 0x7a, 0x3b, 0x38, 0x3d, 0x3e, 0x37, 0x34, 0x31, 0x32, 0x23, 0x20, 0x25, 0x26, 0x2f, 0x2c, 0x29, 0x2a, 0xb, 0x8, 0xd, 0xe, 0x7, 0x4, 0x1, 0x2, 0x13, 0x10, 0x15, 0x16, 0x1f, 0x1c, 0x19, 0x1a];
        const CMUL_4: [u16; 256] = [0x0, 0x4, 0x8, 0xc, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40, 0x44, 0x48, 0x4c, 0x50, 0x54, 0x58, 0x5c, 0x60, 0x64, 0x68, 0x6c, 0x70, 0x74, 0x78, 0x7c, 0x80, 0x84, 0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4, 0xa8, 0xac, 0xb0, 0xb4, 0xb8, 0xbc, 0xc0, 0xc4, 0xc8, 0xcc, 0xd0, 0xd4, 0xd8, 0xdc, 0xe0, 0xe4, 0xe8, 0xec, 0xf0, 0xf4, 0xf8, 0xfc, 0x1b, 0x1f, 0x13, 0x17, 0xb, 0xf, 0x3, 0x7, 0x3b, 0x3f, 0x33, 0x37, 0x2b, 0x2f, 0x23, 0x27, 0x5b, 0x5f, 0x53, 0x57, 0x4b, 0x4f, 0x43, 0x47, 0x7b, 0x7f, 0x73, 0x77, 0x6b, 0x6f, 0x63, 0x67, 0x9b, 0x9f, 0x93, 0x97, 0x8b, 0x8f, 0x83, 0x87, 0xbb, 0xbf, 0xb3, 0xb7, 0xab, 0xaf, 0xa3, 0xa7, 0xdb, 0xdf, 0xd3, 0xd7, 0xcb, 0xcf, 0xc3, 0xc7, 0xfb, 0xff, 0xf3, 0xf7, 0xeb, 0xef, 0xe3, 0xe7, 0x36, 0x32, 0x3e, 0x3a, 0x26, 0x22, 0x2e, 0x2a, 0x16, 0x12, 0x1e, 0x1a, 0x6, 0x2, 0xe, 0xa, 0x76, 0x72, 0x7e, 0x7a, 0x66, 0x62, 0x6e, 0x6a, 0x56, 0x52, 0x5e, 0x5a, 0x46, 0x42, 0x4e, 0x4a, 0xb6, 0xb2, 0xbe, 0xba, 0xa6, 0xa2, 0xae, 0xaa, 0x96, 0x92, 0x9e, 0x9a, 0x86, 0x82, 0x8e, 0x8a, 0xf6, 0xf2, 0xfe, 0xfa, 0xe6, 0xe2, 0xee, 0xea, 0xd6, 0xd2, 0xde, 0xda, 0xc6, 0xc2, 0xce, 0xca, 0x2d, 0x29, 0x25, 0x21, 0x3d, 0x39, 0x35, 0x31, 0xd, 0x9, 0x5, 0x1, 0x1d, 0x19, 0x15, 0x11, 0x6d, 0x69, 0x65, 0x61, 0x7d, 0x79, 0x75, 0x71, 0x4d, 0x49, 0x45, 0x41, 0x5d, 0x59, 0x55, 0x51, 0xad, 0xa9, 0xa5, 0xa1, 0xbd, 0xb9, 0xb5, 0xb1, 0x8d, 0x89, 0x85, 0x81, 0x9d, 0x99, 0x95, 0x91, 0xed, 0xe9, 0xe5, 0xe1, 0xfd, 0xf9, 0xf5, 0xf1, 0xcd, 0xc9, 0xc5, 0xc1, 0xdd, 0xd9, 0xd5, 0xd1];

        fn test_cmul(c: usize, expected: &[u16; 256]) {
            let circuit = {
                let mut b = CircuitBuilder::new();
                let inputs = b.bin_garbler_input(8);
                let outputs = cmul_mod_x8_x4_x3_x_1(&mut b, c, &inputs).unwrap();
                assert_eq!(b.bin_outputs(&[outputs]).unwrap(), None);
                b.finish()
            };
            let (enc, gc) = garble(&circuit).unwrap();
            for (x,y) in expected.iter().enumerate() {
                let x_bits: Vec<u16> = (0..8).map(|i| (x as u16 >> i) & 0x1).collect();
                let y_bits: Vec<u16> = (0..8).map(|i| (*y as u16 >> i) & 0x1).collect();
                let inputs = enc.encode_garbler_inputs(&x_bits);
                let outputs = gc.eval(&circuit, &inputs, &[]).unwrap();
                assert_eq!(&outputs, &y_bits);
            }
        }

        test_cmul(1, &CMUL_1);
        test_cmul(2, &CMUL_2);
        test_cmul(3, &CMUL_3);
        test_cmul(4, &CMUL_4);
    }

    // fn eval_circuit<C>(circuit_f)

    // #[test]
    // fn test_sbox_layer_proj() {
    //     let n = 25;
    //     let state = (0..n).map(|i| i % 16).collect_vec();
    //     let tt = [1,0,3,2,5,4,7,6,9,8,11,10,13,12,15,14];
    //     let b = CircuitBuilder
    // }

    #[test]
    fn test_mix_columns_mds() {
        let mut b = Dummy::new();
        let mut cells = Vec::with_capacity(16);
            for i in 0..4 {
                for j in 0..4 {
                    cells.push(
                        if i == j {
                            1
                        }else{
                            0
                        }
                    )
                }
            }
            println!("{:?}", cells);
            let mut matrix = cells.iter().map(|v| b.constant(*v, &Modulus::X4_X_1).unwrap()).collect::<Vec<_>>();
            mix_columns_mds(&mut b, &mut matrix, 4, &[4, 1, 2, 2], 4).unwrap();
            let outputs = b.outputs(&matrix).unwrap().unwrap();
            assert_eq!(outputs, vec![0x4, 0x8, 0xb, 0x2, 0x1, 0x6, 0xe, 0x2, 0x2, 0x5, 0xa, 0xf, 0x2, 0x6, 0x9, 0xb]);
    }
}