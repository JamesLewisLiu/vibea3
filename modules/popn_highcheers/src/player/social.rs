use vibea3::Kbin;

use crate::protocol::COURSE_LICENSE_DATA_COUNT;

use crate::database::CourseRecord;

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct TsumTsumRequest {
    #[kbin(attr)]
    pub ref_id: String,
    #[kbin(attr)]
    pub uid: String,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct TsumTsumResponse {
    pub status: i8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct FriendRequest {
    #[kbin(attr)]
    pub ref_id: String,
    #[kbin(attr)]
    pub data_id: String,
    #[kbin(attr)]
    pub no: String,
    #[kbin(attr)]
    pub tran_no: String,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct FriendResponse {
    pub friend: Option<FriendData>,
}

impl FriendResponse {
    pub(super) fn disconnected() -> Self {
        Self {
            friend: Some(FriendData {
                con_flg: false,
                no: None,
                g_pm_id: None,
                name: None,
                chara_num: None,
                is_open: None,
                nice: None,
                music: Vec::new(),
                course_data: Vec::new(),
            }),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "friend")]
pub(super) struct FriendData {
    con_flg: bool,
    no: Option<i16>,
    g_pm_id: Option<String>,
    name: Option<String>,
    chara_num: Option<i16>,
    is_open: Option<i8>,
    #[kbin(array)]
    nice: Option<Vec<i16>>,
    #[kbin(repeated)]
    music: Vec<FriendMusic>,
    #[kbin(repeated)]
    course_data: Vec<CourseData>,
}

#[derive(Kbin)]
#[kbin(node = "music")]
struct FriendMusic {
    #[kbin(attr)]
    music_num: i16,
    #[kbin(attr)]
    sheet_num: u8,
    #[kbin(attr)]
    score: i32,
    #[kbin(attr, rename = "clearrank")]
    clear_rank: u8,
    #[kbin(attr, rename = "cleartype")]
    clear_type: u8,
}

#[derive(Kbin)]
#[kbin(node = "course_data")]
pub(super) struct CourseData {
    course_id: i16,
    clear_type: u8,
    clear_rank: u8,
    total_score: i32,
    update_count: i32,
    sheet_num: u8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct RankingRequest {
    pub pref: i16,
    pub location_id: String,
    pub ref_id: String,
    pub name: String,
    pub chara_num: i16,
    pub course_id: i16,
    pub total_score: i32,
    pub music_num: i16,
    pub sheet_num: u8,
    pub clear_type: u8,
    pub clear_rank: u8,
    pub play_id: i32,
    pub stage: u8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct RankingResponse {
    all_ranking: Option<RankingEntry>,
    pref_ranking: Option<RankingEntry>,
    location_ranking: Option<RankingEntry>,
}

impl RankingResponse {
    pub(super) fn empty() -> Self {
        Self {
            all_ranking: None,
            pref_ranking: None,
            location_ranking: None,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "ranking")]
struct RankingEntry {
    name: String,
    chara_num: i16,
    total_score: i32,
    clear_type: u8,
    clear_rank: u8,
    player_count: i16,
    player_rank: i16,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct CourseRequest {
    pub pref: i16,
    pub location_id: String,
    pub ref_id: String,
    pub data_id: String,
    pub name: String,
    pub chara_num: i16,
    pub play_id: i32,
    pub course_id: i16,
    pub course_name: String,
    pub stage1_music_num: i16,
    pub stage1_sheet_num: u8,
    pub stage2_music_num: i16,
    pub stage2_sheet_num: u8,
    pub stage3_music_num: i16,
    pub stage3_sheet_num: u8,
    pub stage4_music_num: i16,
    pub stage4_sheet_num: u8,
    pub norma_type: u8,
    pub norma_1_num: i32,
    pub norma_2_num: i32,
    pub clear_medal: u8,
    pub clear_norma: u8,
    pub total_score: i32,
    pub max_combo: i16,
    pub last_gauge: i16,
    pub is_image_store: bool,
    pub license: Option<CourseLicense>,
}

impl CourseRequest {
    pub(super) fn ref_id(&self) -> String {
        self.ref_id.clone()
    }

    pub(super) fn data_id(&self) -> String {
        self.data_id.clone()
    }

    pub(super) fn into_record(self) -> CourseRecord {
        let (is_license, license_data) = self
            .license
            .map(|license| {
                (
                    license.is_license,
                    normalize(license.license_data, COURSE_LICENSE_DATA_COUNT, -1),
                )
            })
            .unwrap_or((false, vec![-1; COURSE_LICENSE_DATA_COUNT]));
        CourseRecord {
            id: String::new(),
            user_id: String::new(),
            pref: self.pref,
            location_id: self.location_id,
            data_id: self.data_id,
            name: self.name,
            chara_num: self.chara_num,
            play_id: self.play_id,
            course_id: self.course_id,
            course_name: self.course_name,
            stage1_music_num: self.stage1_music_num,
            stage1_sheet_num: self.stage1_sheet_num,
            stage2_music_num: self.stage2_music_num,
            stage2_sheet_num: self.stage2_sheet_num,
            stage3_music_num: self.stage3_music_num,
            stage3_sheet_num: self.stage3_sheet_num,
            stage4_music_num: self.stage4_music_num,
            stage4_sheet_num: self.stage4_sheet_num,
            norma_type: self.norma_type,
            norma_1_num: self.norma_1_num,
            norma_2_num: self.norma_2_num,
            clear_medal: self.clear_medal,
            clear_norma: self.clear_norma,
            total_score: self.total_score,
            max_combo: self.max_combo,
            last_gauge: self.last_gauge,
            is_image_store: self.is_image_store,
            is_license,
            license_data,
            updated_at: bson::DateTime::now(),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "license")]
pub(super) struct CourseLicense {
    pub is_license: bool,
    #[kbin(array)]
    pub license_data: Vec<i16>,
}

fn normalize<T: Clone>(mut values: Vec<T>, length: usize, fill: T) -> Vec<T> {
    values.truncate(length);
    values.resize(length, fill);
    values
}

#[cfg(test)]
mod tests {
    use vibea3::{DecodeOptions, EncodeOptions, decode_kbin, encode_kbin, encode_xml};

    use super::*;

    #[test]
    fn course_license_uses_the_psmap_element_count() {
        let license = CourseLicense {
            is_license: true,
            license_data: vec![-1; COURSE_LICENSE_DATA_COUNT],
        };
        let xml =
            String::from_utf8(encode_xml(&license, EncodeOptions::default()).unwrap()).unwrap();
        assert!(xml.contains("license_data __type=\"s16\" __count=\"20\""));

        let kbin = encode_kbin(&license, EncodeOptions::default()).unwrap();
        let decoded = decode_kbin::<CourseLicense>(&kbin, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.license_data.len(), COURSE_LICENSE_DATA_COUNT);
    }
}
