#![allow(dead_code)]

use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, Neg},
};

use strum_macros::{Display, EnumIter, FromRepr};

#[rustfmt::skip]
#[allow(clippy::upper_case_acronyms)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
pub enum CornerPosition {
    URF, UFL, ULB, UBR, DFR, DLF, DBL, DRB,
}

impl CornerPosition {
    pub fn repr(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Corner(pub CornerPosition, pub PieceOrientation<3>);

impl Display for Corner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.0.to_string();
        let mut name = name.chars().collect::<Box<_>>();
        name.rotate_left(self.1.repr() as usize);
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

#[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
pub enum EdgePosition {
    UR, UF, UL, UB, DR, DF, DL, DB, FR, FL, BL, BR,
}

impl EdgePosition {
    pub fn repr(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Edge(pub EdgePosition, pub PieceOrientation<2>);

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.0.to_string();
        let mut name = name.chars().collect::<Box<_>>();
        name.rotate_left(self.1.repr() as usize);
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Center(pub PieceOrientation<4>);

impl Display for Center {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.repr())
    }
}

impl TryFrom<u8> for Center {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let ori = PieceOrientation::from_repr(value).ok_or(())?;
        Ok(Center(ori))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct PieceOrientation<const N: u8>(u8);

impl<const N: u8> PieceOrientation<N> {
    pub fn from_repr(repr: u8) -> Option<Self> {
        if (0..N).contains(&repr) {
            Some(PieceOrientation(repr))
        } else {
            None
        }
    }

    pub fn repr(self) -> u8 {
        self.0
    }
}

impl<const N: u8> Add<PieceOrientation<N>> for PieceOrientation<N> {
    type Output = PieceOrientation<N>;

    fn add(self, rhs: PieceOrientation<N>) -> Self::Output {
        Self::from_repr((self.repr() + rhs.repr()) % N).unwrap()
    }
}

impl<const N: u8> Sum for PieceOrientation<N> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::from_repr(iter.map(|c| c.repr()).sum::<u8>() % N).unwrap()
    }
}

impl<'a, const N: u8> Sum<&'a PieceOrientation<N>> for PieceOrientation<N> {
    fn sum<I: Iterator<Item = &'a PieceOrientation<N>>>(iter: I) -> Self {
        Self::from_repr(iter.map(|c| c.repr()).sum::<u8>() % N).unwrap()
    }
}

impl<const N: u8> Neg for PieceOrientation<N> {
    type Output = PieceOrientation<N>;

    fn neg(self) -> Self::Output {
        Self::from_repr((N - self.repr()) % N).unwrap()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CubeState {
    pub corners: [Corner; 8],
    pub edges: [Edge; 12],
    pub centers: [Center; 6],
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
        CubeState::new(corners, edges)
    }
}

impl CubeState {
    pub fn new(corners: [Corner; 8], edges: [Edge; 12]) -> Self {
        CubeState {
            corners,
            edges,
            centers: [0.try_into().unwrap(); 6],
        }
    }

    pub fn reset_centers(&mut self) {
        self.centers = [0.try_into().unwrap(); 6];
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, EnumIter, FromRepr)]
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
    pub fn repr(self) -> u8 {
        self as u8
    }

    pub fn is_clockwise(self) -> bool {
        self.repr() % 2 == 0
    }

    pub fn rev(self) -> Self {
        let repr = self.repr();
        let (ind, dir) = (repr / 2, repr % 2);
        let dir = (dir + 1) % 2;
        let repr = ind * 2 + dir;
        Self::from_repr(repr).unwrap()
    }

    pub fn abs(self) -> Self {
        Self::from_repr(self.repr() / 2 * 2).unwrap()
    }

    pub fn commute(self, other: Self) -> bool {
        self.repr() / 2 % 3 == other.repr() / 2 % 3
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
