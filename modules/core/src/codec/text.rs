mod reader;
mod shared;
mod writer;

pub(crate) use reader::XmlReader;
pub(crate) use writer::XmlWriter;

use crate::{Result, schema::Kbin};

use super::{
    DecodeMeta, DecodeOptions, EncodeOptions,
    types::{Encoder, EncoderInner},
};

pub fn encode_xml<T: Kbin>(value: &T, options: EncodeOptions) -> Result<Vec<u8>> {
    let mut writer = XmlWriter::new(options)?;
    writer.begin_node(T::SCHEMA.node)?;
    value.encode(&mut Encoder {
        inner: EncoderInner::Xml(&mut writer),
    })?;
    writer.end_node()?;
    writer.finish()
}

pub fn decode_xml<T: Kbin>(input: &[u8], options: DecodeOptions) -> Result<T> {
    XmlReader::new(input, options)?.decode_root::<T>()
}

pub(crate) fn decode_xml_meta<T: Kbin>(
    input: &[u8],
    options: DecodeOptions,
) -> Result<(T, DecodeMeta)> {
    let mut reader = XmlReader::new(input, options)?;
    reader.capture_method = true;
    let value = reader.decode_root::<T>()?;
    Ok((
        value,
        DecodeMeta {
            method: reader.method,
        },
    ))
}

pub(crate) fn encode_fault(
    class: &str,
    status: i32,
    fault: &str,
    expire: Option<i32>,
    options: EncodeOptions,
) -> Result<Vec<u8>> {
    writer::encode_fault(class, status, fault, expire, options)
}
