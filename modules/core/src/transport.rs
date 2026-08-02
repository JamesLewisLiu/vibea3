use md5::{Digest, Md5};

use crate::{
    Error, Result,
    codec::PacketFormat,
    lz77::{self, CompressionLevel},
};

pub const GAME_KEY_BYTES: usize = 26;
const DERIVED_KEY_BYTES: usize = 16;
const EAMUSE_SECONDS_HEX_LENGTH: usize = 8;
const EAMUSE_SALT_HEX_LENGTH: usize = 4;
const EAMUSE_INFO_BYTES: usize = 6;
const HEX_PAIR_BYTES: usize = 2;
const RC4_STATE_SIZE: usize = 256;
const DEFAULT_MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_MAX_DECODED_BYTES: usize = 32 * 1024 * 1024;

pub const DEFAULT_GAME_KEY: &[u8; GAME_KEY_BYTES] = &[
    0x69, 0xd7, 0x46, 0x27, 0xd9, 0x85, 0xee, 0x21, 0x87, 0x16, 0x15, 0x70, 0xd0, 0x8d, 0x93, 0xb1,
    0x24, 0x55, 0x03, 0x5b, 0x6d, 0xf0, 0xd8, 0x20, 0x5d, 0xf5,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Compression {
    #[default]
    None,
    Lz77,
}

impl Compression {
    pub fn parse(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("none").to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "lz77" => Ok(Self::Lz77),
            _ => Err(Error::Transport("unsupported X-Compress value".into())),
        }
    }

    pub fn header(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Lz77 => "lz77",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransportConfig {
    pub game_key: [u8; GAME_KEY_BYTES],
    pub max_body: usize,
    pub max_decoded: usize,
    pub compression_level: CompressionLevel,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            game_key: *DEFAULT_GAME_KEY,
            max_body: DEFAULT_MAX_BODY_BYTES,
            max_decoded: DEFAULT_MAX_DECODED_BYTES,
            compression_level: CompressionLevel::Balanced,
        }
    }
}

#[derive(Debug)]
pub struct DecodedBody {
    pub bytes: Vec<u8>,
    pub format: PacketFormat,
}

#[derive(Debug)]
pub struct EncodedBody {
    pub bytes: Vec<u8>,
    pub compression: Compression,
}

pub fn decode(
    body: &[u8],
    eamuse_info: Option<&str>,
    compression: Compression,
    config: &TransportConfig,
) -> Result<DecodedBody> {
    if body.len() > config.max_body {
        return Err(Error::Limit);
    }
    let mut bytes = body.to_vec();
    if let Some(info) = eamuse_info {
        apply_eamuse_rc4(&mut bytes, info, &config.game_key)?;
    }
    if compression == Compression::Lz77 {
        bytes = lz77::decompress(&bytes, config.max_decoded)?;
    }
    if bytes.len() > config.max_decoded {
        return Err(Error::Limit);
    }
    let format = match bytes.first() {
        Some(0xa0) => PacketFormat::Kbin,
        Some(b'<') | Some(b'\xef') => PacketFormat::Xml,
        _ => return Err(Error::Transport("body is neither XML nor kbinxml".into())),
    };
    Ok(DecodedBody { bytes, format })
}

pub fn encode(
    body: &[u8],
    eamuse_info: Option<&str>,
    compression: Compression,
    config: &TransportConfig,
) -> Result<Vec<u8>> {
    if body.len() > config.max_decoded {
        return Err(Error::Limit);
    }
    let mut bytes = match compression {
        Compression::None => body.to_vec(),
        Compression::Lz77 => lz77::compress(body, config.compression_level),
    };
    if let Some(info) = eamuse_info {
        apply_eamuse_rc4(&mut bytes, info, &config.game_key)?;
    }
    if bytes.len() > config.max_body {
        return Err(Error::Limit);
    }
    Ok(bytes)
}

/// Encode a response using AVS' adaptive compression behavior.
///
/// The official transport only emits `X-Compress: lz77` when the compressed
/// representation is strictly smaller than the original body.
pub fn encode_adaptive(
    body: &[u8],
    eamuse_info: Option<&str>,
    compression: Compression,
    config: &TransportConfig,
) -> Result<EncodedBody> {
    if body.len() > config.max_decoded {
        return Err(Error::Limit);
    }
    let (mut bytes, compression) = match compression {
        Compression::None => (body.to_vec(), Compression::None),
        Compression::Lz77 => {
            let compressed = lz77::compress(body, config.compression_level);
            if compressed.len() < body.len() {
                (compressed, Compression::Lz77)
            } else {
                (body.to_vec(), Compression::None)
            }
        }
    };
    if let Some(info) = eamuse_info {
        apply_eamuse_rc4(&mut bytes, info, &config.game_key)?;
    }
    if bytes.len() > config.max_body {
        return Err(Error::Limit);
    }
    Ok(EncodedBody { bytes, compression })
}

pub fn derive_key(info: &str, game_key: &[u8; GAME_KEY_BYTES]) -> Result<[u8; DERIVED_KEY_BYTES]> {
    let bytes = parse_eamuse_info(info)?;
    let mut md5 = Md5::new();
    md5.update(bytes);
    md5.update(game_key);
    Ok(md5.finalize().into())
}

pub fn apply_eamuse_rc4(
    data: &mut [u8],
    info: &str,
    game_key: &[u8; GAME_KEY_BYTES],
) -> Result<()> {
    let key = derive_key(info, game_key)?;
    rc4(data, &key);
    Ok(())
}

fn parse_eamuse_info(info: &str) -> Result<[u8; EAMUSE_INFO_BYTES]> {
    let Some(rest) = info.strip_prefix("1-") else {
        return Err(Error::Transport("invalid X-Eamuse-Info version".into()));
    };
    let Some((seconds, salt)) = rest.split_once('-') else {
        return Err(Error::Transport("invalid X-Eamuse-Info".into()));
    };
    if seconds.len() != EAMUSE_SECONDS_HEX_LENGTH
        || salt.len() != EAMUSE_SALT_HEX_LENGTH
        || !seconds
            .bytes()
            .chain(salt.bytes())
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Transport("invalid X-Eamuse-Info".into()));
    }
    let mut out = [0u8; EAMUSE_INFO_BYTES];
    for (index, pair) in seconds
        .as_bytes()
        .chunks_exact(HEX_PAIR_BYTES)
        .chain(salt.as_bytes().chunks_exact(HEX_PAIR_BYTES))
        .enumerate()
    {
        out[index] = u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16)
            .map_err(|_| Error::Transport("invalid X-Eamuse-Info hex".into()))?;
    }
    Ok(out)
}

fn rc4(data: &mut [u8], key: &[u8]) {
    let mut state = [0u8; RC4_STATE_SIZE];
    for (index, value) in state.iter_mut().enumerate() {
        *value = index as u8;
    }
    let mut j = 0u8;
    for i in 0..RC4_STATE_SIZE {
        j = j.wrapping_add(state[i]).wrapping_add(key[i % key.len()]);
        state.swap(i, usize::from(j));
    }
    let mut i = 0u8;
    j = 0;
    for byte in data {
        i = i.wrapping_add(1);
        j = j.wrapping_add(state[usize::from(i)]);
        state.swap(usize::from(i), usize::from(j));
        let index = state[usize::from(i)].wrapping_add(state[usize::from(j)]);
        *byte ^= state[usize::from(index)];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_is_symmetric() {
        let mut data = b"hello".to_vec();
        apply_eamuse_rc4(&mut data, "1-12345678-abcd", DEFAULT_GAME_KEY).unwrap();
        assert_ne!(data, b"hello");
        apply_eamuse_rc4(&mut data, "1-12345678-abcd", DEFAULT_GAME_KEY).unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn key_format_is_strict() {
        assert!(derive_key("1-12345678-abcd", DEFAULT_GAME_KEY).is_ok());
        assert!(derive_key("2-12345678-abcd", DEFAULT_GAME_KEY).is_err());
        assert!(derive_key("1-12345678-ABCD", DEFAULT_GAME_KEY).is_err());
    }

    #[test]
    fn adaptive_compression_requires_a_strict_size_win() {
        let config = TransportConfig::default();
        let small = encode_adaptive(b"x", None, Compression::Lz77, &config).unwrap();
        assert_eq!(small.compression, Compression::None);
        assert_eq!(small.bytes, b"x");

        let large = encode_adaptive(&[0; 4096], None, Compression::Lz77, &config).unwrap();
        assert_eq!(large.compression, Compression::Lz77);
        assert!(large.bytes.len() < 4096);
    }

    #[test]
    fn documented_key_derivation_vector() {
        assert_eq!(
            derive_key("1-12345678-abcd", DEFAULT_GAME_KEY).unwrap(),
            [
                0x24, 0x62, 0x54, 0xab, 0x05, 0xeb, 0xa6, 0x1f, 0x90, 0x3b, 0x1b, 0x48, 0xb2, 0xf2,
                0x8e, 0xd1
            ]
        );
    }
}
