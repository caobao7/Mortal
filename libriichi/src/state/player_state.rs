use super::action::ActionCandidate;
use super::item::{ChiPon, KawaItem, Sutehai};
use crate::hand::tiles_to_string;
use crate::must_tile;
use crate::tile::Tile;
use std::iter;

use anyhow::Result;
use derivative::Derivative;
use pyo3::prelude::*;
use serde_json as json;
use tinyvec::{ArrayVec, TinyVec};

/// `PlayerState` is the core of the lib, which holds all the observable game
/// state information from a specific seat's perspective with the ability to
/// identify the legal actions the specified player can make upon an incoming
/// mjai event, along with some helper functions to build an actual agent.
/// Notably, `PlayerState` encodes observation features into numpy arrays which
/// serve as inputs for deep learning model.
#[pyclass]
#[derive(Debug, Clone, Derivative)]
#[derivative(Default)]
pub struct PlayerState {
    #[pyo3(get)]
    pub(super) player_id: u8,

    /// Does not include aka.
    #[derivative(Default(value = "[0; 34]"))]
    pub(super) tehai: [u8; 34],

    /// Does not consider yakunashi, but does consider other kinds of
    /// furiten.
    /// 听的牌
    #[derivative(Default(value = "[false; 34]"))]
    pub(super) waits: [bool; 34],

    /// dora牌 id : cnt
    #[derivative(Default(value = "[0; 34]"))]
    pub(super) dora_factor: [u8; 34],

    /// For calculating `waits` and `doras_seen`.
    /// 自己能看到已经出现过的所有牌 id : cnt
    #[derivative(Default(value = "[0; 34]"))]
    pub(super) tiles_seen: [u8; 34],

    /// 丢了之后向听数不变
    #[derivative(Default(value = "[false; 34]"))]
    pub(super) keep_shanten_discards: [bool; 34],

    /// 丢了之后向听数减小 (即进章了
    #[derivative(Default(value = "[false; 34]"))]
    pub(super) next_shanten_discards: [bool; 34],

    /// 不允许打出的牌 (如食替规则限制)
    #[derivative(Default(value = "[false; 34]"))]
    pub(super) forbidden_tiles: [bool; 34],

    /// Used for furiten check.
    /// 自身打过的牌
    #[derivative(Default(value = "[false; 34]"))]
    pub(super) discarded_tiles: [bool; 34],

    /// 场风
    pub(super) bakaze: Tile,
    /// 自风
    pub(super) jikaze: Tile,
    /// 东一~西四，%4
    /// Counts from 0 unlike mjai.
    pub(super) kyoku: u8,
    /// 本场
    pub(super) honba: u8,
    /// 点棒数
    pub(super) kyotaku: u8,
    /// Rotated, `scores[0]` is the score of the player.
    pub(super) scores: [i32; 4],
    pub(super) rank: u8,
    /// 亲
    /// Relative to `player_id`.
    pub(super) oya: u8,
    /// Including 西入 sudden deatch.
    pub(super) is_all_last: bool,
    /// dora指示牌
    pub(super) dora_indicators: ArrayVec<[Tile; 5]>,

    /// 24 is the theoretical max size of kawa, however, since None is included
    /// in the kawa, in some very rare cases (about one in a million hanchans),
    /// the size can exceed 24.
    ///
    /// Reference:
    /// <https://detail.chiebukuro.yahoo.co.jp/qa/question_detail/q1020002370>
    /// 牌河的牌
    pub(super) kawa: [TinyVec<[Option<KawaItem>; 24]>; 4],
    /// 最后一张手切的牌
    pub(super) last_tedashis: [Option<Sutehai>; 4],
    /// 立直宣告牌
    pub(super) riichi_sutehais: [Option<Sutehai>; 4],

    /// Using 34-D arrays here may be more efficient, but I don't want to mess up
    /// with aka doras.
    pub(super) kawa_overview: [ArrayVec<[Tile; 24]>; 4],
    pub(super) fuuro_overview: [ArrayVec<[ArrayVec<[Tile; 4]>; 4]>; 4],
    /// In this field all `Tile` are deaka'd.
    pub(super) ankan_overview: [ArrayVec<[Tile; 4]>; 4],

    /// 喊立直
    pub(super) riichi_declared: [bool; 4],
    /// 立直成功
    pub(super) riichi_accepted: [bool; 4],

    pub(super) at_turn: u8,
    /// 剩余牌数量
    pub(super) tiles_left: u8,
    /// 刚发生的杠事件，为了记录牌河的牌的状态
    pub(super) intermediate_kan: ArrayVec<[Tile; 4]>,
    /// 刚发生的吃碰事件，为了记录牌河的牌的状态
    pub(super) intermediate_chi_pon: Option<ChiPon>,

    /// 向听数
    pub(super) shanten: i8,

    /// 当前摸得最后一张牌
    pub(super) last_self_tsumo: Option<Tile>,
    /// 所有人牌河的最后一张牌
    pub(super) last_kawa_tile: Option<Tile>,
    /// 当前事件后能做的事
    pub(super) last_cans: ActionCandidate,

    /// Both deaka'd
    /// 辅助last_cans 可以暗杠的牌
    pub(super) ankan_candidates: ArrayVec<[Tile; 3]>,
    /// 辅助last_cans 可以加杠的牌
    pub(super) kakan_candidates: ArrayVec<[Tile; 3]>,
    /// 抢杠
    pub(super) chankan_chance: Option<()>,

    /// 能否w立
    pub(super) can_w_riichi: bool,
    /// 是否w立
    pub(super) is_w_riichi: bool,
    /// 是否岭上牌
    pub(super) at_rinshan: bool,
    /// 是否一发
    pub(super) at_ippatsu: bool,
    /// 是否在振听
    pub(super) at_furiten: bool,
    /// 用以辅助标记同巡振听
    pub(super) to_mark_same_cycle_furiten: Option<()>,

    /// Used for 4-kan check.
    pub(super) kans_on_board: u8,

    /// 是否门清
    pub(super) is_menzen: bool,
    /// For agari calc, all deaka'd.
    pub(super) chis: ArrayVec<[u8; 4]>,
    pub(super) pons: ArrayVec<[u8; 4]>,
    pub(super) minkans: ArrayVec<[u8; 4]>,
    pub(super) ankans: ArrayVec<[u8; 4]>,

    /// Including aka, originally for agari calc usage but also encoded as a
    /// feature to the obs. doras数
    pub(super) doras_owned: [u8; 4],
    pub(super) doras_seen: u8,

    /// 5p/m/s r 
    pub(super) akas_in_hand: [bool; 3],

    /// For shanten calc.
    pub(super) tehai_len_div3: u8,

    /// Used in can_riichi. 当向听数为1且这个为true，并且门清就可以立直了
    pub(super) has_next_shanten_discard: bool,
}

#[pymethods]
impl PlayerState {
    /// Panics if `player_id` is outside of range [0, 3].
    #[new]
    #[must_use]
    pub fn new(player_id: u8) -> Self {
        assert!(player_id < 4, "{player_id} is not in range [0, 3]");
        Self {
            player_id,
            ..Default::default()
        }
    }

    /// Returns an `ActionCandidate`.
    #[pyo3(name = "update")]
    pub(super) fn update_json(&mut self, mjai_json: &str) -> Result<ActionCandidate> {
        let event = json::from_str(mjai_json)?;
        Ok(self.update(&event))
    }

    /// Raises an exception if the action is not valid.
    #[pyo3(name = "validate_reaction")]
    pub(super) fn validate_reaction_json(&self, mjai_json: &str) -> Result<()> {
        let action = json::from_str(mjai_json)?;
        self.validate_reaction(&action)
    }

    /// For debug only.
    ///
    /// Return a human readable description of the current state.
    #[must_use]
    pub fn brief_info(&self) -> String {
        let waits = self
            .waits
            .iter()
            .enumerate()
            .filter(|(_, &b)| b)
            .map(|(i, _)| must_tile!(i))
            .collect::<Vec<_>>();

        let zipped_kawa = self.kawa[0]
            .iter()
            .chain(iter::repeat(&None))
            .zip(self.kawa[1].iter().chain(iter::repeat(&None)))
            .zip(self.kawa[2].iter().chain(iter::repeat(&None)))
            .zip(self.kawa[3].iter().chain(iter::repeat(&None)))
            .take_while(|row| !matches!(row, &(((None, None), None), None)))
            .enumerate()
            .map(|(i, (((a, b), c), d))| {
                format!(
                    "{i:2}. {}\t{}\t{}\t{}",
                    a.as_ref()
                        .map_or_else(|| "-".to_owned(), |item| item.to_string()),
                    b.as_ref()
                        .map_or_else(|| "-".to_owned(), |item| item.to_string()),
                    c.as_ref()
                        .map_or_else(|| "-".to_owned(), |item| item.to_string()),
                    d.as_ref()
                        .map_or_else(|| "-".to_owned(), |item| item.to_string()),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"player (abs): {}
oya (rel): {}
kyoku: {}{}-{}
turn: {}
jikaze: {}
score (rel): {:?}
tehai: {}
fuuro: {:?}
ankan: {:?}
tehai len: {}
shanten: {}
furiten: {}
waits: {waits:?}
dora indicators: {:?}
doras owned: {:?}
doras seen: {}
action candidates: {:#?}
last self tsumo: {:?}
last kawa tile: {:?}
tiles left: {}
kawa:
{zipped_kawa}"#,
            self.player_id,
            self.oya,
            self.bakaze,
            self.kyoku + 1,
            self.honba,
            self.at_turn,
            self.jikaze,
            self.scores,
            tiles_to_string(&self.tehai, self.akas_in_hand),
            self.fuuro_overview[0],
            self.ankan_overview[0],
            self.tehai_len_div3,
            self.shanten,
            self.at_furiten,
            self.dora_indicators,
            self.doras_owned,
            self.doras_seen,
            self.last_cans,
            self.last_self_tsumo,
            self.last_kawa_tile,
            self.tiles_left,
        )
    }
}
