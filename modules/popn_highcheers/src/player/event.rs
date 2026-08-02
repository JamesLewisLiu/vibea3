use vibea3::Kbin;

use crate::database::{EventBasket, EventState};

#[derive(Kbin)]
#[kbin(node = "event_p29")]
pub(super) struct EventResponse {
    basket_id: Option<i16>,
    #[kbin(repeated)]
    basket: Vec<EventBasketResponse>,
    ensta: Option<EnstaResponse>,
}

impl From<&EventState> for EventResponse {
    fn from(value: &EventState) -> Self {
        Self {
            basket_id: value.basket_id,
            basket: value.baskets.iter().cloned().map(Into::into).collect(),
            ensta: value.ensta_checked.map(|checked| EnstaResponse {
                serial_code: value.ensta_serial_code.clone().unwrap_or_default(),
                checked,
            }),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "basket")]
struct EventBasketResponse {
    id: i16,
    point: u32,
    is_cleared: bool,
}

impl From<EventBasket> for EventBasketResponse {
    fn from(value: EventBasket) -> Self {
        Self {
            id: value.id,
            point: value.point,
            is_cleared: value.is_cleared,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "ensta")]
struct EnstaResponse {
    serial_code: String,
    checked: bool,
}

#[derive(Kbin)]
#[kbin(node = "event_p29")]
pub(super) struct EventWrite {
    pub basket_id: Option<i16>,
    #[kbin(repeated)]
    pub basket: Vec<EventBasketWrite>,
    pub ensta: Option<EnstaWrite>,
}

#[derive(Kbin)]
#[kbin(node = "basket")]
pub(super) struct EventBasketWrite {
    pub id: i16,
    pub point: u32,
    pub is_cleared: bool,
}

#[derive(Kbin)]
#[kbin(node = "ensta")]
pub(super) struct EnstaWrite {
    pub checked: bool,
}

#[cfg(test)]
mod tests {
    use vibea3::{DecodeOptions, EncodeOptions, decode_kbin, encode_kbin, encode_xml};

    use super::*;

    #[test]
    fn serial_code_psmap_character_buffer_is_a_wire_string() {
        let response = EnstaResponse {
            serial_code: "SERIAL".into(),
            checked: true,
        };
        let xml =
            String::from_utf8(encode_xml(&response, EncodeOptions::default()).unwrap()).unwrap();
        assert!(xml.contains("<serial_code __type=\"str\">SERIAL</serial_code>"));
        assert!(!xml.contains("__type=\"bin\""));

        let kbin = encode_kbin(&response, EncodeOptions::default()).unwrap();
        let decoded = decode_kbin::<EnstaResponse>(&kbin, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.serial_code, "SERIAL");
    }
}
