use bson::doc;
use serde::{Deserialize, Serialize};

use super::{Database, MACHINES, decode};

impl Database {
    pub(crate) async fn machine(&self, pcb_id: String) -> Result<Option<MachineInfo>, String> {
        self.raw
            .find_one(MACHINES.into(), doc! { "_id": pcb_id }, Default::default())
            .await?
            .map(decode)
            .transpose()
            .map(|machine: Option<MachineDocument>| machine.map(Into::into))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MachineInfo {
    pub(crate) eacoin_enabled: bool,
    pub(crate) maintenance: bool,
    pub(crate) facility: FacilityInfo,
}

#[derive(Clone, Debug)]
pub(crate) struct FacilityInfo {
    pub(crate) id: String,
    pub(crate) country: String,
    pub(crate) region: String,
    pub(crate) name: String,
    pub(crate) country_name: String,
    pub(crate) country_jname: String,
    pub(crate) region_name: String,
    pub(crate) region_jname: String,
    pub(crate) port: u16,
    pub(crate) latitude: i32,
    pub(crate) longitude: i32,
    pub(crate) calendar_year: i16,
    pub(crate) holidays: Vec<i16>,
}

impl Default for FacilityInfo {
    fn default() -> Self {
        FacilityDocument::default().into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MachineDocument {
    #[serde(rename = "_id")]
    pcb_id: String,
    #[serde(default)]
    eacoin_enabled: bool,
    #[serde(default)]
    maintenance: bool,
    #[serde(default)]
    facility: FacilityDocument,
}

impl From<MachineDocument> for MachineInfo {
    fn from(value: MachineDocument) -> Self {
        Self {
            eacoin_enabled: value.eacoin_enabled,
            maintenance: value.maintenance,
            facility: value.facility.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FacilityDocument {
    #[serde(default = "default_id")]
    id: String,
    #[serde(default = "default_country")]
    country: String,
    #[serde(default = "default_region")]
    region: String,
    #[serde(default = "default_name")]
    name: String,
    #[serde(default = "default_country_name")]
    country_name: String,
    #[serde(default = "default_country_jname")]
    country_jname: String,
    #[serde(default = "default_region_name")]
    region_name: String,
    #[serde(default = "default_region_jname")]
    region_jname: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_latitude")]
    latitude: i32,
    #[serde(default = "default_longitude")]
    longitude: i32,
    #[serde(default = "default_year")]
    calendar_year: i16,
    #[serde(default)]
    holidays: Vec<i16>,
}

impl Default for FacilityDocument {
    fn default() -> Self {
        Self {
            id: default_id(),
            country: default_country(),
            region: default_region(),
            name: default_name(),
            country_name: default_country_name(),
            country_jname: default_country_jname(),
            region_name: default_region_name(),
            region_jname: default_region_jname(),
            port: default_port(),
            latitude: default_latitude(),
            longitude: default_longitude(),
            calendar_year: default_year(),
            holidays: Vec::new(),
        }
    }
}

impl From<FacilityDocument> for FacilityInfo {
    fn from(value: FacilityDocument) -> Self {
        Self {
            id: value.id,
            country: value.country,
            region: value.region,
            name: value.name,
            country_name: value.country_name,
            country_jname: value.country_jname,
            region_name: value.region_name,
            region_jname: value.region_jname,
            port: value.port,
            latitude: value.latitude,
            longitude: value.longitude,
            calendar_year: value.calendar_year,
            holidays: value.holidays,
        }
    }
}

fn default_id() -> String {
    "1".into()
}
fn default_country() -> String {
    "JP".into()
}
fn default_region() -> String {
    "JP-13".into()
}
fn default_name() -> String {
    "Vibea3 Arcade".into()
}
fn default_country_name() -> String {
    "Japan".into()
}
fn default_country_jname() -> String {
    "日本".into()
}
fn default_region_name() -> String {
    "Tokyo".into()
}
fn default_region_jname() -> String {
    "東京都".into()
}
fn default_port() -> u16 {
    5700
}
fn default_latitude() -> i32 {
    35_689_509
}
fn default_longitude() -> i32 {
    139_691_640
}
fn default_year() -> i16 {
    2026
}
