use std::{net::IpAddr, path::Path, sync::Arc};

use maxminddb::{Reader, geoip2};
use vibea3::GeoLocation;

#[derive(Clone, Default)]
pub struct GeoIp {
    reader: Option<Arc<Reader<Vec<u8>>>>,
}

impl GeoIp {
    pub fn open(path: Option<&Path>) -> Result<Self, String> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        Ok(Self {
            reader: Some(Arc::new(
                Reader::open_readfile(path).map_err(|error| error.to_string())?,
            )),
        })
    }

    pub fn lookup(&self, address: IpAddr) -> Result<Option<GeoLocation>, String> {
        let Some(reader) = &self.reader else {
            return Ok(None);
        };
        let lookup = match reader.lookup(address) {
            Ok(lookup) => lookup,
            Err(_) => return Ok(None),
        };
        let Some(city) = lookup
            .decode::<geoip2::City>()
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        let subdivision = city.subdivisions.first();
        Ok(Some(GeoLocation {
            country: city.country.iso_code.map(str::to_owned),
            country_name: city.country.names.english.map(str::to_owned),
            country_jname: city.country.names.japanese.map(str::to_owned),
            region: subdivision
                .and_then(|value| value.iso_code)
                .map(str::to_owned),
            region_name: subdivision
                .and_then(|value| value.names.english)
                .map(str::to_owned),
            region_jname: subdivision
                .and_then(|value| value.names.japanese)
                .map(str::to_owned),
            city_name: city.city.names.english.map(str::to_owned),
            latitude: city.location.latitude,
            longitude: city.location.longitude,
        }))
    }
}
