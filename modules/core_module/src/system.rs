use des::{
    TdesEde3,
    cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray},
};
use vibea3::{Kbin, RpcContext, RpcResponse, RpcResult, rpc};

use crate::{CARD_ID_HEX_LENGTH, State, fault};

macro_rules! system_packet {
    ($request:ident, $response:ident, $data:ident, $node:literal) => {
        #[derive(Kbin)]
        #[kbin(node = $node)]
        struct $request {
            data: CardData,
        }

        #[derive(Kbin)]
        #[kbin(node = $node)]
        struct $response {
            #[kbin(attr)]
            expire: i32,
            #[kbin(attr)]
            fault: Option<String>,
            result: i32,
            data: Option<$data>,
        }

        #[derive(Kbin)]
        #[kbin(node = "data")]
        struct $data {
            card_number: String,
        }
    };
}

#[derive(Kbin)]
#[kbin(node = "data")]
struct CardData {
    card_id: String,
}

system_packet!(SystemRequest, SystemResponse, SystemData, "system");
system_packet!(System2Request, System2Response, System2Data, "system_2");
system_packet!(System3Request, System3Response, System3Data, "system_3");

#[rpc("system.convcardnumber")]
async fn convert(
    ctx: RpcContext<State>,
    request: SystemRequest,
) -> RpcResult<RpcResponse<SystemResponse>> {
    let number = card_id_to_user_code(&request.data.card_id);
    Ok(RpcResponse::new(
        i32::from(number.is_none()),
        SystemResponse {
            expire: 0,
            fault: fault(&ctx.model),
            result: i32::from(number.is_none()),
            data: number.map(|card_number| SystemData { card_number }),
        },
    ))
}

#[rpc("system_2.convcardnumber")]
async fn convert_2(
    ctx: RpcContext<State>,
    request: System2Request,
) -> RpcResult<RpcResponse<System2Response>> {
    let number = card_id_to_user_code(&request.data.card_id);
    Ok(RpcResponse::new(
        i32::from(number.is_none()),
        System2Response {
            expire: 0,
            fault: fault(&ctx.model),
            result: i32::from(number.is_none()),
            data: number.map(|card_number| System2Data { card_number }),
        },
    ))
}

#[rpc("system_3.convcardnumber")]
async fn convert_3(
    ctx: RpcContext<State>,
    request: System3Request,
) -> RpcResult<RpcResponse<System3Response>> {
    let number = card_id_to_user_code(&request.data.card_id);
    Ok(RpcResponse::new(
        i32::from(number.is_none()),
        System3Response {
            expire: 0,
            fault: fault(&ctx.model),
            result: i32::from(number.is_none()),
            data: number.map(|card_number| System3Data { card_number }),
        },
    ))
}

pub(super) fn card_id_to_user_code(card_id: &str) -> Option<String> {
    const BITS_PER_BYTE: usize = 8;
    const CIPHER_BLOCK_BYTES: usize = 8;
    const EXPANDED_BLOCK_BYTES: usize = CIPHER_BLOCK_BYTES + 1;
    const USER_CODE_SYMBOL_COUNT: usize = 16;
    const PAYLOAD_SYMBOL_COUNT: usize = 13;
    const CARD_TYPE_SYMBOL_INDEX: usize = 14;
    const CHECKSUM_SYMBOL_INDEX: usize = 15;
    const BITS_PER_SYMBOL: usize = 5;
    const SYMBOL_MASK: u32 = 0x1f;
    const CHECKSUM_WEIGHT_COUNT: usize = 3;
    const TDES_EDE3_KEY_BYTES: usize = 24;
    const CARD_TYPE_EAMUSE: u8 = 1;
    const CARD_TYPE_NUMERIC: u8 = 4;
    const CARD_TYPE_OTHER: u8 = 2;
    const KEY: [u8; TDES_EDE3_KEY_BYTES] = [
        0x7e, 0x92, 0x4e, 0xd8, 0xd8, 0x84, 0x64, 0xc6, 0x5c, 0xb2, 0xde, 0xea, 0xb0, 0xb0, 0xb0,
        0xca, 0x9a, 0xca, 0x90, 0xc2, 0xb2, 0xe0, 0xf2, 0x42,
    ];
    const MAP: &[u8; 1 << BITS_PER_SYMBOL] = b"0123456789ABCDEFGHJKLMNPRSTUWXYZ";
    if card_id.len() != CARD_ID_HEX_LENGTH || !card_id.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    let mut block = [0u8; CIPHER_BLOCK_BYTES];
    for (index, pair) in card_id.as_bytes().chunks_exact(2).enumerate() {
        block[CIPHER_BLOCK_BYTES - 1 - index] =
            (hex(pair[0])? << (BITS_PER_BYTE / 2)) | hex(pair[1])?;
    }
    TdesEde3::new_from_slice(&KEY)
        .ok()?
        .encrypt_block(GenericArray::from_mut_slice(&mut block));
    let mut expanded = [0u8; EXPANDED_BLOCK_BYTES];
    expanded[..CIPHER_BLOCK_BYTES].copy_from_slice(&block);
    let mut raw = [0u8; USER_CODE_SYMBOL_COUNT];
    for (index, output) in raw[..PAYLOAD_SYMBOL_COUNT].iter_mut().enumerate() {
        for offset in 0..BITS_PER_SYMBOL {
            let bit = index * BITS_PER_SYMBOL + offset;
            *output = (*output << 1)
                | ((expanded[bit / BITS_PER_BYTE] >> (BITS_PER_BYTE - 1 - bit % BITS_PER_BYTE))
                    & 1);
        }
    }
    raw[PAYLOAD_SYMBOL_COUNT] = 1;
    raw[CARD_TYPE_SYMBOL_INDEX] = if card_id.starts_with("E004") {
        CARD_TYPE_EAMUSE
    } else if card_id.starts_with("4000") && card_id.bytes().all(|byte| byte.is_ascii_digit()) {
        CARD_TYPE_NUMERIC
    } else {
        CARD_TYPE_OTHER
    };
    let mut card_type = raw[CARD_TYPE_SYMBOL_INDEX];
    for value in &mut raw[..CARD_TYPE_SYMBOL_INDEX] {
        *value ^= card_type;
        card_type = *value;
    }
    let mut check = 0u32;
    for (index, value) in raw[..CHECKSUM_SYMBOL_INDEX].iter().enumerate() {
        check += ((index % CHECKSUM_WEIGHT_COUNT) + 1) as u32 * u32::from(*value);
    }
    while check > SYMBOL_MASK {
        check = (check & SYMBOL_MASK) + (check >> BITS_PER_SYMBOL);
    }
    raw[CHECKSUM_SYMBOL_INDEX] = check as u8;
    Some(
        raw.into_iter()
            .map(|value| MAP[value as usize] as char)
            .collect(),
    )
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::card_id_to_user_code;

    #[test]
    fn matches_mermaid_vectors() {
        assert_eq!(
            card_id_to_user_code("E004010000000000").as_deref(),
            Some("0PFCX4FY5XHY6715")
        );
        assert_eq!(
            card_id_to_user_code("E0040123456789AB").as_deref(),
            Some("2KWBT127XD4CGH1Z")
        );
        assert_eq!(
            card_id_to_user_code("4000000000000000").as_deref(),
            Some("G68M8J3S6W5Y014F")
        );
    }
}
