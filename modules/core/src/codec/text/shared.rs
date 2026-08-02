use crate::{Error, Result};

use super::super::types::ScalarRef;

pub(crate) fn encoding_label(id: u8) -> Result<Option<&'static str>> {
    Ok(match id {
        0x00 => None,
        0x80 => Some("SHIFT-JIS"),
        0x20 => Some("ASCII"),
        0x40 => Some("ISO-8859-1"),
        0x60 => Some("EUC-JP"),
        0xa0 => Some("UTF-8"),
        _ => return Err(Error::Encoding("encoding id")),
    })
}

pub(crate) fn type_name(id: u8) -> Result<&'static str> {
    Ok(match id {
        0x02 => "s8",
        0x03 => "u8",
        0x04 => "s16",
        0x05 => "u16",
        0x06 => "s32",
        0x07 => "u32",
        0x08 => "s64",
        0x09 => "u64",
        0x0a => "bin",
        0x0b => "str",
        0x0c => "ip4",
        0x0d => "time",
        0x0e => "float",
        0x0f => "double",
        0x10 => "2s8",
        0x11 => "2u8",
        0x12 => "2s16",
        0x13 => "2u16",
        0x14 => "2s32",
        0x15 => "2u32",
        0x16 => "2s64",
        0x17 => "2u64",
        0x18 => "2f",
        0x19 => "2d",
        0x1a => "3s8",
        0x1b => "3u8",
        0x1c => "3s16",
        0x1d => "3u16",
        0x1e => "3s32",
        0x1f => "3u32",
        0x20 => "3s64",
        0x21 => "3u64",
        0x22 => "3f",
        0x23 => "3d",
        0x24 => "4s8",
        0x25 => "4u8",
        0x26 => "4s16",
        0x27 => "4u16",
        0x28 => "4s32",
        0x29 => "4u32",
        0x2a => "4s64",
        0x2b => "4u64",
        0x2c => "4f",
        0x2d => "4d",
        0x30 => "vs8",
        0x31 => "vu8",
        0x32 => "vs16",
        0x33 => "vu16",
        0x34 => "bool",
        0x35 => "2b",
        0x36 => "3b",
        0x37 => "4b",
        0x38 => "vb",
        _ => return Err(Error::Unsupported("XML type")),
    })
}

pub(crate) fn type_id(name: &str) -> Result<u8> {
    Ok(match name {
        "s8" => 0x02,
        "u8" => 0x03,
        "s16" => 0x04,
        "u16" => 0x05,
        "s32" => 0x06,
        "u32" => 0x07,
        "s64" => 0x08,
        "u64" => 0x09,
        "bin" | "binary" => 0x0a,
        "str" | "string" => 0x0b,
        "ip4" => 0x0c,
        "time" => 0x0d,
        "float" | "f" => 0x0e,
        "double" | "d" => 0x0f,
        "2s8" => 0x10,
        "2u8" => 0x11,
        "2s16" => 0x12,
        "2u16" => 0x13,
        "2s32" => 0x14,
        "2u32" => 0x15,
        "2s64" | "vs64" => 0x16,
        "2u64" | "vu64" => 0x17,
        "2f" => 0x18,
        "2d" | "vd" => 0x19,
        "3s8" => 0x1a,
        "3u8" => 0x1b,
        "3s16" => 0x1c,
        "3u16" => 0x1d,
        "3s32" => 0x1e,
        "3u32" => 0x1f,
        "3s64" => 0x20,
        "3u64" => 0x21,
        "3f" => 0x22,
        "3d" => 0x23,
        "4s8" => 0x24,
        "4u8" => 0x25,
        "4s16" => 0x26,
        "4u16" => 0x27,
        "4s32" | "vs32" => 0x28,
        "4u32" | "vu32" => 0x29,
        "4s64" => 0x2a,
        "4u64" => 0x2b,
        "4f" | "vf" => 0x2c,
        "4d" => 0x2d,
        "vs8" => 0x30,
        "vu8" => 0x31,
        "vs16" => 0x32,
        "vu16" => 0x33,
        "bool" | "b" => 0x34,
        "2b" => 0x35,
        "3b" => 0x36,
        "4b" => 0x37,
        "vb" => 0x38,
        _ => return Err(Error::Unsupported("XML type name")),
    })
}

pub(crate) fn scalar_text(value: ScalarRef<'_>) -> String {
    match value {
        ScalarRef::I8(v) => v.to_string(),
        ScalarRef::U8(v) => v.to_string(),
        ScalarRef::I16(v) => v.to_string(),
        ScalarRef::U16(v) => v.to_string(),
        ScalarRef::I32(v) => v.to_string(),
        ScalarRef::U32(v) => v.to_string(),
        ScalarRef::I64(v) => v.to_string(),
        ScalarRef::U64(v) => v.to_string(),
        ScalarRef::F32(v) => float_text(v as f64),
        ScalarRef::F64(v) => float_text(v),
        ScalarRef::Bool(v) => u8::from(v).to_string(),
        ScalarRef::Str(v) => v.to_owned(),
        ScalarRef::Binary(v) => v.iter().map(|b| format!("{b:02x}")).collect(),
        ScalarRef::Ip4(v) => format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3]),
    }
}

fn float_text(value: f64) -> String {
    if value.is_nan() {
        "(QNaN)".into()
    } else if value == f64::INFINITY {
        "(+INF)".into()
    } else if value == f64::NEG_INFINITY {
        "(-INF)".into()
    } else {
        format!("{value:.6}")
    }
}
