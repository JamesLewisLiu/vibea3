use std::net::Ipv4Addr;

use vibea3::{Ip4, Kbin, RpcContext, RpcError, RpcResponse, RpcResult, rpc};

use crate::{State, database::FacilityInfo, fault};

#[derive(Kbin)]
#[kbin(node = "facility")]
struct Request {
    #[kbin(attr)]
    encoding: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "facility")]
struct Response {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    location: Option<Location>,
    line: Option<Line>,
    portfw: Option<PortForward>,
    public: Option<Public>,
    share: Option<Share>,
    calendar: Option<Calendar>,
}

#[derive(Kbin)]
#[kbin(node = "location")]
struct Location {
    id: String,
    #[kbin(rename = "type")]
    r#type: u8,
    country: String,
    region: String,
    name: String,
    countryname: String,
    countryjname: String,
    regionname: String,
    regionjname: String,
    customercode: String,
    companycode: String,
    latitude: i32,
    longitude: i32,
    accuracy: u8,
}

#[derive(Kbin)]
#[kbin(node = "line")]
struct Line {
    id: String,
    class: u8,
    upclass: u8,
    rtt: u16,
}

#[derive(Kbin)]
#[kbin(node = "portfw")]
struct PortForward {
    globalip: Ip4,
    globalport: u16,
    privateport: u16,
}

#[derive(Kbin)]
#[kbin(node = "public")]
struct Public {
    flag: u8,
    name: String,
    latitude: String,
    longitude: String,
}

#[derive(Kbin)]
#[kbin(node = "share")]
struct Share {
    eacoin: EaCoin,
    url: Url,
    eapass: EaPass,
}

#[derive(Kbin)]
#[kbin(node = "eacoin")]
struct EaCoin {
    notchamount: i32,
    notchcount: i32,
    supplylimit: i32,
}

#[derive(Kbin)]
#[kbin(node = "url")]
struct Url {
    eapass: String,
    arcadefan: String,
    konaminetdx: String,
    konamiid: String,
    eagate: String,
}

#[derive(Kbin)]
#[kbin(node = "eapass")]
struct EaPass {
    valid: u16,
}

#[derive(Kbin)]
#[kbin(node = "calendar")]
struct Calendar {
    year: i16,
    #[kbin(array)]
    holiday: Vec<i16>,
}

#[rpc("facility.get")]
async fn get(ctx: RpcContext<State>, _request: Request) -> RpcResult<RpcResponse<Response>> {
    let machine = ctx
        .state
        .database
        .machine(ctx.srcid.clone().unwrap_or_default())
        .await
        .map_err(database_error)?;
    let mut facility = machine
        .map(|machine| machine.facility)
        .unwrap_or_else(FacilityInfo::default);
    if let Some(address) = ctx.peer_addr.map(|peer| peer.ip())
        && let Some(geo) = ctx
            .state
            .host
            .geolocate(address)
            .await
            .map_err(database_error)?
    {
        if let Some(country) = geo.country {
            facility.country = country;
        }
        if let Some(value) = geo.country_name {
            facility.country_name = value;
        }
        if let Some(value) = geo.country_jname {
            facility.country_jname = value;
        }
        if let Some(region) = geo.region {
            facility.region = format!("{}-{region}", facility.country);
        }
        if let Some(value) = geo.region_name {
            facility.region_name = value;
        }
        if let Some(value) = geo.region_jname {
            facility.region_jname = value;
        }
        if let Some(latitude) = geo.latitude {
            facility.latitude = (latitude * 1_000_000.0).round() as i32;
        }
        if let Some(longitude) = geo.longitude {
            facility.longitude = (longitude * 1_000_000.0).round() as i32;
        }
    }
    let web = ctx.state.config.web_ui_url.clone();
    Ok(RpcResponse::new(
        0,
        Response {
            expire: 43_200,
            fault: fault(&ctx.model),
            location: Some(Location {
                id: format!("{}-{}", facility.country, facility.id),
                r#type: 0,
                country: facility.country,
                region: facility.region,
                name: facility.name.clone(),
                countryname: facility.country_name,
                countryjname: facility.country_jname,
                regionname: facility.region_name,
                regionjname: facility.region_jname,
                customercode: "X000000000".into(),
                companycode: "X000000000".into(),
                latitude: facility.latitude,
                longitude: facility.longitude,
                accuracy: 9,
            }),
            line: Some(Line {
                id: "1".into(),
                class: 8,
                upclass: 8,
                rtt: 40,
            }),
            portfw: Some(PortForward {
                globalip: Ip4(ctx
                    .peer_addr
                    .and_then(|peer| match peer.ip() {
                        std::net::IpAddr::V4(address) => Some(address),
                        std::net::IpAddr::V6(_) => None,
                    })
                    .unwrap_or_else(|| Ipv4Addr::from(ctx.state.config.public_ip))),
                globalport: facility.port,
                privateport: facility.port,
            }),
            public: Some(Public {
                flag: 1,
                name: facility.name,
                latitude: coordinate_text(facility.latitude, 'N', 'S'),
                longitude: coordinate_text(facility.longitude, 'E', 'W'),
            }),
            share: Some(Share {
                eacoin: EaCoin {
                    notchamount: 0,
                    notchcount: 0,
                    supplylimit: 20_000,
                },
                url: Url {
                    eapass: web.clone(),
                    arcadefan: web.clone(),
                    konaminetdx: web.clone(),
                    konamiid: web.clone(),
                    eagate: web,
                },
                eapass: EaPass { valid: 365 },
            }),
            calendar: Some(Calendar {
                year: facility.calendar_year,
                holiday: facility.holidays,
            }),
        },
    ))
}

fn coordinate_text(value: i32, positive: char, negative: char) -> String {
    let direction = if value < 0 { negative } else { positive };
    let decimal = (i64::from(value).abs() as f64) / 1_000_000.0;
    let degrees = decimal.floor() as u16;
    let minutes_total = (decimal - f64::from(degrees)) * 60.0;
    let minutes = minutes_total.floor() as u8;
    let seconds = (minutes_total - f64::from(minutes)) * 60.0;
    format!("{direction}{degrees}.{minutes:02}.{seconds:04.1}")
}

fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}

#[cfg(test)]
mod tests {
    use super::coordinate_text;

    #[test]
    fn public_coordinates_use_official_dms_text() {
        assert_eq!(coordinate_text(35_689_509, 'N', 'S'), "N35.41.22.2");
        assert_eq!(coordinate_text(139_691_640, 'E', 'W'), "E139.41.29.9");
        assert_eq!(coordinate_text(-33_868_800, 'N', 'S'), "S33.52.07.7");
    }
}
