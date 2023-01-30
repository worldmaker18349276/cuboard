use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, Neg},
};

#[rustfmt::skip]
#[allow(clippy::upper_case_acronyms)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CornerPosition {
    UFR, ULF, UBL, URB, DRF, DFL, DLB, DBR
}

impl CornerPosition {
    pub const LIST: [CornerPosition; 8] = [
        CornerPosition::UFR,
        CornerPosition::ULF,
        CornerPosition::UBL,
        CornerPosition::URB,
        CornerPosition::DRF,
        CornerPosition::DFL,
        CornerPosition::DLB,
        CornerPosition::DBR,
    ];

    const NAMES: [&str; 8] = ["UFR", "ULF", "UBL", "URB", "DRF", "DFL", "DLB", "DBR"];
}

impl TryFrom<u8> for CornerPosition {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match CornerPosition::LIST.get(value as usize) {
            Some(res) => Ok(*res),
            None => Err(()),
        }
    }
}

#[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CornerOrientation {
    Normal, Clockwise, Counterclockwise
}

impl CornerOrientation {
    pub const LIST: [CornerOrientation; 3] = [
        CornerOrientation::Normal,
        CornerOrientation::Clockwise,
        CornerOrientation::Counterclockwise,
    ];
}

impl TryFrom<u8> for CornerOrientation {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match CornerOrientation::LIST.get(value as usize) {
            Some(res) => Ok(*res),
            None => Err(()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Corner(pub CornerPosition, pub CornerOrientation);

impl Corner {
    pub fn show(self) -> String {
        let name = CornerPosition::NAMES[self.0 as u8 as usize];
        match self.1 {
            CornerOrientation::Normal => name.to_string(),
            CornerOrientation::Clockwise => {
                let mut name = name.as_bytes().to_vec();
                name.rotate_left(1);
                String::from_utf8(name).unwrap()
            }
            CornerOrientation::Counterclockwise => {
                let mut name = name.as_bytes().to_vec();
                name.rotate_left(2);
                String::from_utf8(name).unwrap()
            }
        }
    }
}

impl TryFrom<(u8, u8)> for Corner {
    type Error = ();

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        let pos = CornerPosition::try_from(value.0)?;
        let ori = CornerOrientation::try_from(value.1)?;
        Ok(Corner(pos, ori))
    }
}

#[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum EdgePosition {
    UR, UF, UL, UB, DR, DF, DL, DB, FR, FL, BL, BR
}

impl EdgePosition {
    pub const LIST: [EdgePosition; 12] = [
        EdgePosition::UR,
        EdgePosition::UF,
        EdgePosition::UL,
        EdgePosition::UB,
        EdgePosition::DR,
        EdgePosition::DF,
        EdgePosition::DL,
        EdgePosition::DB,
        EdgePosition::FR,
        EdgePosition::FL,
        EdgePosition::BL,
        EdgePosition::BR,
    ];

    const NAMES: [&str; 12] = [
        "UR", "UF", "UL", "UB", "DR", "DF", "DL", "DB", "FR", "FL", "BL", "BR",
    ];
}

impl TryFrom<u8> for EdgePosition {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match EdgePosition::LIST.get(value as usize) {
            Some(res) => Ok(*res),
            None => Err(()),
        }
    }
}

#[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum EdgeOrientation {
    Normal, Flip
}

impl EdgeOrientation {
    pub const LIST: [EdgeOrientation; 2] = [EdgeOrientation::Normal, EdgeOrientation::Flip];
}

impl TryFrom<u8> for EdgeOrientation {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match EdgeOrientation::LIST.get(value as usize) {
            Some(res) => Ok(*res),
            None => Err(()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Edge(pub EdgePosition, pub EdgeOrientation);

impl Edge {
    pub fn show(self) -> String {
        let name = EdgePosition::NAMES[self.0 as u8 as usize];
        match self.1 {
            EdgeOrientation::Normal => name.to_string(),
            EdgeOrientation::Flip => {
                let mut name = name.as_bytes().to_vec();
                name.rotate_left(1);
                String::from_utf8(name).unwrap()
            }
        }
    }
}

impl TryFrom<(u8, u8)> for Edge {
    type Error = ();

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        let pos = EdgePosition::try_from(value.0)?;
        let ori = EdgeOrientation::try_from(value.1)?;
        Ok(Edge(pos, ori))
    }
}

impl Default for CornerOrientation {
    fn default() -> Self {
        CornerOrientation::Normal
    }
}

impl Add<CornerOrientation> for CornerOrientation {
    type Output = CornerOrientation;

    fn add(self, rhs: CornerOrientation) -> Self::Output {
        Self::try_from((self as u8 + rhs as u8) % 3).unwrap()
    }
}

impl Sum for CornerOrientation {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::try_from(iter.map(|c| c as u8).sum::<u8>() % 3).unwrap()
    }
}

impl<'a> Sum<&'a CornerOrientation> for CornerOrientation {
    fn sum<I: Iterator<Item = &'a CornerOrientation>>(iter: I) -> Self {
        Self::try_from(iter.map(|c| *c as u8).sum::<u8>() % 3).unwrap()
    }
}

impl Neg for CornerOrientation {
    type Output = CornerOrientation;

    fn neg(self) -> Self::Output {
        Self::try_from((3 - self as u8) % 3).unwrap()
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
        Self::try_from((self as u8 + rhs as u8) % 2).unwrap()
    }
}

impl Sum for EdgeOrientation {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::try_from(iter.map(|c| c as u8).sum::<u8>() % 2).unwrap()
    }
}

impl<'a> Sum<&'a EdgeOrientation> for EdgeOrientation {
    fn sum<I: Iterator<Item = &'a EdgeOrientation>>(iter: I) -> Self {
        Self::try_from(iter.map(|c| *c as u8).sum::<u8>() % 2).unwrap()
    }
}

impl Neg for EdgeOrientation {
    type Output = EdgeOrientation;

    fn neg(self) -> Self::Output {
        Self::try_from((2 - self as u8) % 2).unwrap()
    }
}

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
            .zip(value.corners_orientation.into_iter())
            .map(|v| v.try_into())
            .collect::<Result<Vec<Corner>, ()>>()?
            .try_into()
            .unwrap();

        let edges: [Edge; 12] = value
            .edges_position
            .into_iter()
            .zip(value.edges_orientation.into_iter())
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

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum CubeMove {
    U, Up, R, Rp, F, Fp, D, Dp, L, Lp, B, Bp
    // U: white, R: red, F: green, D: yellow, L: orange, B: blue
}

impl Display for CubeMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CubeMove::U => "U ",
            CubeMove::Up => "U'",
            CubeMove::R => "R ",
            CubeMove::Rp => "R'",
            CubeMove::F => "F ",
            CubeMove::Fp => "F'",
            CubeMove::D => "D ",
            CubeMove::Dp => "D'",
            CubeMove::L => "L ",
            CubeMove::Lp => "L'",
            CubeMove::B => "B ",
            CubeMove::Bp => "B'",
        };
        write!(f, "{}", s)
    }
}

impl TryFrom<u8> for CubeMove {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CubeMove::U),
            1 => Ok(CubeMove::Up),
            2 => Ok(CubeMove::R),
            3 => Ok(CubeMove::Rp),
            4 => Ok(CubeMove::F),
            5 => Ok(CubeMove::Fp),
            6 => Ok(CubeMove::D),
            7 => Ok(CubeMove::Dp),
            8 => Ok(CubeMove::L),
            9 => Ok(CubeMove::Lp),
            10 => Ok(CubeMove::B),
            11 => Ok(CubeMove::Bp),
            _ => Err(()),
        }
    }
}

impl CubeMove {
    pub fn rev(self) -> Self {
        match self {
            CubeMove::U => CubeMove::Up,
            CubeMove::Up => CubeMove::U,
            CubeMove::R => CubeMove::Rp,
            CubeMove::Rp => CubeMove::R,
            CubeMove::F => CubeMove::Fp,
            CubeMove::Fp => CubeMove::F,
            CubeMove::D => CubeMove::Dp,
            CubeMove::Dp => CubeMove::D,
            CubeMove::L => CubeMove::Lp,
            CubeMove::Lp => CubeMove::L,
            CubeMove::B => CubeMove::Bp,
            CubeMove::Bp => CubeMove::B,
        }
    }

    pub fn abs(self) -> Self {
        match self {
            CubeMove::U | CubeMove::Up => CubeMove::U,
            CubeMove::R | CubeMove::Rp => CubeMove::R,
            CubeMove::F | CubeMove::Fp => CubeMove::F,
            CubeMove::D | CubeMove::Dp => CubeMove::D,
            CubeMove::L | CubeMove::Lp => CubeMove::L,
            CubeMove::B | CubeMove::Bp => CubeMove::B,
        }
    }

    pub fn mirror(self) -> Self {
        match self {
            CubeMove::U => CubeMove::Dp,
            CubeMove::Up => CubeMove::D,
            CubeMove::R => CubeMove::Lp,
            CubeMove::Rp => CubeMove::L,
            CubeMove::F => CubeMove::Bp,
            CubeMove::Fp => CubeMove::B,
            CubeMove::D => CubeMove::Up,
            CubeMove::Dp => CubeMove::U,
            CubeMove::L => CubeMove::Rp,
            CubeMove::Lp => CubeMove::R,
            CubeMove::B => CubeMove::Fp,
            CubeMove::Bp => CubeMove::F,
        }
    }

    pub fn commute(self, other: Self) -> bool {
        matches!(
            (self, other),
            (
                CubeMove::U | CubeMove::Up | CubeMove::D | CubeMove::Dp,
                CubeMove::U | CubeMove::Up | CubeMove::D | CubeMove::Dp
            ) | (
                CubeMove::R | CubeMove::Rp | CubeMove::L | CubeMove::Lp,
                CubeMove::R | CubeMove::Rp | CubeMove::L | CubeMove::Lp
            ) | (
                CubeMove::F | CubeMove::Fp | CubeMove::B | CubeMove::Bp,
                CubeMove::F | CubeMove::Fp | CubeMove::B | CubeMove::Bp
            )
        )
    }
}
