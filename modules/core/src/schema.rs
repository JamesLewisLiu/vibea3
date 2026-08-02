use crate::{Result, codec::Decoder, codec::Encoder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Attribute,
    Node,
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub id: u16,
    pub name: &'static str,
    pub kind: FieldKind,
    pub optional: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Enter(&'static str),
    Field(u16),
    Exit,
}

#[derive(Debug)]
pub struct Schema {
    pub node: &'static str,
    pub fields: &'static [Field],
    pub ops: &'static [Op],
}

impl Schema {
    /// Resolves an incoming field by executing the derive-generated schema program.
    pub fn resolve(&self, name: &str, kind: FieldKind) -> Option<&Field> {
        self.ops.iter().find_map(|op| {
            let Op::Field(id) = op else {
                return None;
            };
            self.fields
                .iter()
                .find(|field| field.id == *id && field.name == name && field.kind == kind)
        })
    }
}

pub trait Kbin: Sized + Send + 'static {
    type Builder: Default;

    const SCHEMA: &'static Schema;

    fn rpc_status(&self) -> i32 {
        0
    }
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()>;
    fn decode_field(
        builder: &mut Self::Builder,
        id: u16,
        decoder: &mut Decoder<'_, '_>,
    ) -> Result<()>;
    fn finish(builder: Self::Builder) -> Result<Self>;
}
