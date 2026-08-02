use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use std::io::Cursor;

use crate::{Error, Result, schema::FieldKind, value::WireString};

use super::{
    super::{
        EncodeOptions, Primitive,
        names::{decode_text, encode_text},
        types::ScalarRef,
    },
    shared::{encoding_label, scalar_text, type_name},
};

struct Pending {
    name: String,
    attrs: Vec<(String, String)>,
    opened: bool,
}

pub(crate) struct XmlWriter {
    writer: Writer<Cursor<Vec<u8>>>,
    stack: Vec<Pending>,
    encoding: u8,
}

impl XmlWriter {
    pub(crate) fn new(options: EncodeOptions) -> Result<Self> {
        let mut writer = Writer::new(Cursor::new(Vec::with_capacity(512)));
        writer
            .write_event(Event::Decl(BytesDecl::new(
                "1.0",
                encoding_label(options.encoding)?,
                None,
            )))
            .map_err(|e| Error::Xml(e.to_string()))?;
        Ok(Self {
            writer,
            stack: Vec::new(),
            encoding: options.encoding,
        })
    }

    pub(crate) fn begin_node(&mut self, name: &str) -> Result<()> {
        self.flush()?;
        self.stack.push(Pending {
            name: name.to_owned(),
            attrs: Vec::new(),
            opened: false,
        });
        Ok(())
    }

    pub(crate) fn end_node(&mut self) -> Result<()> {
        let Some(node) = self.stack.pop() else {
            return Err(Error::Xml("node stack underflow".into()));
        };
        if node.opened {
            self.writer
                .write_event(Event::End(BytesEnd::new(node.name)))
                .map_err(|e| Error::Xml(e.to_string()))?;
        } else {
            let mut start = BytesStart::new(&node.name);
            let mut attrs = node.attrs;
            attrs.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            for (key, value) in &attrs {
                start.push_attribute((key.as_str(), value.as_str()));
            }
            self.writer
                .write_event(Event::Start(start))
                .map_err(|e| Error::Xml(e.to_string()))?;
            self.writer
                .write_event(Event::End(BytesEnd::new(node.name)))
                .map_err(|e| Error::Xml(e.to_string()))?;
        }
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
            let current = self
                .stack
                .last_mut()
                .ok_or_else(|| Error::Xml("attribute outside node".into()))?;
            if current.opened {
                return Err(Error::Xml("attribute emitted after child".into()));
            }
            current.attrs.push((name.to_owned(), scalar_text(value)));
            return Ok(());
        }
        let text = scalar_text(value);
        self.value(name, type_id, None, &text)
    }

    pub(crate) fn array<T: Primitive>(&mut self, name: &str, values: &[T]) -> Result<()> {
        let text = values
            .iter()
            .map(Primitive::text)
            .collect::<Vec<_>>()
            .join(" ");
        self.value(name, T::TYPE_ID, Some(values.len()), &text)
    }

    pub(crate) fn fixed<T: Primitive, const N: usize>(
        &mut self,
        name: &str,
        type_id: u8,
        values: &[T; N],
    ) -> Result<()> {
        let text = values
            .iter()
            .map(Primitive::text)
            .collect::<Vec<_>>()
            .join(" ");
        self.value(name, type_id, None, &text)
    }

    pub(crate) fn wire_string(
        &mut self,
        name: &str,
        kind: FieldKind,
        value: &WireString,
    ) -> Result<()> {
        let text = decode_text(&value.bytes, value.encoding)?;
        self.scalar(name, kind, 0x0b, ScalarRef::Str(&text))
    }

    fn value(&mut self, name: &str, type_id: u8, count: Option<usize>, text: &str) -> Result<()> {
        self.flush()?;
        let mut start = BytesStart::new(name);
        start.push_attribute(("__type", type_name(type_id)?));
        let count = count.map(|v| v.to_string());
        if let Some(count) = &count {
            start.push_attribute(("__count", count.as_str()));
        }
        let size = (type_id == 0x0a).then(|| text.len().div_ceil(2).to_string());
        if let Some(size) = &size {
            start.push_attribute(("__size", size.as_str()));
        }
        self.writer
            .write_event(Event::Start(start))
            .map_err(|e| Error::Xml(e.to_string()))?;
        self.writer
            .write_event(Event::Text(BytesText::new(text)))
            .map_err(|e| Error::Xml(e.to_string()))?;
        self.writer
            .write_event(Event::End(BytesEnd::new(name)))
            .map_err(|e| Error::Xml(e.to_string()))?;
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let Some(node) = self.stack.last_mut() else {
            return Ok(());
        };
        if node.opened {
            return Ok(());
        }
        let mut start = BytesStart::new(&node.name);
        node.attrs
            .sort_unstable_by(|left, right| left.0.cmp(&right.0));
        for (key, value) in &node.attrs {
            start.push_attribute((key.as_str(), value.as_str()));
        }
        self.writer
            .write_event(Event::Start(start))
            .map_err(|e| Error::Xml(e.to_string()))?;
        node.opened = true;
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<Vec<u8>> {
        if !self.stack.is_empty() {
            return Err(Error::Xml("unclosed node".into()));
        }
        let utf8 = self.writer.into_inner().into_inner();
        if self.encoding == 0xa0 {
            return Ok(utf8);
        }
        encode_text(
            &String::from_utf8(utf8).map_err(|_| Error::Encoding("XML staging"))?,
            self.encoding,
        )
    }
}

pub(crate) fn encode_fault(
    class: &str,
    status: i32,
    fault: &str,
    expire: Option<i32>,
    options: EncodeOptions,
) -> Result<Vec<u8>> {
    let mut writer = XmlWriter::new(options)?;
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
