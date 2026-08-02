use std::{future::Future, net::IpAddr, pin::Pin, sync::Arc};

pub use bson::{Bson, Document};

pub const HOST_API_FINGERPRINT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "-host@",
    env!("CARGO_PKG_VERSION"),
    "+document-api-v4"
);

pub type DbResult<T> = Result<T, String>;
pub type DbFuture<T> = Pin<Box<dyn Future<Output = DbResult<T>> + Send + 'static>>;

#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub instance_id: String,
    pub service_url: String,
    pub legacy_service_url: String,
    pub web_ui_url: String,
    pub ntp_url: String,
    pub keepalive_url: String,
    pub public_ip: [u8; 4],
    pub maintenance: bool,
    pub infinite_eacoin: bool,
}

#[derive(Clone, Debug)]
pub struct GeoLocation {
    pub country: Option<String>,
    pub country_name: Option<String>,
    pub country_jname: Option<String>,
    pub region: Option<String>,
    pub region_name: Option<String>,
    pub region_jname: Option<String>,
    pub city_name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct FindOneOptions {
    pub sort: Option<Document>,
}

#[derive(Clone, Debug, Default)]
pub struct FindOptions {
    pub sort: Option<Document>,
    pub limit: Option<u64>,
}

#[derive(Clone, Debug)]
pub enum Update {
    Document(Document),
    Pipeline(Vec<Document>),
}

impl From<Document> for Update {
    fn from(value: Document) -> Self {
        Self::Document(value)
    }
}

impl From<Vec<Document>> for Update {
    fn from(value: Vec<Document>) -> Self {
        Self::Pipeline(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReturnDocument {
    #[default]
    Before,
    After,
}

#[derive(Clone, Debug, Default)]
pub struct UpdateOptions {
    pub upsert: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FindOneAndUpdateOptions {
    pub upsert: bool,
    pub return_document: ReturnDocument,
}

#[derive(Clone, Debug)]
pub struct IndexDefinition {
    pub keys: Document,
    pub name: Option<String>,
    pub unique: bool,
    pub expire_after_seconds: Option<u64>,
}

pub trait DocumentDatabase: Send + Sync + 'static {
    fn create_indexes(&self, collection: String, indexes: Vec<IndexDefinition>) -> DbFuture<()>;

    fn drop_index(&self, collection: String, name: String) -> DbFuture<()> {
        let _ = (collection, name);
        Box::pin(async { Ok(()) })
    }

    fn find_one(
        &self,
        collection: String,
        filter: Document,
        options: FindOneOptions,
    ) -> DbFuture<Option<Document>>;

    fn find_many(
        &self,
        collection: String,
        filter: Document,
        options: FindOptions,
    ) -> DbFuture<Vec<Document>> {
        let _ = (collection, filter, options);
        Box::pin(async { Err("find_many is not supported by this document store".into()) })
    }

    fn update_one(
        &self,
        collection: String,
        filter: Document,
        update: Update,
        options: UpdateOptions,
    ) -> DbFuture<()>;

    fn find_one_and_update(
        &self,
        collection: String,
        filter: Document,
        update: Update,
        options: FindOneAndUpdateOptions,
    ) -> DbFuture<Option<Document>>;

    fn insert_one(&self, collection: String, document: Document) -> DbFuture<()>;

    fn insert_many(&self, collection: String, documents: Vec<Document>) -> DbFuture<()>;

    fn delete_one(&self, collection: String, filter: Document) -> DbFuture<()> {
        let _ = (collection, filter);
        Box::pin(async { Err("delete_one is not supported by this document store".into()) })
    }

    fn delete_many(&self, collection: String, filter: Document) -> DbFuture<()> {
        let _ = (collection, filter);
        Box::pin(async { Err("delete_many is not supported by this document store".into()) })
    }
}

pub trait HostServices: Send + Sync + 'static {
    fn config(&self) -> ServiceConfig;
    fn database(&self) -> Arc<dyn DocumentDatabase>;
    fn geolocate(&self, _address: IpAddr) -> DbFuture<Option<GeoLocation>> {
        Box::pin(async { Ok(None) })
    }
}
