use euclid::{default::Point2D, point2};
use std::hash::Hash;

use crate::Winding;

const INIT_COMMANDS_SIZE: usize = 256;

pub(super) enum Command {
    MoveTo(Point2D<f32>),
    LineTo(Point2D<f32>),
    BezierTo {
        pos: Point2D<f32>,
        h1_pos: Point2D<f32>,
        h2_pos: Point2D<f32>,
    },
    Close,
    Winding(Winding),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandType {
    MoveTo = 0,
    LineTo,
    BezierTo,
    ClosePath,
    Winding,
}

impl CommandType {
    fn from_u8(c: u8) -> Option<Self> {
        match c {
            0 => Some(Self::MoveTo),
            1 => Some(Self::LineTo),
            2 => Some(Self::BezierTo),
            3 => Some(Self::ClosePath),
            4 => Some(Self::Winding),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct PackedCommandBuffer {
    pub data: Vec<u8>,
}

impl PackedCommandBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(INIT_COMMANDS_SIZE),
        }
    }

    pub fn move_to(&mut self, pos: Point2D<f32>) {
        self.data.reserve(1 + 8);
        self.data.push(CommandType::MoveTo as u8);
        self.data.extend_from_slice(&pos.x.to_ne_bytes());
        self.data.extend_from_slice(&pos.y.to_ne_bytes());
    }

    pub fn line_to(&mut self, pos: Point2D<f32>) {
        self.data.reserve(1 + 8);
        self.data.push(CommandType::LineTo as u8);
        self.data.extend_from_slice(&pos.x.to_ne_bytes());
        self.data.extend_from_slice(&pos.y.to_ne_bytes());
    }

    pub fn bezier_to(&mut self, pos: Point2D<f32>, h1_pos: Point2D<f32>, h2_pos: Point2D<f32>) {
        self.data.reserve(1 + 8 + 8 + 8);
        self.data.push(CommandType::BezierTo as u8);
        self.data.extend_from_slice(&pos.x.to_ne_bytes());
        self.data.extend_from_slice(&pos.y.to_ne_bytes());
        self.data.extend_from_slice(&h1_pos.x.to_ne_bytes());
        self.data.extend_from_slice(&h1_pos.y.to_ne_bytes());
        self.data.extend_from_slice(&h2_pos.x.to_ne_bytes());
        self.data.extend_from_slice(&h2_pos.y.to_ne_bytes());
    }

    pub fn close_path(&mut self) {
        self.data.push(CommandType::ClosePath as u8);
    }

    pub fn winding(&mut self, winding: Winding) {
        self.data.push(CommandType::Winding as u8);
        self.data.push(winding as u8);
    }

    pub fn iter<'a>(&'a self) -> CommandIterator<'a> {
        CommandIterator {
            data: &self.data,
            curr: 0,
        }
    }
}

pub(super) struct CommandIterator<'a> {
    pub data: &'a [u8],
    pub curr: usize,
}

impl<'a> Iterator for CommandIterator<'a> {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(cmd_byte) = self.data.get(self.curr).copied() else {
            return None;
        };

        let Some(cmd_type) = CommandType::from_u8(cmd_byte) else {
            return None;
        };

        self.curr += 1;

        Some(match cmd_type {
            CommandType::MoveTo => {
                if self.curr + 8 > self.data.len() {
                    return None;
                }

                let b = &self.data[self.curr..self.curr + 8];

                let pos = point2(
                    f32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
                    f32::from_ne_bytes([b[4], b[5], b[6], b[7]]),
                );

                self.curr += 8;

                Command::MoveTo(pos)
            }
            CommandType::LineTo => {
                if self.curr + 8 > self.data.len() {
                    return None;
                }

                let b = &self.data[self.curr..self.curr + 8];

                let pos = point2(
                    f32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
                    f32::from_ne_bytes([b[4], b[5], b[6], b[7]]),
                );

                self.curr += 8;

                Command::LineTo(pos)
            }
            CommandType::BezierTo => {
                if self.curr + 8 + 8 + 8 > self.data.len() {
                    return None;
                }

                let b = &self.data[self.curr..self.curr + 8 + 8 + 8];

                let pos = point2(
                    f32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
                    f32::from_ne_bytes([b[4], b[5], b[6], b[7]]),
                );
                let h1_pos = point2(
                    f32::from_ne_bytes([b[8], b[9], b[10], b[11]]),
                    f32::from_ne_bytes([b[12], b[13], b[14], b[15]]),
                );
                let h2_pos = point2(
                    f32::from_ne_bytes([b[16], b[17], b[18], b[19]]),
                    f32::from_ne_bytes([b[20], b[21], b[22], b[23]]),
                );

                self.curr += 8 + 8 + 8;

                Command::BezierTo {
                    pos,
                    h1_pos,
                    h2_pos,
                }
            }
            CommandType::Winding => {
                if self.curr == self.data.len() {
                    return None;
                }

                let winding = Winding::from_u8(self.data[self.curr]);

                Command::Winding(winding)
            }
            CommandType::ClosePath => Command::Close,
        })
    }
}
