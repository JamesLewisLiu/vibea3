use bytes::Bytes;

use crate::{
    Result,
    error::Error,
    schema::{FieldKind, Kbin},
    value::WireString,
};

use super::{
    ARRAY_FLAG, ATTRIBUTE_TYPE, BINARY_TYPE, DecodeMeta, DecodeOptions, EncodeOptions, NODE_TYPE,
    NameMode, Primitive, STRING_TYPE, TYPE_ID_MASK,
    names::{encode_text, read_name, write_name},
    packing::{DataReader, DataWriter},
    types::{Decoder, DecoderInner, Encoder, EncoderInner, OwnedScalar, ScalarRef},
};

const MAGIC: u8 = 0xa0;
// Section terminators always carry the format's array/high bit. Their base
// type IDs are 0x3e/0x3f, but the bytes on the wire are 0xfe/0xff.
const NODE_END: u8 = 0xfe;
const FILE_END: u8 = 0xff;
const PACKED_DATA: u8 = 0x42;
const FULL_DATA: u8 = 0x45;

pub fn encode_kbin<T: Kbin>(value: &T, options: EncodeOptions) -> Result<Vec<u8>> {
    let mut writer = BinaryWriter::new(options);
    writer.begin_node(T::SCHEMA.node)?;
    value.encode(&mut Encoder {
        inner: EncoderInner::Binary(&mut writer),
    })?;
    writer.end_node()?;
    writer.finish()
}

pub fn decode_kbin<T: Kbin>(input: &[u8], options: DecodeOptions) -> Result<T> {
    let mut reader = BinaryReader::new(input, options)?;
    reader.decode_root::<T>()
}

pub(crate) fn decode_kbin_meta<T: Kbin>(
    input: &[u8],
    options: DecodeOptions,
) -> Result<(T, DecodeMeta)> {
    let mut reader = BinaryReader::new(input, options)?;
    reader.capture_method = true;
    let value = reader.decode_root::<T>()?;
    Ok((
        value,
        DecodeMeta {
            method: reader.method,
        },
    ))
}

pub(crate) struct BinaryWriter {
    options: EncodeOptions,
    schema: Vec<u8>,
    data: DataWriter,
    nodes: Vec<PendingNode>,
}

struct PendingNode {
    attributes: Vec<PendingAttribute>,
    content_started: bool,
}

struct PendingAttribute {
    name: String,
    bytes: Vec<u8>,
}

impl BinaryWriter {
    pub(crate) fn new(options: EncodeOptions) -> Self {
        Self {
            options,
            schema: Vec::with_capacity(256),
            data: DataWriter::default(),
            nodes: Vec::new(),
        }
    }

    pub(crate) fn begin_node(&mut self, name: &str) -> Result<()> {
        self.start_content()?;
        self.schema.push(NODE_TYPE);
        write_name(
            &mut self.schema,
            name,
            self.options.names,
            self.options.encoding,
        )?;
        self.nodes.push(PendingNode {
            attributes: Vec::new(),
            content_started: false,
        });
        Ok(())
    }

    pub(crate) fn end_node(&mut self) -> Result<()> {
        self.flush_attributes()?;
        if self.nodes.pop().is_none() {
            return Err(Error::Schema("node stack underflow".into()));
        }
        self.schema.push(NODE_END);
        Ok(())
    }

    pub(crate) fn scalar(
        &mut self,
        name: &str,
        kind: FieldKind,
        type_id: u8,
        value: ScalarRef<'_>,
    ) -> Result<()> {
        if kind == FieldKind::Attribute {
            let node = self
                .nodes
                .last_mut()
                .ok_or_else(|| Error::Schema("attribute outside node".into()))?;
            if node.content_started {
                return Err(Error::Schema("attribute emitted after child".into()));
            }
            let text = scalar_text(value);
            let bytes = encode_text(&text, self.options.encoding)?;
            node.attributes.push(PendingAttribute {
                name: name.to_owned(),
                bytes,
            });
            return Ok(());
        }

        self.start_content()?;
        self.schema.push(type_id);
        write_name(
            &mut self.schema,
            name,
            self.options.names,
            self.options.encoding,
        )?;
        let result = if let ScalarRef::Str(text) = value {
            let bytes = encode_text(text, self.options.encoding)?;
            self.data.variable(&bytes, true)
        } else {
            self.data.scalar(value)
        };
        result?;
        self.schema.push(NODE_END);
        Ok(())
    }

    pub(crate) fn array<T: Primitive>(&mut self, name: &str, values: &[T]) -> Result<()> {
        self.start_content()?;
        self.schema.push(T::TYPE_ID | ARRAY_FLAG);
        write_name(
            &mut self.schema,
            name,
            self.options.names,
            self.options.encoding,
        )?;
        let mut bytes = Vec::with_capacity(values.len() * T::SIZE);
        for value in values {
            value.append_be(&mut bytes);
        }
        self.data.variable(&bytes, false)?;
        self.schema.push(NODE_END);
        Ok(())
    }

    pub(crate) fn fixed<T: Primitive, const N: usize>(
        &mut self,
        name: &str,
        type_id: u8,
        values: &[T; N],
    ) -> Result<()> {
        self.start_content()?;
        self.schema.push(type_id);
        write_name(
            &mut self.schema,
            name,
            self.options.names,
            self.options.encoding,
        )?;
        let mut bytes = Vec::with_capacity(N * T::SIZE);
        for value in values {
            value.append_be(&mut bytes);
        }
        self.data.put(&bytes);
        self.schema.push(NODE_END);
        Ok(())
    }

    pub(crate) fn wire_string(
        &mut self,
        name: &str,
        kind: FieldKind,
        value: &WireString,
    ) -> Result<()> {
        if value.encoding != self.options.encoding {
            return Err(Error::Encoding(
                "WireString encoding does not match packet encoding",
            ));
        }
        if kind == FieldKind::Attribute {
            let node = self
                .nodes
                .last_mut()
                .ok_or_else(|| Error::Schema("attribute outside node".into()))?;
            if node.content_started {
                return Err(Error::Schema("attribute emitted after child".into()));
            }
            node.attributes.push(PendingAttribute {
                name: name.to_owned(),
                bytes: value.bytes.to_vec(),
            });
            return Ok(());
        }

        self.start_content()?;
        self.schema.push(STRING_TYPE);
        write_name(
            &mut self.schema,
            name,
            self.options.names,
            self.options.encoding,
        )?;
        self.data.variable(&value.bytes, true)?;
        self.schema.push(NODE_END);
        Ok(())
    }

    fn start_content(&mut self) -> Result<()> {
        if self.nodes.is_empty() {
            return Ok(());
        }
        self.flush_attributes()?;
        self.nodes.last_mut().unwrap().content_started = true;
        Ok(())
    }

    fn flush_attributes(&mut self) -> Result<()> {
        let Some(node) = self.nodes.last_mut() else {
            return Ok(());
        };
        node.attributes
            .sort_unstable_by(|left, right| left.name.cmp(&right.name));
        let attributes = std::mem::take(&mut node.attributes);
        for attribute in attributes {
            self.schema.push(ATTRIBUTE_TYPE);
            write_name(
                &mut self.schema,
                &attribute.name,
                self.options.names,
                self.options.encoding,
            )?;
            self.data.variable(&attribute.bytes, true)?;
        }
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<Vec<u8>> {
        if !self.nodes.is_empty() {
            return Err(Error::Schema("unclosed node".into()));
        }
        self.schema.push(FILE_END);
        while self.schema.len() % 4 != 0 {
            self.schema.push(0);
        }
        let schema_len = u32::try_from(self.schema.len()).map_err(|_| Error::Limit)?;
        let data_len = u32::try_from(self.data.buf.len()).map_err(|_| Error::Limit)?;
        let capacity = 12usize
            .checked_add(self.schema.len())
            .and_then(|value| value.checked_add(self.data.buf.len()))
            .ok_or(Error::Limit)?;
        let mut out = Vec::with_capacity(capacity);
        out.extend_from_slice(&[
            MAGIC,
            match self.options.names {
                NameMode::Packed => PACKED_DATA,
                NameMode::Full => FULL_DATA,
            },
            self.options.encoding,
            !self.options.encoding,
        ]);
        out.extend_from_slice(&schema_len.to_be_bytes());
        out.extend_from_slice(&self.schema);
        out.extend_from_slice(&data_len.to_be_bytes());
        out.extend_from_slice(&self.data.buf);
        Ok(out)
    }
}

pub(crate) fn encode_fault(
    class: &str,
    status: i32,
    fault: &str,
    expire: Option<i32>,
    options: EncodeOptions,
) -> Result<Vec<u8>> {
    let mut writer = BinaryWriter::new(options);
    writer.begin_node("response")?;
    writer.begin_node(class)?;
    if let Some(expire) = expire {
        writer.scalar("expire", FieldKind::Attribute, 0x06, ScalarRef::I32(expire))?;
    }
    writer.scalar("fault", FieldKind::Attribute, 0x0b, ScalarRef::Str(fault))?;
    writer.scalar("status", FieldKind::Attribute, 0x06, ScalarRef::I32(status))?;
    writer.end_node()?;
    writer.end_node()?;
    writer.finish()
}

pub(crate) struct BinaryReader<'a> {
    input: &'a [u8],
    schema_pos: usize,
    schema_end: usize,
    data: DataReader<'a>,
    names: NameMode,
    current: Option<NodeHeader>,
    options: DecodeOptions,
    nodes: usize,
    depth: usize,
    method: Option<String>,
    capture_method: bool,
    file_end_seen: bool,
}

#[derive(Clone, Debug)]
struct NodeHeader {
    type_id: u8,
    name: String,
    is_array: bool,
}

impl<'a> BinaryReader<'a> {
    fn new(input: &'a [u8], options: DecodeOptions) -> Result<Self> {
        if input.len() < 12 || input[0] != MAGIC {
            return Err(Error::Header("bad magic"));
        }
        let names = match input[1] {
            PACKED_DATA => NameMode::Packed,
            FULL_DATA => NameMode::Full,
            _ => return Err(Error::Header("unsupported content byte")),
        };
        if input[3] != !input[2] {
            return Err(Error::Header("encoding complement mismatch"));
        }
        let schema_len = read_u32(input, 4)? as usize;
        let schema_end = 8usize.checked_add(schema_len).ok_or(Error::Limit)?;
        if schema_end + 4 > input.len() {
            return Err(Error::Header("schema length out of bounds"));
        }
        let data_len = read_u32(input, schema_end)? as usize;
        let data_start = schema_end + 4;
        if data_len > options.max_data || data_start + data_len != input.len() {
            return Err(Error::Header("data length mismatch"));
        }
        Ok(Self {
            input,
            schema_pos: 8,
            schema_end,
            data: DataReader::new(&input[data_start..], input[2]),
            names,
            current: None,
            options,
            nodes: 0,
            depth: 0,
            method: None,
            capture_method: false,
            file_end_seen: false,
        })
    }

    fn decode_root<T: Kbin>(&mut self) -> Result<T> {
        let root = self
            .next_header()?
            .ok_or_else(|| Error::Schema("missing root".into()))?;
        if root.type_id != NODE_TYPE || root.is_array || root.name != T::SCHEMA.node {
            return Err(Error::Schema(format!(
                "expected root {}, got {}",
                T::SCHEMA.node,
                root.name
            )));
        }
        let value = self.decode_body::<T>()?;
        if self.next_header()?.is_some() || !self.file_end_seen {
            return Err(Error::Schema("missing file terminator".into()));
        }
        if !self.data.is_fully_consumed() {
            return Err(Error::Schema("unconsumed data section".into()));
        }
        Ok(value)
    }

    fn decode_body<T: Kbin>(&mut self) -> Result<T> {
        self.depth += 1;
        if self.depth > self.options.max_depth {
            return Err(Error::Limit);
        }
        let mut builder = T::Builder::default();
        loop {
            let Some(header) = self.next_header()? else {
                return Err(Error::Schema("unexpected schema end".into()));
            };
            if header.type_id == (NODE_END & TYPE_ID_MASK) {
                self.depth -= 1;
                return T::finish(builder);
            }
            if self.capture_method && header.type_id == ATTRIBUTE_TYPE && header.name == "method" {
                self.current = Some(header);
                self.method = Some(self.data.variable_string()?);
                self.current = None;
                continue;
            }
            let kind = if header.type_id == ATTRIBUTE_TYPE {
                FieldKind::Attribute
            } else {
                FieldKind::Node
            };
            let field = T::SCHEMA.resolve(&header.name, kind);
            let leaf_node = kind == FieldKind::Node && header.type_id != NODE_TYPE;
            self.current = Some(header);
            if let Some(field) = field {
                T::decode_field(
                    &mut builder,
                    field.id,
                    &mut Decoder {
                        inner: DecoderInner::Binary(self),
                    },
                )?;
            } else {
                self.skip_current()?;
            }
            self.current = None;
            if leaf_node && field.is_some() {
                self.expect_node_end()?;
            }
        }
    }

    pub(crate) fn decode_current_nested<T: Kbin>(&mut self) -> Result<T> {
        let header = self
            .current
            .as_ref()
            .ok_or_else(|| Error::Schema("no current node".into()))?;
        if header.type_id != NODE_TYPE {
            return Err(Error::Schema("expected nested node".into()));
        }
        self.decode_body::<T>()
    }

    pub(crate) fn read_scalar(&mut self, expected: u8) -> Result<OwnedScalar> {
        let header = self
            .current
            .as_ref()
            .ok_or_else(|| Error::Schema("no current value".into()))?;
        if header.is_array {
            return Err(Error::Schema("expected scalar, got array".into()));
        }
        if header.type_id == ATTRIBUTE_TYPE {
            let s = self.data.variable_string()?;
            return parse_text_scalar(expected, &s);
        }
        if header.type_id != expected {
            return Err(Error::Schema(format!(
                "expected type {expected:#x}, got {:#x}",
                header.type_id
            )));
        }
        self.data.scalar(expected)
    }

    pub(crate) fn read_array<T: Primitive>(&mut self) -> Result<Vec<T>> {
        let header = self
            .current
            .as_ref()
            .ok_or_else(|| Error::Schema("no current array".into()))?;
        if !header.is_array || header.type_id != T::TYPE_ID {
            return Err(Error::Schema("array type mismatch".into()));
        }
        self.data.primitive_array::<T>()
    }

    pub(crate) fn read_fixed<T: Primitive, const N: usize>(
        &mut self,
        type_id: u8,
    ) -> Result<[T; N]> {
        let header = self
            .current
            .as_ref()
            .ok_or_else(|| Error::Schema("no current fixed vector".into()))?;
        if header.is_array || header.type_id != type_id {
            return Err(Error::Schema("fixed-vector type mismatch".into()));
        }
        let raw = self.data.bytes(N * T::SIZE)?;
        let values = raw
            .chunks_exact(T::SIZE)
            .map(T::read_be)
            .collect::<Result<Vec<_>>>()?;
        values
            .try_into()
            .map_err(|_| Error::Schema("fixed-vector length mismatch".into()))
    }

    pub(crate) fn read_wire_string(&mut self) -> Result<WireString> {
        let header = self
            .current
            .as_ref()
            .ok_or_else(|| Error::Schema("no current WireString".into()))?;
        if header.is_array || !matches!(header.type_id, STRING_TYPE | ATTRIBUTE_TYPE) {
            return Err(Error::Schema("WireString type mismatch".into()));
        }
        let raw = self.data.variable()?;
        let raw = raw.strip_suffix(&[0]).unwrap_or(raw);
        Ok(WireString {
            bytes: Bytes::copy_from_slice(raw),
            encoding: self.data.encoding,
        })
    }

    fn skip_current(&mut self) -> Result<()> {
        let current = self.current.as_ref().unwrap();
        let type_id = current.type_id;
        let is_array = current.is_array;
        if type_id == NODE_TYPE {
            loop {
                let h = self
                    .next_header()?
                    .ok_or_else(|| Error::Schema("unterminated unknown node".into()))?;
                if h.type_id == (NODE_END & TYPE_ID_MASK) {
                    break;
                }
                self.current = Some(h);
                self.skip_current()?;
            }
            Ok(())
        } else if is_array || matches!(type_id, BINARY_TYPE | STRING_TYPE | ATTRIBUTE_TYPE) {
            self.data.variable().map(drop)?;
            if type_id != ATTRIBUTE_TYPE {
                self.expect_node_end()?;
            }
            Ok(())
        } else {
            self.data.bytes(type_size(type_id)?).map(drop)?;
            self.expect_node_end()
        }
    }

    fn expect_node_end(&mut self) -> Result<()> {
        match self.next_header()? {
            Some(header) if header.type_id == (NODE_END & TYPE_ID_MASK) => Ok(()),
            _ => Err(Error::Schema("missing node terminator".into())),
        }
    }

    fn next_header(&mut self) -> Result<Option<NodeHeader>> {
        while self.schema_pos < self.schema_end && self.input[self.schema_pos] == 0 {
            self.schema_pos += 1;
        }
        if self.schema_pos >= self.schema_end {
            return Ok(None);
        }
        let raw = self.input[self.schema_pos];
        self.schema_pos += 1;
        let type_id = raw & TYPE_ID_MASK;
        let is_array = raw & ARRAY_FLAG != 0;
        if raw == FILE_END {
            self.file_end_seen = true;
            return Ok(None);
        }
        if raw == NODE_END {
            return Ok(Some(NodeHeader {
                type_id,
                name: String::new(),
                is_array,
            }));
        }
        self.nodes += 1;
        if self.nodes > self.options.max_nodes {
            return Err(Error::Limit);
        }
        let name = read_name(
            self.input,
            &mut self.schema_pos,
            self.schema_end,
            self.names,
            self.data.encoding,
        )?;
        Ok(Some(NodeHeader {
            type_id,
            name,
            is_array,
        }))
    }
}

fn scalar_text(value: ScalarRef<'_>) -> String {
    match value {
        ScalarRef::I8(v) => v.to_string(),
        ScalarRef::U8(v) => v.to_string(),
        ScalarRef::I16(v) => v.to_string(),
        ScalarRef::U16(v) => v.to_string(),
        ScalarRef::I32(v) => v.to_string(),
        ScalarRef::U32(v) => v.to_string(),
        ScalarRef::I64(v) => v.to_string(),
        ScalarRef::U64(v) => v.to_string(),
        ScalarRef::F32(v) => format!("{v:.6}"),
        ScalarRef::F64(v) => format!("{v:.6}"),
        ScalarRef::Bool(v) => u8::from(v).to_string(),
        ScalarRef::Str(v) => v.to_owned(),
        ScalarRef::Binary(v) => v.iter().map(|b| format!("{b:02x}")).collect(),
        ScalarRef::Ip4(v) => format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3]),
    }
}

pub(crate) fn parse_text_scalar(expected: u8, text: &str) -> Result<OwnedScalar> {
    let bad = || Error::Value {
        field: "value".into(),
        reason: format!("cannot parse {text:?}"),
    };
    Ok(match expected {
        0x02 => OwnedScalar::I8(parse_signed(text).map_err(|_| bad())?),
        0x03 => OwnedScalar::U8(parse_unsigned(text).map_err(|_| bad())?),
        0x04 => OwnedScalar::I16(parse_signed(text).map_err(|_| bad())?),
        0x05 => OwnedScalar::U16(parse_unsigned(text).map_err(|_| bad())?),
        0x06 => OwnedScalar::I32(parse_signed(text).map_err(|_| bad())?),
        0x07 | 0x0d => OwnedScalar::U32(parse_unsigned(text).map_err(|_| bad())?),
        0x08 => OwnedScalar::I64(parse_signed(text).map_err(|_| bad())?),
        0x09 => OwnedScalar::U64(parse_unsigned(text).map_err(|_| bad())?),
        0x0a => {
            let compact: String = text
                .chars()
                .filter(|ch| !ch.is_ascii_whitespace())
                .collect();
            let mut out = Vec::with_capacity(compact.len() / 2);
            for pair in compact.as_bytes().chunks_exact(2) {
                let pair = std::str::from_utf8(pair).map_err(|_| bad())?;
                out.push(u8::from_str_radix(pair, 16).map_err(|_| bad())?);
            }
            OwnedScalar::Binary(Bytes::from(out))
        }
        0x0b => OwnedScalar::Str(text.to_owned()),
        0x0c => {
            let v: Vec<u8> = text
                .split('.')
                .map(str::parse)
                .collect::<core::result::Result<_, _>>()
                .map_err(|_| bad())?;
            OwnedScalar::Ip4(v.try_into().map_err(|_| bad())?)
        }
        0x0e => OwnedScalar::F32(parse_float(text).map_err(|_| bad())? as f32),
        0x0f => OwnedScalar::F64(parse_float(text).map_err(|_| bad())?),
        0x34 => OwnedScalar::Bool(match text.trim().trim_start_matches('0') {
            "" => false,
            "1" => true,
            _ => return Err(bad()),
        }),
        _ => return Err(Error::Unsupported("text scalar type")),
    })
}

fn parse_signed<T>(text: &str) -> core::result::Result<T, ()>
where
    T: TryFrom<i128>,
{
    let text = text.trim();
    if text.starts_with('+') {
        return Err(());
    }
    let text = if text.is_empty() { "0" } else { text };
    let (negative, digits) = text.strip_prefix('-').map_or((false, text), |v| (true, v));
    let (radix, digits) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .map_or((10, digits), |v| (16, v));
    if digits.is_empty() {
        return Err(());
    }
    let magnitude = i128::from_str_radix(digits, radix).map_err(|_| ())?;
    let value = if negative {
        magnitude.checked_neg().ok_or(())?
    } else {
        magnitude
    };
    T::try_from(value).map_err(|_| ())
}

fn parse_unsigned<T>(text: &str) -> core::result::Result<T, ()>
where
    T: TryFrom<u128>,
{
    let text = text.trim();
    if text.starts_with(['+', '-']) {
        return Err(());
    }
    let text = if text.is_empty() { "0" } else { text };
    let (radix, digits) = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .map_or((10, text), |v| (16, v));
    if digits.is_empty() {
        return Err(());
    }
    let value = u128::from_str_radix(digits, radix).map_err(|_| ())?;
    T::try_from(value).map_err(|_| ())
}

fn parse_float(text: &str) -> core::result::Result<f64, ()> {
    match text.trim() {
        "" => Ok(0.0),
        "(+INF)" => Ok(f64::INFINITY),
        "(-INF)" => Ok(f64::NEG_INFINITY),
        "(QNaN)" => Ok(f64::NAN),
        value
            if value.eq_ignore_ascii_case("nan") || value.to_ascii_lowercase().contains("inf") =>
        {
            Err(())
        }
        value => value.parse().map_err(|_| ()),
    }
}

fn read_u32(input: &[u8], at: usize) -> Result<u32> {
    input
        .get(at..at + 4)
        .ok_or(Error::Header("truncated integer"))
        .map(|v| u32::from_be_bytes(v.try_into().unwrap()))
}

fn type_size(id: u8) -> Result<usize> {
    Ok(match id {
        NODE_TYPE => 0,
        0x02 | 0x03 | 0x34 => 1,
        0x04 | 0x05 | 0x10 | 0x11 | 0x35 => 2,
        0x06 | 0x07 | 0x0c | 0x0d | 0x0e | 0x12 | 0x13 | 0x24 | 0x25 | 0x37 => 4,
        0x1c | 0x1d => 6,
        0x08 | 0x09 | 0x0f | 0x14 | 0x15 | 0x18 | 0x26 | 0x27 => 8,
        0x1a | 0x1b | 0x36 => 3,
        0x1e | 0x1f | 0x22 => 12,
        0x16 | 0x17 | 0x19 | 0x28 | 0x29 | 0x2c | 0x30 | 0x31 | 0x32 | 0x33 | 0x38 => 16,
        0x20 | 0x21 | 0x23 => 24,
        0x2a | 0x2b | 0x2d => 32,
        _ => return Err(Error::Unsupported("unknown kbin type")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kbin, schema::Schema};

    #[derive(Kbin)]
    #[kbin(node = "services")]
    struct ServicesAttributes {
        #[kbin(attr)]
        expire: i32,
        #[kbin(attr)]
        fault: String,
        #[kbin(attr)]
        mode: String,
        #[kbin(attr)]
        product_domain: u8,
    }

    struct StatusLayer {
        status: i32,
        payload: ServicesAttributes,
    }

    impl Kbin for StatusLayer {
        type Builder = ();
        const SCHEMA: &'static Schema = ServicesAttributes::SCHEMA;

        fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
            encoder.field("status", FieldKind::Attribute, &self.status)?;
            self.payload.encode(encoder)
        }

        fn decode_field(
            _builder: &mut Self::Builder,
            _id: u16,
            _decoder: &mut Decoder<'_, '_>,
        ) -> Result<()> {
            Err(Error::Schema("encode-only test type".into()))
        }

        fn finish(_builder: Self::Builder) -> Result<Self> {
            Err(Error::Schema("encode-only test type".into()))
        }
    }

    #[test]
    fn layered_attributes_sort_names_and_values_together() {
        let packet = encode_kbin(
            &StatusLayer {
                status: 0,
                payload: ServicesAttributes {
                    expire: 10_800,
                    fault: "0".into(),
                    mode: "operation".into(),
                    product_domain: 1,
                },
            },
            EncodeOptions::default(),
        )
        .unwrap();
        let mut reader = BinaryReader::new(&packet, DecodeOptions::default()).unwrap();
        let root = reader.next_header().unwrap().unwrap();
        assert_eq!(root.name, "services");

        let mut attributes = Vec::new();
        loop {
            let header = reader.next_header().unwrap().unwrap();
            if header.type_id == (NODE_END & TYPE_ID_MASK) {
                break;
            }
            assert_eq!(header.type_id, ATTRIBUTE_TYPE);
            attributes.push((header.name, reader.data.variable_string().unwrap()));
        }
        assert_eq!(
            attributes,
            [
                ("expire".into(), "10800".into()),
                ("fault".into(), "0".into()),
                ("mode".into(), "operation".into()),
                ("product_domain".into(), "1".into()),
                ("status".into(), "0".into()),
            ]
        );
    }
}
