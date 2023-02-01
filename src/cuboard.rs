#![allow(dead_code)]

use std::ops::Range;

use crate::cube::CubeMove;

#[derive(Debug, thiserror::Error)]
pub enum CuboardInputError {
    #[error("unfinished input")]
    UnfinishedInput,
}

#[derive(Debug, Clone)]
pub struct CuboardKey {
    pub main: CubeMove,
    pub num: usize, // 0..4
    pub is_shifted: bool,
}

impl CuboardKey {
    const KEYS: [[CubeMove; 4]; 12] = {
        use CubeMove::*;
        [
            [L, B, R, F], // U
            [L, B, R, F], // Up
            [D, F, U, B], // R
            [D, F, U, B], // Rp
            [U, R, D, L], // F
            [U, R, D, L], // Fp
            [B, L, F, R], // D
            [B, L, F, R], // Dp
            [F, D, B, U], // L
            [F, D, B, U], // Lp
            [R, U, L, D], // B
            [R, U, L, D], // Bp
        ]
    };

    fn parse(value: &[CubeMove], mut start: usize) -> Vec<(Self, Range<usize>)> {
        let mut res = Vec::new();
        loop {
            let (adj, main, is_shifted) = match value[start..] {
                [a, a_, b, ..] if a == a_ && a != b => (a.abs(), b, true),
                [a, b, ..] if a != b => (a.abs(), b, false),
                _ => return res,
            };
            let order = &Self::KEYS[main as u8 as usize];
            let Some(num) = order.iter().position(|a| adj == *a) else {
                return res;
            };
            let end = if is_shifted { start + 3 } else { start + 2 };
            res.push((
                CuboardKey {
                    main,
                    num,
                    is_shifted,
                },
                start..end,
            ));
            start = end;
        }
    }
}

pub struct Cuboard {
    moves: Vec<CubeMove>,
    keys: Vec<(CuboardKey, Range<usize>)>,
}

impl Cuboard {
    pub fn new() -> Self {
        Cuboard {
            moves: Vec::new(),
            keys: Vec::new(),
        }
    }

    pub fn moves(&self) -> &[CubeMove] {
        &self.moves
    }

    pub fn keys(&self) -> &[(CuboardKey, Range<usize>)] {
        &self.keys
    }

    pub fn remains(&self) -> &[CubeMove] {
        let chunk_end = self.keys.last().map_or(0, |k| k.1.end);
        &self.moves[chunk_end..]
    }

    pub fn cancel(&mut self) {
        self.moves.clear();
        self.keys.clear();
    }

    pub fn is_completed(&self) -> bool {
        self.keys.last().map_or(0, |k| k.1.end) == self.moves.len()
    }

    pub fn finish(&mut self) -> Vec<CuboardKey> {
        let chunk_end = self.keys.last().map_or(0, |k| k.1.end);
        let res = self.keys.drain(..).map(|k| k.0).collect();
        self.moves.drain(..chunk_end);
        res
    }

    pub fn input(&mut self, mv: CubeMove) -> bool {
        let tail = self
            .moves
            .iter()
            .rev()
            .take_while(|a| a.commute(mv))
            .copied()
            .collect::<Vec<_>>();

        let indices = tail
            .iter()
            .enumerate()
            .filter_map(|(i, &a)| if a.abs() == mv.abs() { Some(i) } else { None })
            .collect::<Vec<_>>();

        let is_canonical =
            indices.is_empty() || (indices.first() == Some(&0) && tail.first() == Some(&mv));
        let mut is_changed = false;

        if is_canonical {
            self.moves.push(mv);
        } else {
            let len = self.moves.len();
            let start = len - 1 - *indices.last().unwrap();
            let end = len - *indices.first().unwrap();
            let mut subseq = self.moves.drain(start..end).collect::<Vec<_>>();
            if subseq[0] == mv {
                subseq.push(mv);
            } else {
                subseq.pop();
            }
            self.moves.append(&mut subseq);

            let broken = self
                .keys
                .iter()
                .rev()
                .take_while(|(_, c)| c.end > start)
                .count();
            if broken > 0 {
                self.keys.drain(self.keys.len() - broken..);
                is_changed = true;
            }
        }

        let chunk_end = self.keys.last().map_or(0, |k| k.1.end);
        let mut new_keys = CuboardKey::parse(&self.moves, chunk_end);
        if !new_keys.is_empty() {
            is_changed = true;
            self.keys.append(&mut new_keys);
        }

        is_changed
    }
}
