use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

#[cfg(not(test))]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Color(char);

#[cfg(test)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Color(pub char);

#[derive(Debug, Error)]
#[error("not a valid character to represent a block color")]
pub struct InvalidColorChar;

impl Color {
    pub fn as_char(&self) -> char {
        self.0
    }
}

impl TryFrom<char> for Color {
    type Error = InvalidColorChar;

    fn try_from(c: char) -> Result<Self, InvalidColorChar> {
        if c.is_alphabetic() {
            Ok(Self(c))
        } else {
            Err(InvalidColorChar)
        }
    }
}

impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[repr(u8)]
#[allow(dead_code)]
pub enum Orientation {
    R0,
    R1,
    R2,
    R3,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::R0
    }
}

#[allow(dead_code)]
impl Orientation {
    #[inline(always)]
    pub fn as_i32(self) -> i32 {
        match self {
            Orientation::R0 => 0,
            Orientation::R1 => 1,
            Orientation::R2 => 2,
            Orientation::R3 => 3,
        }
    }

    pub fn cw(self) -> Orientation {
        match self {
            Orientation::R0 => Orientation::R1,
            Orientation::R1 => Orientation::R2,
            Orientation::R2 => Orientation::R3,
            Orientation::R3 => Orientation::R0,
        }
    }

    pub fn ccw(self) -> Orientation {
        match self {
            Orientation::R0 => Orientation::R3,
            Orientation::R1 => Orientation::R0,
            Orientation::R2 => Orientation::R1,
            Orientation::R3 => Orientation::R2,
        }
    }

    pub fn flip(self) -> Orientation {
        match self {
            Orientation::R0 => Orientation::R2,
            Orientation::R1 => Orientation::R3,
            Orientation::R2 => Orientation::R0,
            Orientation::R3 => Orientation::R1,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[repr(u8)]
pub enum Input {
    Left,
    Right,
    CW,
    CCW,
    Hold,
    SD,
    HD,
}
