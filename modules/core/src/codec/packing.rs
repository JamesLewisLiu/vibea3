use bytes::Bytes;

use crate::{Error, Result};

use super::{
    Primitive,
    names::decode_text,
    types::{OwnedScalar, ScalarRef},
};

#[derive(Default)]
pub(crate) struct DataWriter {
    pub(crate) buf: Vec<u8>,
    word: usize,
    short: usize,
    byte: usize,
}

impl DataWriter {
    fn alloc(&mut self, size: usize) -> usize {
        match size {
            1 if self.byte % 4 == 0 => {
                let base = self.next_blocks(4);
                self.byte = base + 1;
                base
            }
            1 => {
                let at = self.byte;
                self.byte += 1;
                at
            }
            2 if self.short % 4 == 0 => {
                let base = self.next_blocks(4);
                self.short = base + 2;
                base
            }
            2 => {
                let at = self.short;
                self.short += 2;
                at
            }
            _ => self.next_blocks(size.div_ceil(4) * 4),
        }
    }

    fn next_blocks(&mut self, size: usize) -> usize {
        let at = self.word;
        self.word += size;
        if self.buf.len() < self.word {
            self.buf.resize(self.word, 0);
        }
        at
    }

    pub(crate) fn put(&mut self, bytes: &[u8]) {
        let at = self.alloc(bytes.len());
        self.buf[at..at + bytes.len()].copy_from_slice(bytes);
    }

    pub(crate) fn variable(&mut self, bytes: &[u8], nul: bool) -> Result<()> {
        let len = bytes.len() + usize::from(nul);
        let encoded_len = u32::try_from(len).map_err(|_| Error::Limit)?;
        let at = self.alloc(4 + len);
        self.buf[at..at + 4].copy_from_slice(&encoded_len.to_be_bytes());
        self.buf[at + 4..at + 4 + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn scalar(&mut self, value: ScalarRef<'_>) -> Result<()> {
        match value {
            ScalarRef::I8(v) => self.put(&v.to_be_bytes()),
            ScalarRef::U8(v) => self.put(&v.to_be_bytes()),
            ScalarRef::I16(v) => self.put(&v.to_be_bytes()),
            ScalarRef::U16(v) => self.put(&v.to_be_bytes()),
            ScalarRef::I32(v) => self.put(&v.to_be_bytes()),
            ScalarRef::U32(v) => self.put(&v.to_be_bytes()),
            ScalarRef::I64(v) => self.put(&v.to_be_bytes()),
            ScalarRef::U64(v) => self.put(&v.to_be_bytes()),
            ScalarRef::F32(v) => self.put(&v.to_bits().to_be_bytes()),
            ScalarRef::F64(v) => self.put(&v.to_bits().to_be_bytes()),
            ScalarRef::Bool(v) => self.put(&[u8::from(v)]),
            ScalarRef::Binary(v) => self.variable(v, false)?,
            ScalarRef::Ip4(v) => self.put(&v),
            ScalarRef::Str(_) => {
                return Err(Error::Schema("unencoded string reached packer".into()));
            }
        }
        Ok(())
    }
}

pub(crate) struct DataReader<'a> {
    buf: &'a [u8],
    word: usize,
    short: usize,
    byte: usize,
    pub(crate) encoding: u8,
}

impl<'a> DataReader<'a> {
    pub(crate) fn new(buf: &'a [u8], encoding: u8) -> Self {
        Self {
            buf,
            word: 0,
            short: 0,
            byte: 0,
            encoding,
        }
    }

    fn alloc(&mut self, size: usize) -> Result<usize> {
        let at = match size {
            1 if self.byte % 4 == 0 => {
                let x = self.next(4)?;
                self.byte = x + 1;
                x
            }
            1 => {
                let x = self.byte;
                self.byte += 1;
                x
            }
            2 if self.short % 4 == 0 => {
                let x = self.next(4)?;
                self.short = x + 2;
                x
            }
            2 => {
                let x = self.short;
                self.short += 2;
                x
            }
            _ => self.next(size.div_ceil(4) * 4)?,
        };
        if at + size > self.buf.len() {
            return Err(Error::Schema("data out of bounds".into()));
        }
        Ok(at)
    }

    fn next(&mut self, size: usize) -> Result<usize> {
        let at = self.word;
        self.word = self.word.checked_add(size).ok_or(Error::Limit)?;
        if self.word > self.buf.len() {
            return Err(Error::Schema("data block out of bounds".into()));
        }
        Ok(at)
    }

    pub(crate) fn bytes(&mut self, size: usize) -> Result<&'a [u8]> {
        let at = self.alloc(size)?;
        Ok(&self.buf[at..at + size])
    }

    pub(crate) fn variable(&mut self) -> Result<&'a [u8]> {
        let at = self.alloc(4)?;
        let len = u32::from_be_bytes(self.buf[at..at + 4].try_into().unwrap()) as usize;
        let data_at = at + 4;
        if data_at + len > self.buf.len() {
            return Err(Error::Schema("variable data out of bounds".into()));
        }
        self.word = self.word.max(at + (4 + len).div_ceil(4) * 4);
        Ok(&self.buf[data_at..data_at + len])
    }

    pub(crate) fn variable_string(&mut self) -> Result<String> {
        let raw = self.variable()?;
        decode_text(raw.strip_suffix(&[0]).unwrap_or(raw), self.encoding)
    }

    pub(crate) fn scalar(&mut self, id: u8) -> Result<OwnedScalar> {
        Ok(match id {
            0x02 => OwnedScalar::I8(i8::from_be_bytes(self.bytes(1)?.try_into().unwrap())),
            0x03 => OwnedScalar::U8(self.bytes(1)?[0]),
            0x04 => OwnedScalar::I16(i16::from_be_bytes(self.bytes(2)?.try_into().unwrap())),
            0x05 => OwnedScalar::U16(u16::from_be_bytes(self.bytes(2)?.try_into().unwrap())),
            0x06 => OwnedScalar::I32(i32::from_be_bytes(self.bytes(4)?.try_into().unwrap())),
            0x07 | 0x0d => OwnedScalar::U32(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap())),
            0x08 => OwnedScalar::I64(i64::from_be_bytes(self.bytes(8)?.try_into().unwrap())),
            0x09 => OwnedScalar::U64(u64::from_be_bytes(self.bytes(8)?.try_into().unwrap())),
            0x0a => OwnedScalar::Binary(Bytes::copy_from_slice(self.variable()?)),
            0x0b => OwnedScalar::Str(self.variable_string()?),
            0x0c => OwnedScalar::Ip4(self.bytes(4)?.try_into().unwrap()),
            0x0e => OwnedScalar::F32(f32::from_bits(u32::from_be_bytes(
                self.bytes(4)?.try_into().unwrap(),
            ))),
            0x0f => OwnedScalar::F64(f64::from_bits(u64::from_be_bytes(
                self.bytes(8)?.try_into().unwrap(),
            ))),
            0x34 => OwnedScalar::Bool(self.bytes(1)?[0] != 0),
            _ => return Err(Error::Unsupported("kbin type")),
        })
    }

    pub(crate) fn primitive_array<T: Primitive>(&mut self) -> Result<Vec<T>> {
        let raw = self.variable()?;
        if raw.len() % T::SIZE != 0 {
            return Err(Error::Schema("array byte length mismatch".into()));
        }
        raw.chunks_exact(T::SIZE).map(T::read_be).collect()
    }

    pub(crate) fn is_fully_consumed(&self) -> bool {
        self.word == self.buf.len()
    }
}
