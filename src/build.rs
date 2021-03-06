#[macro_use]
pub mod bitboard;
pub mod magics;
pub mod types;
use bitboard::*;
use magics::*;
use types::*;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// A few comments on the different scenarios this file handles (and does not) with the corresponding compile commands
// Native compile for own machine : set RUSTFLAGS=-C target-cpu=native; cargo run --release (works)
// Host compile for target machine: set RUSTFLAGS=-C target-cpu=<your-target-machine-cpu>; cargo rustc --release --bin scam --target <your_target>
// In the case that the Host does not have BMI2, while the target-cpu wants BMI2 instructions, this build script will fail.
// Due to https://github.com/rust-lang/cargo/issues/4423 (build.rs can't be given any build flags), we sadly can not detect
// whether the host has the bmi2 instruction set or not. For now, we will just assume it has.
// For cross-compiling purposes, we extract the target-feature from the env var CARGO_CFG_TARGET_FEATURE instead of
// using #[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))] since that is the config for the host, and has nothing to do
// with the target in case of cross compilation. Additionally, if --target is supplied during compilation,
// #[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))] will always evaluate to false due to above issue.
pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let magic_path = Path::new(&out_dir).join("codegen_attacks.rs");
    let mut file = File::create(magic_path).unwrap();

    let has_bmi2 = env::var("CARGO_CFG_TARGET_FEATURE").map_or(false, |x| x.contains("bmi2"));
    if has_bmi2 {
        writeln!(file, "//Tables for BMI2").unwrap();
    } else {
        writeln!(file, "//Tables for Magics").unwrap();
    }
    let attacks = init_attacks(has_bmi2);
    write!(
        file,
        "#[rustfmt::skip]\n pub static ATTACKS: [u64; 107648] = {};\n",
        print_arr1d(&attacks, false)
    )
    .unwrap();

    let between_bb = init_between_bb();
    write!(
        file,
        "#[rustfmt::skip]\n pub const BETWEEN_BB: [[BitBoard; 64]; 64] = {};\n",
        print_arr2d(&between_bb, true)
    )
    .unwrap();
}

pub fn print_arr2d(arr: &[Vec<BitBoard>], bb: bool) -> String {
    let mut res_str = String::new();
    res_str.push('[');
    for arr2 in arr.iter() {
        res_str.push_str(&format!("{},", print_arr1d(arr2, bb)));
    }
    res_str.push(']');
    res_str
}

pub fn print_arr1d(arr: &[BitBoard], bb: bool) -> String {
    let mut res_str = String::new();
    res_str.push('[');
    for &attack in arr.iter() {
        if bb {
            res_str.push_str(&format!("BitBoard({}),", attack.0))
        } else {
            res_str.push_str(&format!("{},", attack.0));
        };
    }
    res_str.push(']');
    res_str
}

pub fn slider_attacks(sq: Square, attack_dirs: &[Direction; 4], occ: BitBoard) -> BitBoard {
    let mut res = BB_ZERO;
    for &dir in attack_dirs.iter() {
        let mut temp = bb!(sq);
        for _ in 0..6 {
            temp |= temp.shift(dir) & !occ;
        }
        res |= temp.shift(dir);
    }
    res
}

impl Magic {
    pub fn build_index(self, occ: BitBoard, has_bmi2: bool) -> usize {
        if has_bmi2 {
            use std::arch::x86_64::_pext_u64;
            self.offset + unsafe { _pext_u64(occ.0, self.mask.0) } as usize
        } else {
            self.offset + (((occ & self.mask).0).wrapping_mul(self.magic) >> self.shift) as usize
        }
    }
}

pub fn init_attacks(has_bmi2: bool) -> Vec<BitBoard> {
    let mut res = vec![BitBoard(0); 107648];
    for (magics, dirs) in [(BISHOP_MAGICS, BISHOP_DIRS), (ROOK_MAGICS, ROOK_DIRS)].iter() {
        for (sq, magic) in magics.iter().enumerate() {
            let mut occ = BB_ZERO;
            loop {
                let attacks = slider_attacks(sq as Square, dirs, occ);
                res[magic.build_index(occ, has_bmi2)] = attacks;
                occ = BitBoard((occ.0.wrapping_sub(magic.mask.0)) & magic.mask.0);
                if occ.is_empty() {
                    break;
                }
            }
        }
    }
    res
}

pub fn init_between_bb() -> Vec<Vec<BitBoard>> {
    let mut res = vec![vec![BB_ZERO; 64]; 64];
    for (sq, res_outer) in res.iter_mut().enumerate() {
        for (sq2, res_inner) in res_outer.iter_mut().enumerate() {
            for pt in [BISHOP_DIRS, ROOK_DIRS].iter() {
                if (slider_attacks(sq as Square, pt, BB_ZERO) & bb!(sq2)).not_empty() {
                    *res_inner |= slider_attacks(sq as Square, pt, bb!(sq2))
                        & slider_attacks(sq2 as Square, pt, bb!(sq));
                }
            }
        }
    }
    res
}
