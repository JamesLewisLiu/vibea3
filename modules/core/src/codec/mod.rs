mod binary;
mod names;
mod packing;
mod primitive;
mod text;
mod types;

pub(crate) const NODE_TYPE: u8 = 0x01;
pub(crate) const BINARY_TYPE: u8 = 0x0a;
pub(crate) const STRING_TYPE: u8 = 0x0b;
pub(crate) const ATTRIBUTE_TYPE: u8 = 0x2e;
pub(crate) const TYPE_ID_MASK: u8 = 0x3f;
pub(crate) const ARRAY_FLAG: u8 = 0x40;

pub(crate) use binary::decode_kbin_meta;
pub use binary::{decode_kbin, encode_kbin};
#[doc(hidden)]
pub use primitive::Primitive;
pub(crate) use text::decode_xml_meta;
pub use text::{decode_xml, encode_xml};
pub use types::{Decoder, Encoder, WireDecode, WireEncode};

#[derive(Debug, Default)]
pub(crate) struct DecodeMeta {
    pub method: Option<String>,
}

pub(crate) fn encode_rpc_fault(
    format: PacketFormat,
    class: &str,
    status: i32,
    fault: &str,
    expire: Option<i32>,
    options: EncodeOptions,
) -> crate::Result<Vec<u8>> {
    match format {
        PacketFormat::Kbin => binary::encode_fault(class, status, fault, expire, options),
        PacketFormat::Xml => text::encode_fault(class, status, fault, expire, options),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketFormat {
    Kbin,
    Xml,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameMode {
    Packed,
    Full,
}

#[derive(Clone, Copy, Debug)]
pub struct EncodeOptions {
    pub encoding: u8,
    pub names: NameMode,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            encoding: 0x80,
            names: NameMode::Packed,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DecodeOptions {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_data: usize,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_nodes: 65_535,
            max_data: 32 * 1024 * 1024,
        }
    }
}
