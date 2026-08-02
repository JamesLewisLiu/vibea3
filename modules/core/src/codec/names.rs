use encoding_rs::{EUC_JP, SHIFT_JIS};

use crate::{Error, Result};

use super::NameMode;

const ALPHABET: &[u8; 64] = b"0123456789:ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz";
const BITS_PER_BYTE: usize = 8;
const PACKED_CHARACTER_BITS: usize = 6;
const PACKED_NAME_MAX_BYTES: usize = 36;
const FULL_NAME_MAX_BYTES: usize = 4096;
const SHORT_FULL_NAME_MAX_BYTES: usize = 64;
const SHORT_FULL_NAME_FLAG: u8 = 0x40;
const SHORT_FULL_NAME_HEADER_MAX: u8 = 0x7f;
const LONG_FULL_NAME_OFFSET: usize = 0x7fbf;
const ASCII_ENCODING: u8 = 0x20;
const ISO_8859_1_ENCODING: u8 = 0x40;
const SHIFT_JIS_ENCODING: u8 = 0x00;
const SHIFT_JIS_ALT_ENCODING: u8 = 0x80;
const EUC_JP_ENCODING: u8 = 0x60;
const UTF8_ENCODING: u8 = 0xa0;

pub(crate) fn write_name(
    out: &mut Vec<u8>,
    name: &str,
    mode: NameMode,
    encoding: u8,
) -> Result<()> {
    match mode {
        NameMode::Packed => {
            if name.len() > PACKED_NAME_MAX_BYTES {
                return Err(Error::Schema("packed name exceeds 36 bytes".into()));
            }
            out.push(name.len() as u8);
            let mut acc = 0u32;
            let mut bits = 0usize;
            for byte in name.bytes() {
                let value = ALPHABET.iter().position(|v| *v == byte).ok_or_else(|| {
                    Error::Schema(format!("invalid packed-name character {}", byte as char))
                })? as u32;
                acc = (acc << PACKED_CHARACTER_BITS) | value;
                bits += PACKED_CHARACTER_BITS;
                while bits >= BITS_PER_BYTE {
                    bits -= BITS_PER_BYTE;
                    out.push((acc >> bits) as u8);
                    acc &= (1u32 << bits).wrapping_sub(1);
                }
            }
            if bits != 0 {
                out.push((acc << (BITS_PER_BYTE - bits)) as u8);
            }
        }
        NameMode::Full => {
            let encoded = encode_text(name, encoding)?;
            let len = encoded.len();
            if !(1..=FULL_NAME_MAX_BYTES).contains(&len) {
                return Err(Error::Schema("full name length out of range".into()));
            }
            if len <= SHORT_FULL_NAME_MAX_BYTES {
                out.push((len - 1) as u8 | SHORT_FULL_NAME_FLAG);
            } else {
                out.extend_from_slice(&((len + LONG_FULL_NAME_OFFSET) as u16).to_be_bytes());
            }
            out.extend_from_slice(&encoded);
        }
    }
    Ok(())
}

pub(crate) fn read_name(
    input: &[u8],
    pos: &mut usize,
    end: usize,
    mode: NameMode,
    encoding: u8,
) -> Result<String> {
    match mode {
        NameMode::Packed => {
            if *pos >= end {
                return Err(Error::Schema("missing packed name length".into()));
            }
            let len = input[*pos] as usize;
            *pos += 1;
            let bytes = (len * PACKED_CHARACTER_BITS).div_ceil(BITS_PER_BYTE);
            if *pos + bytes > end {
                return Err(Error::Schema("packed name out of bounds".into()));
            }
            let mut out = String::with_capacity(len);
            let mut bit = 0usize;
            for _ in 0..len {
                let mut value = 0u8;
                for _ in 0..PACKED_CHARACTER_BITS {
                    value = (value << 1)
                        | ((input[*pos + bit / BITS_PER_BYTE]
                            >> (BITS_PER_BYTE - 1 - bit % BITS_PER_BYTE))
                            & 1);
                    bit += 1;
                }
                out.push(ALPHABET[value as usize] as char);
            }
            *pos += bytes;
            Ok(out)
        }
        NameMode::Full => {
            if *pos >= end {
                return Err(Error::Schema("missing full name length".into()));
            }
            let first = input[*pos];
            *pos += 1;
            let len = if first <= SHORT_FULL_NAME_HEADER_MAX {
                (first as usize & !usize::from(SHORT_FULL_NAME_FLAG)) + 1
            } else {
                if *pos >= end {
                    return Err(Error::Schema("truncated long name length".into()));
                }
                let raw = u16::from_be_bytes([first, input[*pos]]) as usize;
                *pos += 1;
                raw - LONG_FULL_NAME_OFFSET
            };
            if *pos + len > end {
                return Err(Error::Schema("full name out of bounds".into()));
            }
            let value = decode_text(&input[*pos..*pos + len], encoding)?;
            *pos += len;
            Ok(value)
        }
    }
}

pub(crate) fn encode_text(text: &str, encoding: u8) -> Result<Vec<u8>> {
    match encoding {
        ASCII_ENCODING if text.is_ascii() => Ok(text.as_bytes().to_vec()),
        ASCII_ENCODING => Err(Error::Encoding("ASCII")),
        ISO_8859_1_ENCODING => text
            .chars()
            .map(|ch| u8::try_from(ch as u32).map_err(|_| Error::Encoding("ISO-8859-1")))
            .collect(),
        SHIFT_JIS_ENCODING | SHIFT_JIS_ALT_ENCODING => encode_legacy(text, SHIFT_JIS, "SHIFT-JIS"),
        EUC_JP_ENCODING => encode_legacy(text, EUC_JP, "EUC-JP"),
        UTF8_ENCODING => Ok(text.as_bytes().to_vec()),
        _ => Err(Error::Encoding("encoding id")),
    }
}

pub(crate) fn decode_text(bytes: &[u8], encoding: u8) -> Result<String> {
    match encoding {
        ASCII_ENCODING if bytes.is_ascii() => Ok(String::from_utf8(bytes.to_vec()).unwrap()),
        ASCII_ENCODING => Err(Error::Encoding("ASCII")),
        ISO_8859_1_ENCODING => Ok(bytes.iter().map(|b| char::from(*b)).collect()),
        SHIFT_JIS_ENCODING | SHIFT_JIS_ALT_ENCODING => decode_legacy(bytes, SHIFT_JIS, "SHIFT-JIS"),
        EUC_JP_ENCODING => decode_legacy(bytes, EUC_JP, "EUC-JP"),
        UTF8_ENCODING => String::from_utf8(bytes.to_vec()).map_err(|_| Error::Encoding("UTF-8")),
        _ => Err(Error::Encoding("encoding id")),
    }
}

fn encode_legacy(
    text: &str,
    encoding: &'static encoding_rs::Encoding,
    name: &'static str,
) -> Result<Vec<u8>> {
    let (bytes, _, bad) = encoding.encode(text);
    if bad {
        Err(Error::Encoding(name))
    } else {
        Ok(bytes.into_owned())
    }
}

fn decode_legacy(
    bytes: &[u8],
    encoding: &'static encoding_rs::Encoding,
    name: &'static str,
) -> Result<String> {
    let (text, _, bad) = encoding.decode(bytes);
    if bad {
        Err(Error::Encoding(name))
    } else {
        Ok(text.into_owned())
    }
}
