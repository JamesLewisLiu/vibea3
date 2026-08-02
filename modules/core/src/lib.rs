//! Typed codecs and RPC transport for Konami's e-Amusement protocol.

extern crate self as vibea3;

pub mod codec;
pub mod dynamic;
pub mod error;
pub mod host;
pub mod lz77;
pub mod module;
pub mod registry;
pub mod rpc;
pub mod schema;
pub mod transport;
pub mod value;

pub use codec::{
    DecodeOptions, EncodeOptions, NameMode, PacketFormat, decode_kbin, decode_xml, encode_kbin,
    encode_xml,
};
pub use dynamic::{
    DynamicOptions, DynamicRpcServer, DynamicServerBuilder, ModuleError, ModuleManager,
    ModuleStatus,
};
pub use error::{Error, Result};
pub use host::{
    DbFuture, DbResult, Document, DocumentDatabase, FindOneAndUpdateOptions, FindOneOptions,
    FindOptions, GeoLocation, HOST_API_FINGERPRINT, HostServices, IndexDefinition, ReturnDocument,
    ServiceConfig, Update, UpdateOptions,
};
pub use registry::ServiceEndpoint;
pub use rpc::{RpcContext, RpcError, RpcResponse, RpcResult, RpcServer};
pub use schema::{Kbin, Schema};
pub use value::{Binary, Ip4, Timestamp, WireString};
pub use vibea3_macros::{Kbin, export_xrpc_module, rpc, rpc_app};

#[doc(hidden)]
pub mod __private {
    pub use linkme::{self, distributed_slice};
}
