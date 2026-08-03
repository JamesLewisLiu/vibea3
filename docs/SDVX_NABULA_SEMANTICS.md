# SDVX Nabula semantic notes

These notes describe behavior recovered from the exact KFC 2026-07-14
`sv7.dll`. They distinguish fixed wire layout from fields whose business name
is only meaningful to one client subsystem.

Primary evidence locations in this build are `sub_18030FEE0` for extend
records, `sub_1800AE3B0` and `sub_1805BEAB0` for the event registry/dispatcher,
`sub_180611690` plus `sub_180322D40` for music records, and
`sub_180328640`/`sub_180328870` for profile parameter save/load. The option
defaults are initialized by `sub_18053C220`.

## `extend/info`

`extend_type` selects the outer table, but it is usually `param_num_1` or
another numeric operand that selects behavior inside that table. `extend_id`
is not a universal subtype: it may be an item/music identity, an ordering key,
a sentinel, a fallback value, or ignored.

The complete field-level reference, including all 24 outer types, command
grammars, subtype gates, ignored fields, and the client-wide 256-record input
limit, is in [SDVX_NABULA_EXTEND.md](SDVX_NABULA_EXTEND.md).

## `sv7_load_m` music parameter array

The client requires at least 26 `u32` values and consumes the first 26. It has three score
domains. Domain A is the primary server record; domains B and C are retained
by the client for alternate record sources. Selection mode 0 reads A, mode 1
reads B, mode 2 takes the better of A/B, and mode 3 takes the better of all
three.

| Index | Client meaning |
|---:|---|
| 0 | music ID |
| 1 | chart/music type (`0..5`) |
| 2 | domain A score |
| 3 | domain A EX score |
| 4 | domain A clear type (`0..6`) |
| 5 | domain A grade (`0..10`) |
| 6 | domain A max chain |
| 7 | domain A play count |
| 8 | domain A clear count |
| 9 | domain A Ultimate Chain count |
| 10 | domain A Perfect Ultimate Chain count |
| 11 | supplied Volforce value/cap |
| 12 | domain B score |
| 13 | domain B EX score |
| 14 | domain B clear type |
| 15 | domain B grade |
| 16 | domain B max chain |
| 17 | domain B play count |
| 18 | domain C score |
| 19 | domain C EX score |
| 20 | domain C clear type |
| 21 | domain C grade |
| 22 | domain C play count |
| 23 | button rate |
| 24 | long-note rate |
| 25 | VOL/laser rate |

The client derives effective Volforce and six per-level values after loading;
those derived values are not extra wire members. Vibea3 populates domain A and
leaves alternate domains zero until a real source for them exists.

## `sv7_save` / `sv7_load` profile parameters

`param/info` is not keyed by `type` alone. The required record is:

```xml
<info>
  <type __type="s32">...</type>
  <id __type="s32">...</id>
  <param __type="s32" __count="256">...</param>
</info>
```

The client has a 512-slot internal record, but the wire format carries 256
slots and mirrors them into both internal halves on load. Save omits an
all-zero record. Vibea3 therefore persists `(type, id)` as the compound key,
pads short records to 256 values on output, and does not reinterpret unknown
types or IDs.

Known `type=2, id=0` option slots are:

| Slot | Control |
|---:|---|
| 38 | S-Critical display toggle |
| 53 | challenge type |
| 56 | EARLY/LATE display border |
| 57 | plus/minus millisecond display border |
| 65 | system voice toggle |
| 67 | score display style |
| 81 | background mask |
| 85 | EARLY/LATE color/deluxe toggle |
| 103 | target-score type |
| 115 | Konasute-related toggle |

The built-in reset also initializes slot 2 to `47`. Other parameter records
belong to event, story, customization, or per-feature subsystems and are kept
opaque so a client update cannot be damaged by a guessed schema.

## Recognized `event_id` commands

`event/info/event_id` is a command line: the first token is one of the names
below and the rest of the line is that command's argument string. The ID is the
client's internal registry number.

| ID | Command | ID | Command |
|---:|---|---:|---|
| 0 | `DUMMY_UNAVAILABLE` | 1 | `DUMMY_AVAILABLE` |
| 2 | `DUMMY_DEBUGVOID` | 3 | `DUMMY_DEBUGNOVOID` |
| 4 | `MATCHING_MODE` | 5 | `MATCHING_MODE_FREE_IP` |
| 6 | `APICAGACHADRAW` | 7 | `PAUSE_ONLINEUPDATE` |
| 8 | `TENKAICHI_MODE` | 9 | `ICON_POLICY_BREAK` |
| 10 | `ICON_FLOOR_INFECTION` | 11 | `PCBSTATE_CABINETGROUPID` |
| 12 | `SERIALCODE_JP` | 13 | `SERIALCODE_KR` |
| 14 | `SERIALCODE_AS` | 15 | `SERIALCODE_ID` |
| 16 | `SERIALCODE_US` | 17 | `ACHIEVEMENT_ENABLE` |
| 18 | `TOTAL_MEMORIAL_ENABLE` | 19 | `EVENTDATE_APRILFOOL` |
| 20 | `VOLFORCE_ENABLE` | 21 | `FACTORY` |
| 22 | `OMEGA_ENABLE` | 23 | `APPEAL_CARD_UNLOCK` |
| 24 | `APPEAL_CARD_GEN_NEW_PRICE` | 25 | `APPEAL_CARD_GEN_PRICE` |
| 26 | `CLOUD_LINK_ENABLE` | 27 | `EVENTDATE_ONIGO` |
| 28 | `DISABLE_MONITOR_ID_CHECK` | 29 | `EVENTDATE_GOTT` |
| 30 | `FAVORITE_MUSIC_MAX` | 31 | `FAVORITE_APPEALCARD_MAX` |
| 32 | `GENERATOR_ABLE` | 33 | `CREW_SELECT_ABLE` |
| 34 | `SKILL_ANALYZER_ABLE` | 35 | `BLASTER_ABLE` |
| 36 | `STANDARD_UNLOCK_ENABLE` | 37 | `PLAYERJUDGEADJ_ENABLE` |
| 38 | `DISP_PASELI_BANNER` | 39 | `MIXID_INPUT_ENABLE` |
| 40 | `BEMANI_VOTING_2019_ENABLE` | 41 | `KONAMI_50TH_LOGO` |
| 42 | `PREMIUM_TIME_ENABLE` | 43 | `HEXA_ENABLE` |
| 44 | `MEGAMIX_ENABLE` | 45 | `ARENA_ENABLE` |
| 46 | `VALGENE_ENABLE` | 47 | `APRIL_GRACE` |
| 48 | `NEW_YEAR_2022` | 49 | `VALGENE_MASK_BANNER` |
| 50 | `CHARACTER_DISABLE` | 51 | `CHARACTER_IGNORE_DISABLE` |
| 52 | `STAMP_DISABLE` | 53 | `STAMP_IGNORE_DISABLE` |
| 54 | `DISABLED_MUSIC_IN_ARENA_ONLINE` | 55 | `ARENA_ALTER_MODE_WINDOW_ENABLE` |
| 56 | `ARENA_LOCAL_TO_ONLINE_ENABLE` | 57 | `SUBBG_DISABLE` |
| 58 | `SUBBG_IGNORE_DISABLE` | 59 | `NEMSYS_DISABLE` |
| 60 | `NEMSYS_IGNORE_DISABLE` | 61 | `CUSTOMBGM_DISABLE` |
| 62 | `CUSTOMBGM_IGNORE_DISABLE` | 63 | `ARENA_PASS_MATCH_WINDOW_ENABLE` |
| 64 | `ARENA_VOTE_MODE_ENABLE` | 65 | `ARENA_LOCAL_ULTIMATE_MATCH_ALWAYS` |
| 66 | `DEMOLOOP_PASELI_FESTIVAL_2022` | 67 | `MEGAMIX_BATTLE_MATCH_ENABLE` |
| 68 | `DEMOLOOP_INFORMATION` | 69 | `S_PUC_EFFECT_ENABLE` |
| 70 | `GENERATE_USTA_LINK_ID_ENABLE` | 71 | `ValgeneTicketCaution` |
| 72 | `SINGLE_BATTLE_ENABLE` | 73 | `PLAYER_RADAR_ENABLE` |
| 74 | `DEFALUT_MUSIC_ID` | 75 | `DEFAULT_MUSIC_ID` |
| 76 | `BEGINNER_MUSIC_FOLDER` | 77 | `HISCORE_DATA_LIMIT` |
| 78 | `SUBMONITOR_VSYNC_ENABLE` | 79 | `USE_CUDA_VIDEO_PRESENTER` |
| 80 | `SYSBG_DISABLE` | 81 | `SYSBG_IGNORE_DISABLE` |
| 82 | `HEXA_OVERDRIVE_ENABLE` | 83 | `MERRY_CHRISTMAS_2023` |
| 84 | `VALENTINES_DAY_2024` | 85 | `WHITE_DAY_2024` |
| 86 | `SEASON_VOICE_DISABLE` | 87 | `CAMERA_CALIBRATION_TEST_MENU_ENABLE` |
| 88 | `APRIL_RAINBOW_LINE_ACTIVE` | 89 | `TAMAADV_ENABLE` |
| 90 | `HALLOWEEN_EVENT` | 91 | `DemoLoopMusicOverwriteHigh` |
| 92 | `DemoLoopMusicOverwriteLow` | 93 | `SKILLLEVEL_AVERAGE_SCORE_DISP_ENABLE` |
| 94 | `FAVORITE_CREW_ENABLE` | 95 | `FAVORITE_CREW_MAX` |
| 96 | `TAMAADV_VALGENE_BONUS_ENABLE` | 97 | `YUKKURI_RASIS_CREW_ENABLE` |
| 98 | `YUKKURI_RASIS_TITLE_ENABLE` | 99 | `ULTIMATE_MATCH_PLAYABLE_ALWAYS` |
| 100 | `OVER_POWER_ENABLE` | 101 | `BlasterStageAvailable` |
| 102 | `FAVORITE_APPEALSTAMP_MAX` | 103 | `FAVORITE_CUSTOM_NEMSYS_MAX` |
| 104 | `FAVORITE_SYSTEM_BGM_MAX` | 105 | `FAVORITE_SUBMONITOR_BG_MAX` |
| 106 | `FAVORITE_SYSTEM_BG_MAX` | 107 | `NAMEPLATE_BADGE_DISABLE` |
| 108 | `NAMEPLATE_BADGE_IGNORE_DISABLE` | 109 | `NAMEPLATE_BG_DISABLE` |
| 110 | `NAMEPLATE_BG_IGNORE_DISABLE` | 111 | `FAVORITE_NAMEPLATE_BG_MAX` |
| 112 | `FAVORITE_NAMEPLATE_BADGE_MAX` | 113 | `APIPAGENE_ENABLE` |
| 114 | `APIPA_MASK_BANNER` | 115 | `ACHIEVEMENT_EVENT_MISSION` |
| 116 | `CARDLESS_AUTH_ENABLE` | 117 | `IGNORE_ALL_INPUTTED_CONDITION_MUSICS` |
| 118 | `CardMngGetPlayList` | 119 | `CHARACTER_KIND_DISABLE` |
| 120 | `CHARACTER_KIND_IGNORE_DISABLE` |  |  |

Commands ending in `_ENABLE`/`_ABLE` are generally presence gates. Commands
named `*_MAX`, `*_PRICE`, IDs, dates, masks, modes, and disable lists consume
arguments. Vibea3 deliberately does not reject unknown commands so a newer
module data file remains forward-compatible, but this table is the complete
registry for the pinned binary.
