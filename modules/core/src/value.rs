use bytes::Bytes;
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Binary(pub Bytes);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ip4(pub Ipv4Addr);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timestamp(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireString {
    pub bytes: Bytes,
    pub encoding: u8,
}
