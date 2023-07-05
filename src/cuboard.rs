#![allow(dead_code)]

use std::ops::Range;

use crate::{
    bluetooth::gancubev2::ResponseMessage,
    cube::{format_moves, CubeMove},
};

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
            let (main, adj, is_shifted) = match value[start..] {
                [a, a_, b, ..] if a == a_ && a != b => (a, b.abs(), true),
                [a, b, ..] if a != b => (a, b.abs(), false),
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

pub struct CuboardBuffer {
    moves: Vec<CubeMove>,
    keys: Vec<(CuboardKey, Range<usize>)>,
}

impl CuboardBuffer {
    pub fn new() -> Self {
        CuboardBuffer {
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
        trait FirstAndLast: Iterator {
            fn first_and_last(self) -> Option<(Self::Item, Self::Item)>;
        }

        impl<E: Copy, I: Iterator<Item = E>> FirstAndLast for I {
            fn first_and_last(mut self) -> Option<(E, E)> {
                match self.next() {
                    None => None,
                    Some(first) => match self.last() {
                        None => Some((first, first)),
                        Some(last) => Some((first, last)),
                    },
                }
            }
        }

        let collapsed_range = self
            .moves
            .iter()
            .enumerate()
            .rev()
            .take_while(|(_, a)| a.commute(mv))
            .skip_while(|(_, a)| a.abs() != mv.abs())
            .take_while(|(_, a)| a.abs() == mv.abs())
            .map(|(i, _)| i)
            .first_and_last()
            .map_or(self.moves.len()..self.moves.len(), |(j, i)| i..j + 1);

        let broken_keys_count = self
            .keys
            .iter()
            .rev()
            .take_while(|(_, c)| c.end > collapsed_range.start)
            .count();

        let mut subseq = self.moves.drain(collapsed_range).collect::<Vec<_>>();
        if subseq.is_empty() || subseq.last().unwrap() == &mv {
            subseq.push(mv);
        } else {
            subseq.pop();
        }
        self.moves.extend(subseq);

        let mut key_changed = false;

        if broken_keys_count > 0 {
            self.keys.drain(self.keys.len() - broken_keys_count..);
            key_changed = true;
        }

        let chunk_end = self.keys.last().map_or(0, |k| k.1.end);
        let new_keys = CuboardKey::parse(&self.moves, chunk_end);
        if !new_keys.is_empty() {
            self.keys.extend(new_keys);
            key_changed = true;
        }

        key_changed
    }
}

pub struct CuboardInput {
    pub buffer: CuboardBuffer,
    pub keymap: CuboardKeymap,
    count: Option<u8>,
}

pub type CuboardKeymap = [[[&'static str; 4]; 12]; 2];

pub const DEFAULT_KEYMAP: CuboardKeymap = [
    [
        ["d", "u", "c", "k"], // U
        ["(", "[", "{", "<"], // U'
        ["g", "a", "s", "p"], // R
        ["0", " ", "z", "q"], // R'
        ["f", "l", "o", "w"], // F
        ["'", ".", ":", "!"], // F'
        ["j", "i", "n", "x"], // D
        ["+", "-", "*", "/"], // D'
        ["m", "y", "t", "h"], // L
        ["1", "2", "3", "4"], // L'
        ["v", "e", "r", "b"], // B
        ["#", "~", "&", "_"], // B'
    ],
    [
        ["D", "U", "C", "K"],  // U
        [")", "]", "}", ">"],  // U'
        ["G", "A", "S", "P"],  // R
        ["9", "\n", "Z", "Q"], // R'
        ["F", "L", "O", "W"],  // F
        ["\"", ",", ";", "?"], // F'
        ["J", "I", "N", "X"],  // D
        ["=", "|", "^", "\\"], // D'
        ["M", "Y", "T", "H"],  // L
        ["5", "6", "7", "8"],  // L'
        ["V", "E", "R", "B"],  // B
        ["@", "$", "%", "`"],  // B'
    ],
];

#[derive(Clone, Copy)]
pub enum CuboardInputState {
    Uninit,
    Init,
    None,
    Input { accept: usize, skip: usize },
}

impl CuboardInput {
    pub fn new(keymap: CuboardKeymap) -> Self {
        CuboardInput {
            buffer: CuboardBuffer::new(),
            keymap,
            count: None,
        }
    }

    pub fn text(&self) -> String {
        self.buffer
            .keys()
            .iter()
            .map(|k| self.keymap[k.0.is_shifted as usize][k.0.main as u8 as usize][k.0.num])
            .collect()
    }

    pub fn complete_part(&self) -> String {
        let moves = self.buffer.moves();
        let complete = &moves[..moves.len() - self.buffer.remains().len()];
        format_moves(complete)
    }

    pub fn remain_part(&self) -> String {
        format_moves(self.buffer.remains())
    }

    pub fn handle_message(&mut self, msg: ResponseMessage) -> CuboardInputState {
        // ignore messages until the current count is known
        if self.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.count = Some(count);
                return CuboardInputState::Init;
            } else {
                return CuboardInputState::Uninit;
            }
        }

        let ResponseMessage::Moves { count, moves, times: _ } = msg else {
            return CuboardInputState::None;
        };

        let prev_count = self.count.unwrap();
        self.count = Some(count);

        let diff = count.wrapping_sub(prev_count) as usize;
        for &mv in moves[..diff.clamp(0, 7)].iter().rev() {
            if let Some(mv) = mv {
                self.buffer.input(mv);
            }
        }

        CuboardInputState::Input { accept: diff.clamp(0, 7), skip: 7usize.saturating_sub(diff) }
    }
}
