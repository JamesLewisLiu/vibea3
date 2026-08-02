use bytes::Bytes;

use crate::{
    Result,
    error::Error,
    schema::{FieldKind, Kbin},
    value::{Binary, Ip4, Timestamp, WireString},
};

use super::Primitive;
use super::{binary::BinaryReader, binary::BinaryWriter, text::XmlReader, text::XmlWriter};

#[derive(Clone, Copy, Debug)]
pub(crate) enum ScalarRef<'a> {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Bool(bool),
    Str(&'a str),
    Binary(&'a [u8]),
    Ip4([u8; 4]),
}

pub struct Encoder<'a> {
    pub(crate) inner: EncoderInner<'a>,
}

pub(crate) enum EncoderInner<'a> {
    Binary(&'a mut BinaryWriter),
    Xml(&'a mut XmlWriter),
}

impl Encoder<'_> {
    pub fn field<T: WireEncode + ?Sized>(
        &mut self,
        name: &'static str,
        kind: FieldKind,
        value: &T,
    ) -> Result<()> {
        value.wire_encode(self, name, kind)
    }

    pub fn array<T: Primitive>(
        &mut self,
        name: &'static str,
        kind: FieldKind,
        values: &[T],
    ) -> Result<()> {
        if kind == FieldKind::Attribute {
            return Err(Error::Unsupported("array attributes"));
        }
        match &mut self.inner {
            EncoderInner::Binary(writer) => writer.array(name, values),
            EncoderInner::Xml(writer) => writer.array(name, values),
        }
    }

    pub fn fixed<T: Primitive, const N: usize>(
        &mut self,
        name: &'static str,
        kind: FieldKind,
        type_id: u8,
        values: &[T; N],
    ) -> Result<()> {
        if kind == FieldKind::Attribute {
            return Err(Error::Unsupported("fixed-vector attributes"));
        }
        match &mut self.inner {
            EncoderInner::Binary(writer) => writer.fixed(name, type_id, values),
            EncoderInner::Xml(writer) => writer.fixed(name, type_id, values),
        }
    }

    pub(crate) fn scalar(
        &mut self,
        name: &'static str,
        kind: FieldKind,
        type_id: u8,
        value: ScalarRef<'_>,
    ) -> Result<()> {
        match &mut self.inner {
            EncoderInner::Binary(writer) => writer.scalar(name, kind, type_id, value),
            EncoderInner::Xml(writer) => writer.scalar(name, kind, type_id, value),
        }
    }

    pub(crate) fn nested<T: Kbin>(&mut self, name: &'static str, value: &T) -> Result<()> {
        match &mut self.inner {
            EncoderInner::Binary(writer) => {
                writer.begin_node(name)?;
                value.encode(&mut Encoder {
                    inner: EncoderInner::Binary(writer),
                })?;
                writer.end_node()
            }
            EncoderInner::Xml(writer) => {
                writer.begin_node(name)?;
                value.encode(&mut Encoder {
                    inner: EncoderInner::Xml(writer),
                })?;
                writer.end_node()
            }
        }
    }

    pub(crate) fn wire_string(
        &mut self,
        name: &'static str,
        kind: FieldKind,
        value: &WireString,
    ) -> Result<()> {
        match &mut self.inner {
            EncoderInner::Binary(writer) => writer.wire_string(name, kind, value),
            EncoderInner::Xml(writer) => writer.wire_string(name, kind, value),
        }
    }
}

pub struct Decoder<'a, 'input> {
    pub(crate) inner: DecoderInner<'a, 'input>,
}

pub(crate) enum DecoderInner<'a, 'input> {
    Binary(&'a mut BinaryReader<'input>),
    Xml(&'a mut XmlReader<'input>),
}

impl Decoder<'_, '_> {
    pub fn value<T: WireDecode>(&mut self) -> Result<T> {
        T::wire_decode(self)
    }

    pub fn array<T: Primitive>(&mut self) -> Result<Vec<T>> {
        match &mut self.inner {
            DecoderInner::Binary(reader) => reader.read_array::<T>(),
            DecoderInner::Xml(reader) => reader.read_array::<T>(),
        }
    }

    pub fn fixed<T: Primitive, const N: usize>(&mut self, type_id: u8) -> Result<[T; N]> {
        match &mut self.inner {
            DecoderInner::Binary(reader) => reader.read_fixed::<T, N>(type_id),
            DecoderInner::Xml(reader) => reader.read_fixed::<T, N>(type_id),
        }
    }

    pub(crate) fn scalar(&mut self, expected: u8) -> Result<OwnedScalar> {
        match &mut self.inner {
            DecoderInner::Binary(reader) => reader.read_scalar(expected),
            DecoderInner::Xml(reader) => reader.read_scalar(expected),
        }
    }

    pub(crate) fn nested<T: Kbin>(&mut self) -> Result<T> {
        match &mut self.inner {
            DecoderInner::Binary(reader) => reader.decode_current_nested::<T>(),
            DecoderInner::Xml(reader) => reader.decode_current_nested::<T>(),
        }
    }

    pub(crate) fn wire_string(&mut self) -> Result<WireString> {
        match &mut self.inner {
            DecoderInner::Binary(reader) => reader.read_wire_string(),
            DecoderInner::Xml(reader) => reader.read_wire_string(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum OwnedScalar {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Bool(bool),
    Str(String),
    Binary(Bytes),
    Ip4([u8; 4]),
}

pub trait WireEncode {
    fn wire_encode(
        &self,
        encoder: &mut Encoder<'_>,
        name: &'static str,
        kind: FieldKind,
    ) -> Result<()>;
}

pub trait WireDecode: Sized {
    fn wire_decode(decoder: &mut Decoder<'_, '_>) -> Result<Self>;
}

macro_rules! scalar_impl {
    ($ty:ty, $id:expr, $variant:ident) => {
        impl WireEncode for $ty {
            fn wire_encode(
                &self,
                e: &mut Encoder<'_>,
                n: &'static str,
                k: FieldKind,
            ) -> Result<()> {
                e.scalar(n, k, $id, ScalarRef::$variant(*self))
            }
        }
        impl WireDecode for $ty {
            fn wire_decode(d: &mut Decoder<'_, '_>) -> Result<Self> {
                match d.scalar($id)? {
                    OwnedScalar::$variant(v) => Ok(v),
                    _ => Err(Error::Schema(format!(
                        "wire type mismatch for {}",
                        stringify!($ty)
                    ))),
                }
            }
        }
    };
}

scalar_impl!(i8, 0x02, I8);
scalar_impl!(u8, 0x03, U8);
scalar_impl!(i16, 0x04, I16);
scalar_impl!(u16, 0x05, U16);
scalar_impl!(i32, 0x06, I32);
scalar_impl!(u32, 0x07, U32);
scalar_impl!(i64, 0x08, I64);
scalar_impl!(u64, 0x09, U64);
scalar_impl!(f32, 0x0e, F32);
scalar_impl!(f64, 0x0f, F64);
scalar_impl!(bool, 0x34, Bool);

impl WireEncode for str {
    fn wire_encode(&self, e: &mut Encoder<'_>, n: &'static str, k: FieldKind) -> Result<()> {
        e.scalar(n, k, 0x0b, ScalarRef::Str(self))
    }
}

impl WireEncode for String {
    fn wire_encode(&self, e: &mut Encoder<'_>, n: &'static str, k: FieldKind) -> Result<()> {
        self.as_str().wire_encode(e, n, k)
    }
}

impl WireDecode for String {
    fn wire_decode(d: &mut Decoder<'_, '_>) -> Result<Self> {
        match d.scalar(0x0b)? {
            OwnedScalar::Str(v) => Ok(v),
            _ => Err(Error::Schema("wire type mismatch for String".into())),
        }
    }
}

impl WireEncode for Binary {
    fn wire_encode(&self, e: &mut Encoder<'_>, n: &'static str, k: FieldKind) -> Result<()> {
        e.scalar(n, k, 0x0a, ScalarRef::Binary(&self.0))
    }
}

impl WireDecode for Binary {
    fn wire_decode(d: &mut Decoder<'_, '_>) -> Result<Self> {
        match d.scalar(0x0a)? {
            OwnedScalar::Binary(v) => Ok(Self(v)),
            _ => Err(Error::Schema("wire type mismatch for Binary".into())),
        }
    }
}

impl WireEncode for Ip4 {
    fn wire_encode(&self, e: &mut Encoder<'_>, n: &'static str, k: FieldKind) -> Result<()> {
        e.scalar(n, k, 0x0c, ScalarRef::Ip4(self.0.octets()))
    }
}

impl WireDecode for Ip4 {
    fn wire_decode(d: &mut Decoder<'_, '_>) -> Result<Self> {
        match d.scalar(0x0c)? {
            OwnedScalar::Ip4(v) => Ok(Self(v.into())),
            _ => Err(Error::Schema("wire type mismatch for Ip4".into())),
        }
    }
}

impl WireEncode for Timestamp {
    fn wire_encode(&self, e: &mut Encoder<'_>, n: &'static str, k: FieldKind) -> Result<()> {
        e.scalar(n, k, 0x0d, ScalarRef::U32(self.0))
    }
}

impl WireDecode for Timestamp {
    fn wire_decode(d: &mut Decoder<'_, '_>) -> Result<Self> {
        match d.scalar(0x0d)? {
            OwnedScalar::U32(v) => Ok(Self(v)),
            _ => Err(Error::Schema("wire type mismatch for Timestamp".into())),
        }
    }
}

impl<T: Kbin> WireEncode for T {
    fn wire_encode(&self, e: &mut Encoder<'_>, n: &'static str, k: FieldKind) -> Result<()> {
        if k == FieldKind::Attribute {
            return Err(Error::Unsupported("nested attributes"));
        }
        e.nested(n, self)
    }
}

impl<T: Kbin> WireDecode for T {
    fn wire_decode(d: &mut Decoder<'_, '_>) -> Result<Self> {
        d.nested::<T>()
    }
}

impl WireEncode for WireString {
    fn wire_encode(&self, e: &mut Encoder<'_>, n: &'static str, k: FieldKind) -> Result<()> {
        e.wire_string(n, k, self)
    }
}

impl WireDecode for WireString {
    fn wire_decode(d: &mut Decoder<'_, '_>) -> Result<Self> {
        d.wire_string()
    }
}
