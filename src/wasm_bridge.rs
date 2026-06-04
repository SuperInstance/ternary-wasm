//! Serialize/deserialize ternary data for JS↔Rust communication.

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String};

/// Ternary trit value: Negative (-1), Zero (0), or Positive (+1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum Trit {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

impl Trit {
    /// Convert from i8; clamps to nearest trit.
    pub fn from_i8(v: i8) -> Self {
        match v {
            ..=-1 => Trit::Neg,
            0 => Trit::Zero,
            1.. => Trit::Pos,
        }
    }

    /// Multiply two trits (balanced ternary multiplication).
    pub fn mul(self, other: Trit) -> Trit {
        match (self, other) {
            (Trit::Zero, _) | (_, Trit::Zero) => Trit::Zero,
            (Trit::Neg, Trit::Neg) | (Trit::Pos, Trit::Pos) => Trit::Pos,
            _ => Trit::Neg,
        }
    }

    /// Add two trits, returning (carry, sum).
    pub fn add(self, other: Trit) -> (Trit, Trit) {
        let s = self as i8 + other as i8;
        match s {
            -2 => (Trit::Neg, Trit::Pos),
            -1 => (Trit::Zero, Trit::Neg),
            0 => (Trit::Zero, Trit::Zero),
            1 => (Trit::Zero, Trit::Pos),
            2 => (Trit::Pos, Trit::Neg),
            _ => unreachable!(),
        }
    }
}

/// A tryte is 6 trits (range -364..=364).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tryte(pub [Trit; 6]);

impl Tryte {
    pub const ZERO: Tryte = Tryte([Trit::Zero; 6]);

    /// Convert a balanced ternary tryte to its integer value.
    pub fn to_i32(self) -> i32 {
        let mut val: i32 = 0;
        let mut pow: i32 = 1;
        for i in 0..6 {
            val += self.0[i] as i32 * pow;
            pow *= 3;
        }
        val
    }

    /// Convert an integer to a tryte (clamped to range).
    pub fn from_i32(mut v: i32) -> Self {
        const POW3_6: i32 = 364;
        if v > POW3_6 { v = POW3_6; }
        if v < -POW3_6 { v = -POW3_6; }

        let mut trits = [Trit::Zero; 6];
        let mut idx = 0;
        while v != 0 && idx < 6 {
            let r = v.rem_euclid(3);
            match r {
                0 => {
                    trits[idx] = Trit::Zero;
                    v = v.div_euclid(3);
                }
                1 => {
                    trits[idx] = Trit::Pos;
                    v = (v - 1) / 3;
                }
                2 => {
                    trits[idx] = Trit::Neg;
                    v = (v + 1) / 3;
                }
                _ => unreachable!(),
            }
            idx += 1;
        }
        Tryte(trits)
    }
}

/// Bridge for serializing/deserializing ternary data between Rust and JS.
pub struct WasmBridge;

impl WasmBridge {
    /// Serialize a slice of trits into a byte buffer.
    /// Each byte packs 5 trits (2 bits each, 10 bits → 2 bytes per 5 trits).
    pub fn serialize_trits(trits: &[Trit]) -> Vec<u8> {
        let mut out = Vec::with_capacity((trits.len() * 2 + 4) / 5);
        for chunk in trits.chunks(4) {
            let mut byte: u8 = 0;
            for (i, t) in chunk.iter().enumerate() {
                let bits = match t {
                    Trit::Neg => 0b00,
                    Trit::Zero => 0b01,
                    Trit::Pos => 0b10,
                };
                byte |= bits << (i * 2);
            }
            out.push(byte);
        }
        out
    }

    /// Deserialize trits from a byte buffer.
    pub fn deserialize_trits(bytes: &[u8], count: usize) -> Vec<Trit> {
        let mut trits = Vec::with_capacity(count);
        for &byte in bytes {
            for i in 0..4 {
                if trits.len() >= count { break; }
                let bits = (byte >> (i * 2)) & 0b11;
                let trit = match bits {
                    0b00 => Trit::Neg,
                    0b01 => Trit::Zero,
                    0b10 => Trit::Pos,
                    _ => Trit::Zero, // 0b11 unused, treat as Zero
                };
                trits.push(trit);
            }
        }
        trits.truncate(count);
        trits
    }

    /// Encode a tryte array to a flat i16 buffer (for JS DataView).
    pub fn encode_trytes_as_i16(trytes: &[Tryte]) -> Vec<i16> {
        trytes.iter().map(|t| t.to_i32() as i16).collect()
    }

    /// Decode an i16 buffer back to trytes.
    pub fn decode_trytes_from_i16(vals: &[i16]) -> Vec<Tryte> {
        vals.iter().map(|&v| Tryte::from_i32(v as i32)).collect()
    }

    /// Convert a string to ternary representation (each char → tryte of its ASCII value).
    pub fn string_to_trytes(s: &str) -> Vec<Tryte> {
        s.bytes().map(|b| Tryte::from_i32(b as i32)).collect()
    }

    /// Convert ternary representation back to string.
    pub fn trytes_to_string(trytes: &[Tryte]) -> String {
        let bytes: Vec<u8> = trytes
            .iter()
            .map(|t| {
                let v = t.to_i32();
                if v >= 0 && v <= 127 { v as u8 } else { b'?' }
            })
            .collect();
        String::from_utf8_lossy(&bytes).into()
    }
}
