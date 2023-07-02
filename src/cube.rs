#![allow(dead_code)]

use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, Neg},
};

use strum_macros::{Display, EnumIter, EnumString, FromRepr};

// #[rustfmt::skip] // FromRepr break it
#[allow(clippy::upper_case_acronyms)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
pub enum CornerPosition {
    URF, UFL, ULB, UBR, DFR, DLF, DBL, DRB,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Corner(pub CornerPosition, pub PieceOrientation<3>);

impl Display for Corner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.0.to_string();
        let mut name = name.chars().collect::<Box<_>>();
        name.rotate_left(u8::from(self.1) as usize);
        let name = name.iter().collect::<String>();
        write!(f, "{}", name)
    }
}

impl TryFrom<(u8, u8)> for Corner {
    type Error = ();

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        let pos = CornerPosition::from_repr(value.0).ok_or(())?;
        let ori = PieceOrientation::from_repr(value.1).ok_or(())?;
        Ok(Corner(pos, ori))
    }
}

// #[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
pub enum EdgePosition {
    UR, UF, UL, UB, DR, DF, DL, DB, FR, FL, BL, BR,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Edge(pub EdgePosition, pub PieceOrientation<2>);

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.0.to_string();
        let mut name = name.chars().collect::<Box<_>>();
        name.rotate_left(u8::from(self.1) as usize);
        let name = name.iter().collect::<String>();
        write!(f, "{}", name)
    }
}

impl TryFrom<(u8, u8)> for Edge {
    type Error = ();

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        let pos = EdgePosition::from_repr(value.0).ok_or(())?;
        let ori = PieceOrientation::from_repr(value.1).ok_or(())?;
        Ok(Edge(pos, ori))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct PieceOrientation<const N: u8>(u8);

impl<const N: u8> PieceOrientation<N> {
    fn from_repr(repr: u8) -> Option<Self> {
        if (0..N).contains(&repr) {
            Some(PieceOrientation(repr))
        } else {
            None
        }
    }
}

impl<const N: u8> From<PieceOrientation<N>> for u8 {
    fn from(val: PieceOrientation<N>) -> Self {
        val.0
    }
}

impl<const N: u8> Add<PieceOrientation<N>> for PieceOrientation<N> {
    type Output = PieceOrientation<N>;

    fn add(self, rhs: PieceOrientation<N>) -> Self::Output {
        Self::from_repr((u8::from(self) + u8::from(rhs)) % N).unwrap()
    }
}

impl<const N: u8> Sum for PieceOrientation<N> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::from_repr(iter.map(u8::from).sum::<u8>() % N).unwrap()
    }
}

impl<'a, const N: u8> Sum<&'a PieceOrientation<N>> for PieceOrientation<N> {
    fn sum<I: Iterator<Item = &'a PieceOrientation<N>>>(iter: I) -> Self {
        Self::from_repr(iter.map(|c| u8::from(*c)).sum::<u8>() % N).unwrap()
    }
}

impl<const N: u8> Neg for PieceOrientation<N> {
    type Output = PieceOrientation<N>;

    fn neg(self) -> Self::Output {
        Self::from_repr((N - u8::from(self)) % N).unwrap()
    }
}

// cube orientation

#[rustfmt::skip]
#[allow(clippy::upper_case_acronyms)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, EnumString)]
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
        self as u8 >= 24
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CubeState {
    pub corners: [Corner; 8],
    pub edges: [Edge; 12],
}

impl Default for CubeState {
    fn default() -> Self {
        let corners: [Corner; 8] = (0..8)
            .into_iter()
            .map(|i| (i, 0).try_into().unwrap())
            .collect::<Vec<Corner>>()
            .try_into()
            .unwrap();
        let edges: [Edge; 12] = (0..12)
            .into_iter()
            .map(|i| (i, 0).try_into().unwrap())
            .collect::<Vec<Edge>>()
            .try_into()
            .unwrap();
        CubeState { corners, edges }
    }
}

// #[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, EnumIter, FromRepr)]
#[repr(u8)]
pub enum CubeMove {
    U, Up, R, Rp, F, Fp, D, Dp, L, Lp, B, Bp,
    // for gancubev2:
    // U: white, R: red, F: green, D: yellow, L: orange, B: blue
}

impl Display for CubeMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CubeMove::*;
        match self {
            U => write!(f, "U"),
            Up => write!(f, "U'"),
            R => write!(f, "R"),
            Rp => write!(f, "R'"),
            F => write!(f, "F"),
            Fp => write!(f, "F'"),
            D => write!(f, "D"),
            Dp => write!(f, "D'"),
            L => write!(f, "L"),
            Lp => write!(f, "L'"),
            B => write!(f, "B"),
            Bp => write!(f, "B'"),
        }
    }
}

impl CubeMove {
    pub fn is_clockwise(self) -> bool {
        self as u8 % 2 == 0
    }

    pub fn rev(self) -> Self {
        let repr = self as u8;
        let (ind, dir) = (repr / 2, repr % 2);
        let dir = (dir + 1) % 2;
        let repr = ind * 2 + dir;
        Self::from_repr(repr).unwrap()
    }

    pub fn abs(self) -> Self {
        Self::from_repr(self as u8 / 2 * 2).unwrap()
    }

    pub fn commute(self, other: Self) -> bool {
        self as u8 / 2 % 3 == other as u8 / 2 % 3
    }

    pub fn transform(self, trans: CubeOrientation) -> Self {
        let repr = self as u8;
        let (ind, dir) = (repr / 2, repr % 2);
        let trans_ind = trans
            .perm()
            .iter()
            .position(|&i| i == ind as usize)
            .unwrap() as u8;
        let trans_dir = if trans.is_mirror() {
            (dir + 1) % 2
        } else {
            dir
        };
        let trans_repr = trans_ind * 2 + trans_dir;
        Self::from_repr(trans_repr).unwrap()
    }
}

pub fn format_moves(moves: &[CubeMove]) -> String {
    fn group<T: Eq>(slice: &[T]) -> Vec<&[T]> {
        let mut res = Vec::new();
        if slice.is_empty() {
            return res;
        }
        let mut range = 0..0;
        let mut curr = &slice[0];
        for (index, value) in slice.iter().enumerate() {
            if value == curr {
                range.end += 1;
            } else {
                res.push(&slice[range]);
                range = index..index + 1;
                curr = value;
            }
        }
        res.push(&slice[range]);
        res
    }

    group(moves)
        .into_iter()
        .map(|mvs| {
            let len = mvs.len();
            if len == 1 {
                mvs[0].to_string()
            } else {
                mvs[0].to_string() + &len.to_string()
            }
        })
        .collect()
}
