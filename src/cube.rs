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

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
pub enum CornerOrientation {
    Normal,
    Clockwise,
    Counterclockwise,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Corner(pub CornerPosition, pub CornerOrientation);

impl Display for Corner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.0.to_string();
        let name = match self.1 {
            CornerOrientation::Normal => name,
            CornerOrientation::Clockwise => {
                let mut name = name.chars().collect::<Box<_>>();
                name.rotate_left(1);
                name.iter().collect()
            }
            CornerOrientation::Counterclockwise => {
                let mut name = name.chars().collect::<Box<_>>();
                name.rotate_left(2);
                name.iter().collect()
            }
        };
        write!(f, "{}", name)
    }
}

impl TryFrom<(u8, u8)> for Corner {
    type Error = ();

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        let pos = CornerPosition::from_repr(value.0).ok_or(())?;
        let ori = CornerOrientation::from_repr(value.1).ok_or(())?;
        Ok(Corner(pos, ori))
    }
}

// #[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
pub enum EdgePosition {
    UR, UF, UL, UB, DR, DF, DL, DB, FR, FL, BL, BR,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
pub enum EdgeOrientation {
    Normal,
    Flip,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Edge(pub EdgePosition, pub EdgeOrientation);

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.0.to_string();
        let name = match self.1 {
            EdgeOrientation::Normal => name,
            EdgeOrientation::Flip => {
                let mut name = name.chars().collect::<Box<_>>();
                name.rotate_left(1);
                name.iter().collect()
            }
        };
        write!(f, "{}", name)
    }
}

impl TryFrom<(u8, u8)> for Edge {
    type Error = ();

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        let pos = EdgePosition::from_repr(value.0).ok_or(())?;
        let ori = EdgeOrientation::from_repr(value.1).ok_or(())?;
        Ok(Edge(pos, ori))
    }
}

// algebra

impl Default for CornerOrientation {
    fn default() -> Self {
        CornerOrientation::Normal
    }
}

impl Add<CornerOrientation> for CornerOrientation {
    type Output = CornerOrientation;

    fn add(self, rhs: CornerOrientation) -> Self::Output {
        Self::from_repr((self as u8 + rhs as u8) % 3).unwrap()
    }
}

impl Sum for CornerOrientation {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::from_repr(iter.map(|c| c as u8).sum::<u8>() % 3).unwrap()
    }
}

impl<'a> Sum<&'a CornerOrientation> for CornerOrientation {
    fn sum<I: Iterator<Item = &'a CornerOrientation>>(iter: I) -> Self {
        Self::from_repr(iter.map(|c| *c as u8).sum::<u8>() % 3).unwrap()
    }
}

impl Neg for CornerOrientation {
    type Output = CornerOrientation;

    fn neg(self) -> Self::Output {
        Self::from_repr((3 - self as u8) % 3).unwrap()
    }
}

impl Default for EdgeOrientation {
    fn default() -> Self {
        EdgeOrientation::Normal
    }
}

impl Add<EdgeOrientation> for EdgeOrientation {
    type Output = EdgeOrientation;

    fn add(self, rhs: EdgeOrientation) -> Self::Output {
        Self::from_repr((self as u8 + rhs as u8) % 2).unwrap()
    }
}

impl Sum for EdgeOrientation {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::from_repr(iter.map(|c| c as u8).sum::<u8>() % 2).unwrap()
    }
}

impl<'a> Sum<&'a EdgeOrientation> for EdgeOrientation {
    fn sum<I: Iterator<Item = &'a EdgeOrientation>>(iter: I) -> Self {
        Self::from_repr(iter.map(|c| *c as u8).sum::<u8>() % 2).unwrap()
    }
}

impl Neg for EdgeOrientation {
    type Output = EdgeOrientation;

    fn neg(self) -> Self::Output {
        // Self::from_repr((2 - self as u8) % 2).unwrap()
        self
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

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CubeStateRaw {
    pub corners_position: [u8; 8],
    pub corners_orientation: [u8; 8],
    pub edges_position: [u8; 12],
    pub edges_orientation: [u8; 12],
}

impl Default for CubeStateRaw {
    fn default() -> Self {
        CubeStateRaw {
            corners_position: [0, 1, 2, 3, 4, 5, 6, 7],
            corners_orientation: [0; 8],
            edges_position: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            edges_orientation: [0; 12],
        }
    }
}

pub struct CubeState {
    pub corners: [Corner; 8],
    pub edges: [Edge; 12],
}

impl Default for CubeState {
    fn default() -> Self {
        Self::try_from(CubeStateRaw::default()).unwrap()
    }
}

impl TryFrom<CubeStateRaw> for CubeState {
    type Error = ();

    fn try_from(value: CubeStateRaw) -> Result<Self, Self::Error> {
        let corners: [Corner; 8] = value
            .corners_position
            .into_iter()
            .zip(value.corners_orientation)
            .map(|v| v.try_into())
            .collect::<Result<Vec<Corner>, ()>>()?
            .try_into()
            .unwrap();

        let edges: [Edge; 12] = value
            .edges_position
            .into_iter()
            .zip(value.edges_orientation)
            .map(|v| v.try_into())
            .collect::<Result<Vec<Edge>, ()>>()?
            .try_into()
            .unwrap();

        Ok(CubeState { corners, edges })
    }
}

impl From<CubeState> for CubeStateRaw {
    fn from(value: CubeState) -> Self {
        let corners_position: [u8; 8] = value
            .corners
            .into_iter()
            .map(|c| c.0 as u8)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let corners_orientation: [u8; 8] = value
            .corners
            .into_iter()
            .map(|c| c.1 as u8)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let edges_position: [u8; 12] = value
            .edges
            .into_iter()
            .map(|c| c.0 as u8)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let edges_orientation: [u8; 12] = value
            .edges
            .into_iter()
            .map(|c| c.1 as u8)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        CubeStateRaw {
            corners_position,
            corners_orientation,
            edges_position,
            edges_orientation,
        }
    }
}

// #[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, FromRepr)]
#[repr(u8)]
pub enum CubeMove {
    U, Up, R, Rp, F, Fp, D, Dp, L, Lp, B, Bp,
    // for gancubev2:
    // U: white, R: red, F: green, D: yellow, L: orange, B: blue
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
