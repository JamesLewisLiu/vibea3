//! Konami's 4 KiB-window LZSS transport codec.

use crate::{Error, Result};

const WINDOW: usize = 0x1000;
const MAX_DISTANCE: usize = WINDOW - 1;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 18;
const HASH_SIZE: usize = 1 << 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompressionLevel {
    Fast,
    #[default]
    Balanced,
    Best,
}

impl CompressionLevel {
    fn probes(self) -> usize {
        match self {
            Self::Fast => 16,
            Self::Balanced => 64,
            Self::Best => 512,
        }
    }
    fn lazy(self) -> bool {
        !matches!(self, Self::Fast)
    }
}

pub struct Scratch {
    head: Vec<i32>,
    prev: Vec<i32>,
}

impl Default for Scratch {
    fn default() -> Self {
        Self {
            head: vec![-1; HASH_SIZE],
            prev: vec![-1; WINDOW],
        }
    }
}

impl Scratch {
    fn reset(&mut self) {
        self.head.fill(-1);
        self.prev.fill(-1);
    }
}

pub fn compress(input: &[u8], level: CompressionLevel) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + input.len().div_ceil(8) + 3);
    let mut scratch = Scratch::default();
    compress_into(input, level, &mut out, &mut scratch);
    out
}

pub fn compress_into(
    input: &[u8],
    level: CompressionLevel,
    out: &mut Vec<u8>,
    scratch: &mut Scratch,
) {
    out.clear();
    scratch.reset();
    encode_matches(input, level, out, scratch);

    let mut literals = Vec::with_capacity(input.len() + input.len().div_ceil(8) + 3);
    encode_literals(input, &mut literals);
    if literals.len() < out.len() {
        *out = literals;
    }
}

fn encode_matches(input: &[u8], level: CompressionLevel, out: &mut Vec<u8>, scratch: &mut Scratch) {
    // Kbin's aligned data section contains many `00 00 xx yy` prefixes. A
    // fourth hash byte separates those fields instead of spending the short
    // Fast/Balanced chains on collisions. Best retains the exhaustive generic
    // chain because it produces a slightly smaller result on large corpora.
    let kbin = !matches!(level, CompressionLevel::Best)
        && matches!(input.get(..2), Some([0xa0, 0x42 | 0x45]));
    let mut pos = 0usize;
    loop {
        let flag_at = out.len();
        out.push(0);
        let mut flags = 0u8;
        for bit in 0..8 {
            if pos >= input.len() {
                out.extend_from_slice(&[0, 0]);
                out[flag_at] = flags;
                return;
            }

            let current = find_match(input, pos, level.probes(), scratch, kbin);
            insert(input, pos, scratch, kbin);
            let lazy = if level.lazy() && current.1 >= MIN_MATCH && pos + 1 < input.len() {
                let next = find_match(input, pos + 1, level.probes(), scratch, kbin);
                next.1 > current.1 + 1
            } else {
                false
            };

            if current.1 >= MIN_MATCH && !lazy {
                let token = ((current.0 as u16) << 4) | ((current.1 - MIN_MATCH) as u16);
                out.extend_from_slice(&token.to_be_bytes());
                for skipped in 1..current.1 {
                    insert(input, pos + skipped, scratch, kbin);
                }
                pos += current.1;
            } else {
                flags |= 1 << bit;
                out.push(input[pos]);
                pos += 1;
            }
        }
        out[flag_at] = flags;
    }
}

fn encode_literals(input: &[u8], out: &mut Vec<u8>) {
    let mut pos = 0;
    loop {
        let flag_at = out.len();
        out.push(0);
        let mut flags = 0u8;
        for bit in 0..8 {
            if pos == input.len() {
                out.extend_from_slice(&[0, 0]);
                out[flag_at] = flags;
                return;
            }
            flags |= 1 << bit;
            out.push(input[pos]);
            pos += 1;
        }
        out[flag_at] = flags;
    }
}

fn hash(input: &[u8], pos: usize, kbin: bool) -> Option<usize> {
    let bytes = input.get(pos..pos + 3)?;
    if kbin
        && usize::from(bytes[0] == 0) + usize::from(bytes[1] == 0) + usize::from(bytes[2] == 0) >= 2
        && let Some(bytes) = input.get(pos..pos + 4)
    {
        let mut h = u32::from_be_bytes(bytes.try_into().unwrap());
        h ^= h >> 16;
        h = h.wrapping_mul(0x7feb_352d);
        h ^= h >> 15;
        return Some((h as usize) & (HASH_SIZE - 1));
    }
    let mut h = u32::from(bytes[0]) * 251;
    h = (h ^ u32::from(bytes[1])) * 251;
    h ^= u32::from(bytes[2]);
    Some((h as usize) & (HASH_SIZE - 1))
}

fn insert(input: &[u8], pos: usize, scratch: &mut Scratch, kbin: bool) {
    if let Some(h) = hash(input, pos, kbin) {
        scratch.prev[pos & (WINDOW - 1)] = scratch.head[h];
        scratch.head[h] = pos as i32;
    }
}

fn find_match(
    input: &[u8],
    pos: usize,
    probes: usize,
    scratch: &Scratch,
    kbin: bool,
) -> (usize, usize) {
    let zero_len = if pos < MAX_DISTANCE {
        input[pos..]
            .iter()
            .take(MAX_MATCH)
            .take_while(|byte| **byte == 0)
            .count()
    } else {
        0
    };
    let Some(h) = hash(input, pos, kbin) else {
        return if zero_len >= MIN_MATCH {
            (MAX_DISTANCE, zero_len)
        } else {
            (0, 0)
        };
    };
    let mut best_distance = if zero_len >= MIN_MATCH {
        MAX_DISTANCE
    } else {
        0
    };
    let mut best_len = zero_len;
    let max_len = MAX_MATCH.min(input.len() - pos);
    search_chain(
        input,
        pos,
        scratch.head[h],
        &scratch.prev,
        probes,
        max_len,
        &mut best_distance,
        &mut best_len,
    );
    (best_distance, best_len)
}

#[allow(clippy::too_many_arguments)]
fn search_chain(
    input: &[u8],
    pos: usize,
    mut candidate: i32,
    prev: &[i32],
    probes: usize,
    max_len: usize,
    best_distance: &mut usize,
    best_len: &mut usize,
) {
    let mut checked = 0;
    while candidate >= 0 && checked < probes {
        let at = candidate as usize;
        let distance = pos.saturating_sub(at);
        if distance == 0 || distance > MAX_DISTANCE {
            break;
        }
        let mut len = 0;
        while len < max_len && input[at + len] == input[pos + len] {
            len += 1;
        }
        if len > *best_len {
            *best_len = len;
            *best_distance = distance;
            if len == max_len {
                return;
            }
        }
        candidate = prev[at & (WINDOW - 1)];
        checked += 1;
    }
}

pub fn decompress(input: &[u8], max_output: usize) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    decompress_into(input, &mut out, max_output)?;
    Ok(out)
}

pub fn decompress_into(input: &[u8], out: &mut Vec<u8>, max_output: usize) -> Result<()> {
    out.clear();
    let mut pos = 0usize;
    loop {
        let flags = *input.get(pos).ok_or(Error::Lz77("missing flag byte"))?;
        pos += 1;
        for bit in 0..8 {
            if flags & (1 << bit) != 0 {
                let byte = *input.get(pos).ok_or(Error::Lz77("truncated literal"))?;
                pos += 1;
                push_checked(out, byte, max_output)?;
                continue;
            }
            let token = u16::from_be_bytes([
                *input.get(pos).ok_or(Error::Lz77("truncated match"))?,
                *input.get(pos + 1).ok_or(Error::Lz77("truncated match"))?,
            ]);
            pos += 2;
            let distance = usize::from(token >> 4);
            if distance == 0 {
                return Ok(());
            }
            let mut len = usize::from(token & 0x0f) + MIN_MATCH;

            if distance > out.len() {
                let zeros = (distance - out.len()).min(len);
                if out.len() + zeros > max_output {
                    return Err(Error::Limit);
                }
                out.resize(out.len() + zeros, 0);
                len -= zeros;
            }
            for _ in 0..len {
                let source = out
                    .len()
                    .checked_sub(distance)
                    .ok_or(Error::Lz77("invalid distance"))?;
                let byte = out[source];
                push_checked(out, byte, max_output)?;
            }
        }
    }
}

fn push_checked(out: &mut Vec<u8>, byte: u8, max: usize) -> Result<()> {
    if out.len() == max {
        return Err(Error::Limit);
    }
    out.push(byte);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn round_trip() {
        for data in [
            b"".as_slice(),
            b"hello world",
            b"aaaaaaaaaaaaaaaaaaaaaaaa",
            &[0; 80],
        ] {
            for level in [
                CompressionLevel::Fast,
                CompressionLevel::Balanced,
                CompressionLevel::Best,
            ] {
                let packed = compress(data, level);
                assert_eq!(decompress(&packed, 1024).unwrap(), data);
            }
        }
    }

    #[test]
    fn overlapping_match() {
        let packed = [0x01, b'a', 0x00, 0x1f, 0x00, 0x00];
        assert_eq!(decompress(&packed, 64).unwrap(), vec![b'a'; 19]);
    }

    proptest! {
        #[test]
        fn arbitrary_round_trip(data in proptest::collection::vec(any::<u8>(), 0..8192)) {
            let packed = compress(&data, CompressionLevel::Balanced);
            prop_assert!(packed.len() <= data.len() + data.len().div_ceil(8) + 3);
            prop_assert_eq!(decompress(&packed, 8192).unwrap(), data);
        }
    }

    #[test]
    fn kbin_profile_separates_big_endian_field_prefixes() {
        let mut kbin = vec![0xa0, 0x42, 0x80, 0x7f];
        for _ in 0..512 {
            for value in 0u32..128 {
                kbin.extend_from_slice(&value.to_be_bytes());
            }
        }
        let mut generic = kbin.clone();
        generic[0] = 0xa1;
        assert!(
            compress(&kbin, CompressionLevel::Balanced).len()
                < compress(&generic, CompressionLevel::Balanced).len()
        );
    }
}
