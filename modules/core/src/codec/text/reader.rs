use quick_xml::{
    Reader, XmlVersion,
    events::{BytesStart, Event},
};
use std::cell::Cell;

use crate::{
    Error, Result,
    schema::{FieldKind, Kbin},
    value::WireString,
};

use super::{
    super::{
        BINARY_TYPE, DecodeOptions, NODE_TYPE, Primitive, STRING_TYPE,
        binary::parse_text_scalar,
        types::{Decoder, DecoderInner, OwnedScalar},
    },
    shared::type_id,
};

#[derive(Debug)]
enum Current {
    Attribute(String),
    Element(Element),
}

#[derive(Debug)]
struct Element {
    name: String,
    type_id: u8,
    attrs: Vec<(String, String)>,
    empty: bool,
    is_array: bool,
    count: Option<usize>,
    size: Option<usize>,
}

pub(crate) struct XmlReader<'a> {
    reader: Reader<&'a [u8]>,
    current: Option<Current>,
    options: DecodeOptions,
    nodes: Cell<usize>,
    depth: usize,
    pub(crate) method: Option<String>,
    pub(crate) capture_method: bool,
}

impl<'a> XmlReader<'a> {
    pub(crate) fn new(input: &'a [u8], options: DecodeOptions) -> Result<Self> {
        let mut reader = Reader::from_reader(input);
        // Preserve leading/trailing whitespace around entity references. quick-xml
        // reports `foo &amp; bar` as separate text/reference/text events, so
        // trimming each event would silently turn it into `foo&bar`.
        reader.config_mut().trim_text(false);
        Ok(Self {
            reader,
            current: None,
            options,
            nodes: Cell::new(0),
            depth: 0,
            method: None,
            capture_method: false,
        })
    }

    pub(crate) fn decode_root<T: Kbin>(&mut self) -> Result<T> {
        let root = loop {
            match self.read_event()? {
                Some(EventOwned::Element(value)) => break value,
                Some(EventOwned::Other) => continue,
                Some(EventOwned::End) => return Err(Error::Xml("unexpected end".into())),
                None => return Err(Error::Xml("missing root".into())),
            }
        };
        if root.name != T::SCHEMA.node {
            return Err(Error::Schema(format!(
                "expected root {}, got {}",
                T::SCHEMA.node,
                root.name
            )));
        }
        self.current = Some(Current::Element(root));
        self.decode_current_nested::<T>()
    }

    fn decode_body<T: Kbin>(&mut self, element: Element) -> Result<T> {
        self.depth += 1;
        if self.depth > self.options.max_depth {
            return Err(Error::Limit);
        }
        let mut builder = T::Builder::default();
        for (name, value) in element.attrs {
            if name.starts_with("__") {
                continue;
            }
            if self.capture_method && name == "method" {
                self.method = Some(value);
                continue;
            }
            if let Some(field) = T::SCHEMA.resolve(&name, FieldKind::Attribute) {
                self.current = Some(Current::Attribute(value));
                T::decode_field(
                    &mut builder,
                    field.id,
                    &mut Decoder {
                        inner: DecoderInner::Xml(self),
                    },
                )?;
            }
        }
        if !element.empty {
            loop {
                match self.read_event()? {
                    Some(EventOwned::Element(child)) => {
                        let field = T::SCHEMA.resolve(&child.name, FieldKind::Node);
                        self.current = Some(Current::Element(child));
                        if let Some(field) = field {
                            T::decode_field(
                                &mut builder,
                                field.id,
                                &mut Decoder {
                                    inner: DecoderInner::Xml(self),
                                },
                            )?;
                        } else {
                            self.skip_current()?;
                        }
                    }
                    Some(EventOwned::End) => break,
                    Some(EventOwned::Other) => {}
                    None => return Err(Error::Xml("unterminated element".into())),
                }
            }
        }
        self.depth -= 1;
        T::finish(builder)
    }

    pub(crate) fn decode_current_nested<T: Kbin>(&mut self) -> Result<T> {
        match self.current.take() {
            Some(Current::Element(element)) if element.type_id == NODE_TYPE => {
                self.decode_body::<T>(element)
            }
            _ => Err(Error::Schema("expected nested node".into())),
        }
    }

    pub(crate) fn read_scalar(&mut self, expected: u8) -> Result<OwnedScalar> {
        match self.current.take() {
            Some(Current::Attribute(value)) => parse_text_scalar(expected, &value),
            Some(Current::Element(element))
                if !element.is_array
                    && (element.type_id == expected
                        || (expected == STRING_TYPE && element.type_id == NODE_TYPE)) =>
            {
                let value = if element.empty {
                    parse_text_scalar(expected, "")?
                } else {
                    parse_text_scalar(expected, &self.read_text("scalar")?)?
                };
                if expected == BINARY_TYPE {
                    resize_binary(value, element.size)
                } else {
                    Ok(value)
                }
            }
            _ => Err(Error::Schema("XML scalar type mismatch".into())),
        }
    }

    pub(crate) fn read_array<T: Primitive>(&mut self) -> Result<Vec<T>> {
        let Some(Current::Element(element)) = self.current.take() else {
            return Err(Error::Schema("no XML array".into()));
        };
        if !element.is_array || element.type_id != T::TYPE_ID {
            return Err(Error::Schema("XML array type mismatch".into()));
        }
        let mut values = if element.empty {
            Vec::new()
        } else {
            let text = self.read_text("array")?;
            if text.trim().is_empty() {
                Vec::new()
            } else {
                text.split_ascii_whitespace()
                    .map(T::parse)
                    .collect::<Result<Vec<_>>>()?
            }
        };
        let count = element
            .count
            .ok_or_else(|| Error::Schema("XML array has no __count".into()))?;
        if values.len() > count {
            return Err(Error::Schema("XML array has too many values".into()));
        }
        values.resize_with(count, T::default);
        Ok(values)
    }

    pub(crate) fn read_fixed<T: Primitive, const N: usize>(
        &mut self,
        type_id: u8,
    ) -> Result<[T; N]> {
        let Some(Current::Element(element)) = self.current.take() else {
            return Err(Error::Schema("no XML fixed vector".into()));
        };
        if element.is_array || element.type_id != type_id || element.empty {
            return Err(Error::Schema("XML fixed-vector mismatch".into()));
        }
        self.read_text("fixed vector")?
            .split_ascii_whitespace()
            .map(T::parse)
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .map_err(|_| Error::Schema("fixed-vector item count mismatch".into()))
    }

    pub(crate) fn read_wire_string(&mut self) -> Result<WireString> {
        let value = match self.read_scalar(STRING_TYPE)? {
            OwnedScalar::Str(value) => value,
            _ => return Err(Error::Schema("WireString type mismatch".into())),
        };
        Ok(WireString {
            bytes: value.into_bytes().into(),
            encoding: 0xa0,
        })
    }

    fn read_text(&mut self, kind: &str) -> Result<String> {
        let mut text = String::new();
        loop {
            match self
                .reader
                .read_event()
                .map_err(|e| Error::Xml(e.to_string()))?
            {
                Event::Text(value) => {
                    text.push_str(&value.decode().map_err(|e| Error::Xml(e.to_string()))?)
                }
                Event::CData(value) => text.push_str(&String::from_utf8_lossy(&value)),
                Event::GeneralRef(value) => {
                    let name = value.decode().map_err(|e| Error::Xml(e.to_string()))?;
                    text.push(resolve_reference(&name)?);
                }
                Event::End(_) => return Ok(text),
                Event::Comment(_) => {}
                Event::Eof => return Err(Error::Xml(format!("unterminated {kind}"))),
                other => {
                    return Err(Error::Xml(format!(
                        "{kind} contains unexpected XML event {other:?}"
                    )));
                }
            }
        }
    }

    fn skip_current(&mut self) -> Result<()> {
        let Some(Current::Element(element)) = self.current.take() else {
            return Ok(());
        };
        if element.empty {
            return Ok(());
        }
        let mut depth = 1;
        while depth != 0 {
            match self
                .reader
                .read_event()
                .map_err(|e| Error::Xml(e.to_string()))?
            {
                Event::Start(_) => depth += 1,
                Event::End(_) => depth -= 1,
                Event::Eof => return Err(Error::Xml("unterminated unknown element".into())),
                _ => {}
            }
        }
        Ok(())
    }

    fn read_event(&mut self) -> Result<Option<EventOwned>> {
        match self
            .reader
            .read_event()
            .map_err(|e| Error::Xml(e.to_string()))?
        {
            Event::Start(value) => Ok(Some(EventOwned::Element(self.element(value, false)?))),
            Event::Empty(value) => Ok(Some(EventOwned::Element(self.element(value, true)?))),
            Event::End(_) => Ok(Some(EventOwned::End)),
            Event::DocType(_) => Err(Error::Xml("DOCTYPE is not allowed".into())),
            Event::Eof => Ok(None),
            Event::Decl(_) | Event::Comment(_) | Event::Text(_) | Event::PI(_) => {
                Ok(Some(EventOwned::Other))
            }
            _ => self.read_event(),
        }
    }

    fn element(&self, start: BytesStart<'_>, empty: bool) -> Result<Element> {
        let nodes = self.nodes.get() + 1;
        if nodes > self.options.max_nodes {
            return Err(Error::Limit);
        }
        self.nodes.set(nodes);
        let name = String::from_utf8(start.name().as_ref().to_vec())
            .map_err(|_| Error::Encoding("XML name"))?;
        let mut attrs = Vec::new();
        let mut wire_type = NODE_TYPE;
        let mut is_array = false;
        let mut count = None;
        let mut size = None;
        for attr in start.attributes() {
            let attr = attr.map_err(|e| Error::Xml(e.to_string()))?;
            let key = String::from_utf8(attr.key.as_ref().to_vec())
                .map_err(|_| Error::Encoding("attribute name"))?;
            let value = attr
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, self.reader.decoder())
                .map_err(|e| Error::Xml(e.to_string()))?
                .into_owned();
            if key == "__type" {
                wire_type = type_id(&value)?;
            }
            if key == "__count" {
                is_array = true;
                count = Some(value.parse::<usize>().map_err(|_| Error::Value {
                    field: "__count".into(),
                    reason: format!("invalid value {value:?}"),
                })?);
            }
            if key == "__size" {
                size = Some(value.parse::<usize>().map_err(|_| Error::Value {
                    field: "__size".into(),
                    reason: format!("invalid value {value:?}"),
                })?);
            }
            attrs.push((key, value));
        }
        Ok(Element {
            name,
            type_id: wire_type,
            attrs,
            empty,
            is_array,
            count,
            size,
        })
    }
}

fn resize_binary(value: OwnedScalar, size: Option<usize>) -> Result<OwnedScalar> {
    let OwnedScalar::Binary(value) = value else {
        return Err(Error::Schema("expected binary scalar".into()));
    };
    let Some(size) = size else {
        return Ok(OwnedScalar::Binary(value));
    };
    if value.len() > size {
        return Err(Error::Schema("binary exceeds XML __size".into()));
    }
    let mut value = value.to_vec();
    value.resize(size, 0);
    Ok(OwnedScalar::Binary(value.into()))
}

enum EventOwned {
    Element(Element),
    End,
    Other,
}

fn resolve_reference(name: &str) -> Result<char> {
    match name {
        "amp" => Ok('&'),
        "lt" => Ok('<'),
        "gt" => Ok('>'),
        "quot" => Ok('"'),
        "apos" => Ok('\''),
        value if value.starts_with("#x") => u32::from_str_radix(&value[2..], 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| Error::Xml("invalid hexadecimal character reference".into())),
        value if value.starts_with('#') => value[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| Error::Xml("invalid decimal character reference".into())),
        _ => Err(Error::Xml("custom entities are not allowed".into())),
    }
}
