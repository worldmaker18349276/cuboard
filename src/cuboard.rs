#![allow(dead_code)]

use std::{f32::consts::PI, ops::Range};

use kiss3d::nalgebra::{Quaternion, UnitQuaternion, Vector3};

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

    pub fn flush(&mut self) -> Vec<CuboardKey> {
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

const BUFFER_SIZE: usize = 20;

pub struct CuboardInput {
    pub buffer: CuboardBuffer,
    pub keymap: CuboardKeymap,
    handler: CuboardInputMessageHandler,
}

pub struct CuboardInputMessageHandler {
    count: Option<u8>,
    recognizer: GyroGestureRecognizer<BUFFER_SIZE>,
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

#[derive(Clone)]
pub enum CuboardInputEvent {
    Uninit,
    Init,
    Cancel,
    Finish(String),
    Input { accept: String, skip: usize },
}

impl CuboardInput {
    pub fn new(keymap: CuboardKeymap) -> Self {
        CuboardInput {
            buffer: CuboardBuffer::new(),
            keymap,
            handler: CuboardInputMessageHandler {
                count: None,
                recognizer: GyroGestureRecognizer::new(),
            },
        }
    }

    pub fn buffered_text(&self) -> String {
        self.buffer
            .keys()
            .iter()
            .map(|k| self.keymap[k.0.is_shifted as usize][k.0.main as u8 as usize][k.0.num])
            .collect::<String>()
    }

    pub fn complete_part(&self) -> String {
        let moves = self.buffer.moves();
        let complete = &moves[..moves.len() - self.buffer.remains().len()];
        format_moves(complete)
    }

    pub fn remain_part(&self) -> String {
        format_moves(self.buffer.remains())
    }

    pub fn cancel(&mut self) {
        self.buffer.cancel();
    }

    pub fn finish(&mut self) -> String {
        let accepted_text = self.buffered_text();
        self.buffer.cancel();
        accepted_text
    }

    pub fn input(&mut self, mvs: &[CubeMove]) -> String {
        let mut res = String::new();
        for mv in mvs {
            self.buffer.input(*mv);
            if self.buffered_text().contains('\n') {
                res += &self.finish();
            }
        }
        res
    }

    pub fn handle_message(&mut self, msg: ResponseMessage) -> Option<CuboardInputEvent> {
        // ignore messages until the current count is known
        if self.handler.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.handler.count = Some(count);
                return Some(CuboardInputEvent::Init);
            } else {
                return Some(CuboardInputEvent::Uninit);
            }
        }

        if let ResponseMessage::Gyroscope {
            q1,
            q1p,
            q2: _,
            q2p: _,
        } = msg
        {
            let orientation =
                UnitQuaternion::new_normalize(Quaternion::new(q1.0, q1.1, q1.2, q1.3));
            let torque = Vector3::new(q1p.0, q1p.1, q1p.2);
            let gesture = self.handler.recognizer.put(orientation, torque);
            match gesture {
                Some(GyroGesture::TurningAround) => {
                    let accept = self.finish();
                    return Some(CuboardInputEvent::Finish(accept));
                }
                Some(GyroGesture::Shaking) => {
                    self.cancel();
                    return Some(CuboardInputEvent::Cancel);
                }
                _ => {}
            }
        }

        let ResponseMessage::Moves { count, moves, times: _ } = msg else {
            return None;
        };

        let prev_count = self.handler.count.unwrap();
        self.handler.count = Some(count);

        let diff = count.wrapping_sub(prev_count) as usize;
        let mut skip = 7usize.saturating_sub(diff);
        let mut accept_moves = vec![];
        for &mv in moves[..diff.clamp(0, 7)].iter().rev() {
            if let Some(mv) = mv {
                accept_moves.push(mv);
            } else {
                skip += 1;
            }
        }
        let accept = self.input(&accept_moves);
        Some(CuboardInputEvent::Input { accept, skip })
    }
}

struct GyroGestureRecognizer<const N: usize> {
    orientations: [UnitQuaternion<f32>; N],
    torques: [Vector3<f32>; N],
    index: usize,

    shaking_torque: f32,
    turning_tolerance: f32,
    debounce: usize,
}

#[derive(Clone, Copy, Debug)]
enum GyroGesture {
    TurningAround,
    Shaking,
}

impl<const N: usize> GyroGestureRecognizer<N> {
    fn new() -> Self {
        const SHAKING_TORQUE: f32 = 0.25f32;
        const TOLERANCE: f32 = 0.1;
        let orientation = UnitQuaternion::identity();
        let torque = Vector3::default();
        GyroGestureRecognizer {
            orientations: [orientation; N],
            torques: [torque; N],
            index: 0,
            shaking_torque: SHAKING_TORQUE,
            turning_tolerance: TOLERANCE,
            debounce: 0,
        }
    }

    fn put(
        &mut self,
        orientation: UnitQuaternion<f32>,
        torque: Vector3<f32>,
    ) -> Option<GyroGesture> {
        self.orientations[self.index] = orientation;
        self.torques[self.index] = torque;
        self.index = (self.index + 1) % N;

        if self.debounce > 0 {
            self.debounce -= 1;
            return None;
        }

        if self.is_turning_around() {
            self.debounce = N;
            return Some(GyroGesture::TurningAround);
        }

        if self.is_shaking() {
            self.debounce = N;
            return Some(GyroGesture::Shaking);
        }

        None
    }

    fn is_turning_around(&self) -> bool {
        fn half_angle(q: UnitQuaternion<f32>) -> f32 {
            (q.i.powi(2) + q.j.powi(2) + q.k.powi(2)).sqrt().atan2(q.w)
        }

        let first_ori = self.orientations[self.index];
        let last_ori = self.orientations[(self.index + N - 1) % N];
        let angle = half_angle(last_ori * first_ori.conjugate()) * 2.0;

        (angle / (2.0 * PI) - 1.0).abs() < self.turning_tolerance
    }

    fn is_shaking(&self) -> bool {
        let mean = self.torques.iter().sum::<Vector3<f32>>() / N as f32;
        let var = self
            .torques
            .iter()
            .map(|p| (p - mean).norm_squared())
            .sum::<f32>()
            / N as f32;
        var > self.shaking_torque.powi(2)
    }
}
