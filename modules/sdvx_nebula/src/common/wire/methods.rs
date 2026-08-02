use super::super::*;
use super::*;

#[rpc("game.sv7_common")]
async fn common(ctx: RpcContext<State>, _request: EmptyRequest) -> RpcResult<CommonResponse> {
    Ok(CommonResponse::new(ctx.state.common.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibea3::{DecodeOptions, EncodeOptions, NameMode, decode_kbin, encode_kbin};

    const FIRST_MUSIC_ID: u32 = 1;
    const ULTIMATE_MUSIC_ID: u32 = 636;

    #[test]
    fn checked_in_catalog_matches_the_pinned_update_without_count_invariants() {
        let data = load_from_manifest().unwrap();
        let first = data
            .music
            .iter()
            .find(|music| music.music_id == FIRST_MUSIC_ID)
            .unwrap();
        assert_eq!(first.title_name, "ALBIDA Powerless Mix");
        assert!(
            data.music
                .iter()
                .any(|music| music.music_id == ULTIMATE_MUSIC_ID && music.ultimate.is_some())
        );
        assert!(data.appeal_cards.iter().any(|card| card.appeal_id == 1));
        assert!(
            data.akaname_parts
                .iter()
                .any(|part| part.part_id == 10_001 && part.is_default)
        );
    }

    #[test]
    fn catalog_delta_round_trips_as_shift_jis_kbin() {
        let data = load_from_manifest().unwrap();
        let mut response = CommonResponse::new(&data);
        response.music = Some(MusicCatalog {
            info: data.music.iter().take(2).cloned().collect(),
        });
        response.appealcard = Some(AppealCards {
            info: data.appeal_cards.iter().take(2).cloned().collect(),
        });
        let packet = encode_kbin(
            &response,
            EncodeOptions {
                encoding: 0x80,
                names: NameMode::Packed,
            },
        )
        .unwrap();
        let decoded: CommonResponse = decode_kbin(&packet, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.music.unwrap().info[0].music_id, FIRST_MUSIC_ID);
        assert_eq!(decoded.appealcard.unwrap().info[0].appeal_id, 1);
    }

    #[test]
    fn codec_can_represent_the_full_official_local_catalog() {
        let data = load_from_manifest().unwrap();
        let mut response = CommonResponse::new(&data);
        response.music = Some(MusicCatalog {
            info: data.music.clone(),
        });
        response.appealcard = Some(AppealCards {
            info: data.appeal_cards.clone(),
        });
        response.akaname = Some(Akaname {
            info: data.akaname_parts.clone(),
        });
        let packet = encode_kbin(
            &response,
            EncodeOptions {
                encoding: 0x80,
                names: NameMode::Packed,
            },
        )
        .unwrap();
        let decoded: CommonResponse = decode_kbin(&packet, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.music.unwrap().info.len(), data.music.len());
        assert_eq!(
            decoded.appealcard.unwrap().info.len(),
            data.appeal_cards.len()
        );
        assert_eq!(
            decoded.akaname.unwrap().info.len(),
            data.akaname_parts.len()
        );
    }

    fn load_from_manifest() -> Result<CommonData, String> {
        let old = std::env::var_os("SDVX_NEBULA_DATA_DIR");
        unsafe {
            std::env::set_var(
                "SDVX_NEBULA_DATA_DIR",
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sdvx_nebula"),
            );
        }
        let result = load();
        match old {
            Some(value) => unsafe { std::env::set_var("SDVX_NEBULA_DATA_DIR", value) },
            None => unsafe { std::env::remove_var("SDVX_NEBULA_DATA_DIR") },
        }
        result
    }
}
