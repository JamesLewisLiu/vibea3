mod data;
pub(crate) mod wire;

use std::path::PathBuf;

use vibea3::{RpcContext, RpcResult, rpc};

use crate::State;
pub(crate) use data::InfoData;
use wire::{CommonRequest, CommonResponse};

pub(crate) fn load() -> Result<InfoData, String> {
    let directory = std::env::var_os("POPN_HIGHCHEERS_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("VIBEA3_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data"))
                .join("popn_highcheers")
        });
    InfoData::load(&directory)
}

#[rpc("info.common")]
async fn common(ctx: RpcContext<State>, request: CommonRequest) -> RpcResult<CommonResponse> {
    let _ = request.loc_id;
    let usage = ctx
        .state
        .database
        .usage_snapshot(None)
        .await
        .map_err(crate::player::database_error)?;
    Ok(CommonResponse::new(ctx.state.info.as_ref(), &usage))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use encoding_rs::SHIFT_JIS;

    use super::InfoData;

    const FIRST_MUSIC_ID: i16 = 0;
    const ODO_MUSIC_ID: i16 = 2319;
    const EXPECTED_LICENSE_MUSIC: &[i16] = &[
        670, 2208, 2236, 2257, 2272, 2286, 2288, 2289, 2290, 2309, 2312, 2313, 2314, 2316, 2318,
        2319, 2320, 2321,
    ];

    #[test]
    fn checked_in_catalog_is_the_complete_m39_table() {
        let directory =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/popn_highcheers");
        let data = InfoData::load(&directory).unwrap();
        let first = data
            .music_catalog
            .iter()
            .find(|music| music.music_id == FIRST_MUSIC_ID)
            .unwrap();
        assert_eq!(first.title, "I REALLY WANT TO HURT YOU");
        let odo = data
            .music_catalog
            .iter()
            .find(|music| music.music_id == ODO_MUSIC_ID)
            .unwrap();
        assert_eq!(odo.artist, "Ado");
        assert!(data.music_catalog.iter().any(|music| {
            let (encoded, _, _) = SHIFT_JIS.encode(&music.title_sort);
            encoded.len() > 127
        }));
        assert!(data.music_supplements.is_empty());
        assert_eq!(data.license_music, EXPECTED_LICENSE_MUSIC);
        assert!(data.phases.is_empty());
        assert!(data.news.is_empty());
    }
}
