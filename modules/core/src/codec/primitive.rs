use crate::{Error, Result};

#[doc(hidden)]
pub trait Primitive: Sized + Send + Default + 'static {
    const TYPE_ID: u8;
    const SIZE: usize;
    fn append_be(&self, out: &mut Vec<u8>);
    fn read_be(bytes: &[u8]) -> Result<Self>;
    fn text(&self) -> String;
    fn parse(text: &str) -> Result<Self>;
}

macro_rules! primitive_int {
    ($ty:ty, $id:expr) => {
        impl Primitive for $ty {
            const TYPE_ID: u8 = $id;
            const SIZE: usize = core::mem::size_of::<Self>();
            fn append_be(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }
            fn read_be(bytes: &[u8]) -> Result<Self> {
                Ok(<$ty>::from_be_bytes(bytes.try_into().map_err(|_| {
                    Error::Schema("bad primitive width".into())
                })?))
            }
            fn text(&self) -> String {
                self.to_string()
            }
            fn parse(text: &str) -> Result<Self> {
                text.parse().map_err(|_| Error::Value {
                    field: "array".into(),
                    reason: format!("invalid value {text:?}"),
                })
            }
        }
    };
}

primitive_int!(i8, 0x02);
primitive_int!(u8, 0x03);
primitive_int!(i16, 0x04);
primitive_int!(u16, 0x05);
primitive_int!(i32, 0x06);
primitive_int!(u32, 0x07);
primitive_int!(i64, 0x08);
primitive_int!(u64, 0x09);

impl Primitive for f32 {
    const TYPE_ID: u8 = 0x0e;
    const SIZE: usize = 4;
    fn append_be(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_bits().to_be_bytes());
    }
    fn read_be(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_bits(u32::from_be_bytes(
            bytes
                .try_into()
                .map_err(|_| Error::Schema("bad f32 width".into()))?,
        )))
    }
    fn text(&self) -> String {
        format!("{self:.6}")
    }
    fn parse(text: &str) -> Result<Self> {
        text.parse().map_err(|_| Error::Value {
            field: "array".into(),
            reason: "invalid f32".into(),
        })
    }
}

impl Primitive for f64 {
    const TYPE_ID: u8 = 0x0f;
    const SIZE: usize = 8;
    fn append_be(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_bits().to_be_bytes());
    }
    fn read_be(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_bits(u64::from_be_bytes(
            bytes
                .try_into()
                .map_err(|_| Error::Schema("bad f64 width".into()))?,
        )))
    }
    fn text(&self) -> String {
        format!("{self:.6}")
    }
    fn parse(text: &str) -> Result<Self> {
        text.parse().map_err(|_| Error::Value {
            field: "array".into(),
            reason: "invalid f64".into(),
        })
    }
}

impl Primitive for bool {
    const TYPE_ID: u8 = 0x34;
    const SIZE: usize = 1;
    fn append_be(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
    fn read_be(bytes: &[u8]) -> Result<Self> {
        Ok(bytes
            .first()
            .copied()
            .ok_or_else(|| Error::Schema("bad bool width".into()))?
            != 0)
    }
    fn text(&self) -> String {
        u8::from(*self).to_string()
    }
    fn parse(text: &str) -> Result<Self> {
        match text {
            "0" | "false" => Ok(false),
            "1" | "true" => Ok(true),
            _ => Err(Error::Value {
                field: "array".into(),
                reason: "invalid bool".into(),
            }),
        }
    }
}
