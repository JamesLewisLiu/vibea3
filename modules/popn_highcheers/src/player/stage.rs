use vibea3::Kbin;

use crate::database::PlayStage;

/// Complete M39 `player.write/stage` record audited from `popn.dll`.
#[derive(Kbin)]
#[kbin(node = "stage")]
pub(super) struct StageWrite {
    pub no: i16,
    pub sheet: u8,
    pub clear_rank: u8,
    pub clear_type: u8,
    pub score: i32,
    pub cool: i16,
    pub great: i16,
    pub good: i16,
    pub bad: i16,
    pub combo: i16,
    pub highlight: i16,
    pub gauge: i16,
    pub gauge_type: i8,
    pub is_win: i8,
    pub matching: i8,
    pub ojama_vs: i8,
}

impl From<&StageWrite> for PlayStage {
    fn from(value: &StageWrite) -> Self {
        Self {
            music_num: value.no,
            sheet_num: value.sheet,
            clear_rank: value.clear_rank,
            clear_type: value.clear_type,
            score: value.score,
            cool: value.cool,
            great: value.great,
            good: value.good,
            bad: value.bad,
            combo: value.combo,
            highlight: value.highlight,
            gauge: value.gauge,
            gauge_type: value.gauge_type,
            is_win: value.is_win,
            matching: value.matching,
            ojama_vs: value.ojama_vs,
        }
    }
}
