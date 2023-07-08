#![allow(dead_code)]

use std::{
    collections::HashMap,
    iter::Sum,
    ops::{Add, Neg},
};

use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

use crate::cube::CubeMove;

#[rustfmt::skip]
#[allow(clippy::upper_case_acronyms)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, EnumIter, EnumString)]
pub enum CubeOrientation {
    // <A><B><C><D><E><F>: move <A>/<B>/<C>/<D>/<E>/<F> face to up/right/front/down/left/back
    URFDLB, UFLDBR, ULBDRF, UBRDFL, DFRUBL, DLFURB, DBLUFR, DRBULF, // CornerPosition
    FURBDL, LUFRDB, BULFDR, RUBLDF, RDFLUB, FDLBUR, LDBRUF, BDRFUL, // CornerPosition.rotate_right()
    RFULBD, FLUBRD, LBURFD, BRUFLD, FRDBLU, LFDRBU, BLDFRU, RBDLFU, // CornerPosition.rotate_left()
    FRUBLD, LFURBD, BLUFRD, RBULFD, RFDLBU, FLDBRU, LBDRFU, BRDFLU, // CornerPosition.rev()
    UFRDBL, ULFDRB, UBLDFR, URBDLF, DRFULB, DFLUBR, DLBURF, DBRUFL, // CornerPosition.rev().rotate_right()
    RUFLDB, FULBDR, LUBRDF, BURFDL, FDRBUL, LDFRUB, BDLFUR, RDBLUF, // CornerPosition.rev().rotate_left()
}

impl CubeOrientation {
    // no rotation
    const I: CubeOrientation = CubeOrientation::URFDLB;

    // rotate along face
    const U: CubeOrientation = CubeOrientation::UBRDFL;
    const D: CubeOrientation = CubeOrientation::UFLDBR;
    const R: CubeOrientation = CubeOrientation::FRDBLU;
    const L: CubeOrientation = CubeOrientation::BRUFLD;
    const F: CubeOrientation = CubeOrientation::LUFRDB;
    const B: CubeOrientation = CubeOrientation::RDFLUB;

    // swap face
    const UD: CubeOrientation = CubeOrientation::DRFULB;
    const RL: CubeOrientation = CubeOrientation::ULFDRB;
    const FB: CubeOrientation = CubeOrientation::URBDLF;
}

impl CubeOrientation {
    pub fn repr(self) -> u8 {
        self as u8
    }

    fn chars(self) -> [char; 6] {
        self.to_string()
            .chars()
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    fn perm(self) -> [usize; 6] {
        let chars = Self::default().chars();
        self.chars()
            .map(|c| chars.iter().position(|&a| a == c).unwrap())
    }

    fn try_from_perm(perm: [usize; 6]) -> Option<Self> {
        use std::str::FromStr;
        let chars = Self::default().chars();
        let name = perm.iter().map(|&i| chars[i]).collect::<String>();
        Self::from_str(&name).ok()
    }

    pub fn is_mirror(self) -> bool {
        self.repr() >= 24
    }

    pub fn as_map(&self) -> HashMap<CubeMove, CubeMove> {
        CubeMove::iter()
            .map(|mv| {
                let repr = mv.repr();
                let (ind, dir) = (repr / 2, repr % 2);
                let ind_ = self.perm().iter().position(|&i| i == ind as usize).unwrap() as u8;
                let dir_ = if self.is_mirror() { (dir + 1) % 2 } else { dir };
                let repr_ = ind_ * 2 + dir_;
                (mv, CubeMove::from_repr(repr_).unwrap())
            })
            .collect::<HashMap<_, _>>()
    }
}

impl Default for CubeOrientation {
    fn default() -> Self {
        CubeOrientation::URFDLB
    }
}

impl Add<CubeOrientation> for CubeOrientation {
    type Output = CubeOrientation;

    fn add(self, rhs: CubeOrientation) -> Self::Output {
        let perm = self.perm();
        let added_perm = rhs.perm().map(|i| perm[i]);
        Self::try_from_perm(added_perm).unwrap()
    }
}

impl Sum for CubeOrientation {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let sum_perm = iter
            .map(|r| r.perm())
            .fold(Self::default().perm(), |lhs, rhs| rhs.map(|i| lhs[i]));
        Self::try_from_perm(sum_perm).unwrap()
    }
}

impl<'a> Sum<&'a CubeOrientation> for CubeOrientation {
    fn sum<I: Iterator<Item = &'a CubeOrientation>>(iter: I) -> Self {
        let sum_perm = iter
            .map(|r| r.perm())
            .fold(Self::default().perm(), |lhs, rhs| rhs.map(|i| lhs[i]));
        Self::try_from_perm(sum_perm).unwrap()
    }
}

impl Neg for CubeOrientation {
    type Output = CubeOrientation;

    fn neg(self) -> Self::Output {
        let perm = self.perm();
        let neg_perm = Self::default()
            .perm()
            .map(|i| perm.iter().position(|&j| j == i).unwrap());
        Self::try_from_perm(neg_perm).unwrap()
    }
}

pub fn span<A>(gen: &[A]) -> Vec<(A, Vec<A>)>
where
    A: Add<A, Output = A> + Copy + Eq,
{
    let mut leaves = gen.to_vec();
    let mut res = leaves.iter().map(|&a| (a, vec![a])).collect::<Vec<_>>();

    while let Some(leaf) = leaves.pop() {
        let path = res.iter().find(|(a, _)| a == &leaf).cloned().unwrap().1;
        for &a in gen.iter() {
            let next = leaf + a;
            if res.iter().any(|(a, _)| a == &next) {
                continue;
            };
            let mut next_path = path.clone();
            next_path.push(a);
            res.push((next, next_path));
            leaves.push(next);
        }
    }
    res
}
