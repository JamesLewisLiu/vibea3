# SDVX Nebula (`sv7.dll`) reverse-engineering notes

## Target build

- Client: `soundvoltex.dll` / `sv7.dll`
- SHA-256: `c2d1b5e08953994dd5fe658d8a574ea74db625103f5ea4eb53e72038b95d0b83`
- Model: `KFC`
- Update release code: `2026071400`
- Module date gate: `20260714` through the unbounded future. Vibea3 gates on the eight-digit date portion of the model string.
- Exported service: `local`
- Dynamic module name: `sdvx-nebula`

The workspace copy at `samples/sdvx_nebula_update` matches the IDA input hash. No implementation step depends on files outside the workspace.

## Route inventory

The client registers exactly 30 XRPC methods, all under class `game`:

| Method | Request builder | Response consumer |
|---|---:|---:|
| `sv7_save_usta_link` | `0x18022bbe0` | `0x18022bca0` |
| `sv7_buy` | `0x18060e8d0` | `0x18060eb30` |
| `sv7_common` | `0x18060ed50` | `0x18060eef0` |
| `sv7_log` | `0x18060f580` | none |
| `sv7_entry_e` | `0x18060f790` | none |
| `sv7_entry_s` | `0x18060f950` | `0x18060fb70` |
| `sv7_exception` | `0x18060fe50` | none |
| `sv7_frozen` | `0x180610040` | `0x180610150` |
| `sv7_hiscore` | `0x1806102c0` | `0x180610450` |
| `sv7_load` | `0x180610560` | `0x180610840` |
| `sv7_load_ap` | `0x180611200` | `0x180611320` |
| `sv7_load_m` | `0x1806115a0` | `0x180611690` |
| `sv7_load_r` | `0x180611a00` | `0x180611af0` |
| `sv7_lounge` | `0x180611cc0` | `0x180611da0` |
| `sv7_new` | `0x180611fa0` | none |
| `sv7_play_e` | `0x180612330` | none |
| `sv7_play_s` | `0x180612870` | `0x180612920` |
| `sv7_sample` | `0x180612a20` | `0x180612b00` |
| `sv7_save` | `0x180612c20` | none |
| `sv7_save_ap` | `0x1806135b0` | `0x1806137f0` |
| `sv7_save_campaign` | `0x180613a10` | `0x180613c90` |
| `sv7_save_c` | `0x180613f30` | none |
| `sv7_save_e` | `0x1806149a0` | `0x180615030` |
| `sv7_save_fi` | `0x1806154a0` | `0x1806155c0` |
| `sv7_save_mega` | `0x180615780` | none |
| `sv7_save_m` | `0x180615e20` | none |
| `sv7_save_pb` | `0x180616260` | `0x180616380` |
| `sv7_serial` | `0x180616510` | `0x1806165c0` |
| `sv7_shop` | `0x180616790` | `0x1806175f0` |
| `sv7_save_valgene` | `0x1806177e0` | `0x180617a60` |

`modules/sdvx_nebula/src/lib.rs` asserts this exact route set so additions or omissions cannot be accidental.

## Required response corners

These are cases where status `0` alone is insufficient:

- `sv7_hiscore` requires an `sc` node. `sub_18031A690` logs `node(sc) not found` and rejects a response without it. An empty ranking therefore emits `<sc/>`, not an entirely empty game node.
- `sv7_serial` requires `serial_name`, `gamecoin_packet`, `gamecoin_block`, and an s8 `result`. Its `item` children are repeated root nodes with u32 `type`, `id`, `param`, and `param_after`.
- `sv7_save_usta_link` consumes `usta_link/link_id`; a generic `result` field is ignored.
- `sv7_sample` consumes the string attribute `release@` into a 32-byte client buffer.
- `sv7_entry_s` loops over repeated `entry` nodes containing `port:u16`, `gip:ip4`, and `lip:ip4`.
- `sv7_frozen`, `sv7_buy`, `sv7_save_fi`, and `sv7_save_pb` use one-byte result values, not s32.
- `sv7_load_m` ignores a music `param` array unless it contains at least 26 u32 elements.

## Psmap schemas

The only XRPC property-map imports in this client are:

- Lounge wait entry: `m_id:u32`.
- Shop response: `nxt_time:u32`, official default 1800 seconds.
- Rival score: `param:u32[6]`.

The psmap enum is interpreted through `docs/AVS2_PSMAP_TYPES.md`, not as normal property type IDs. Lounge waits are repeated `wait` nodes; rival parameters are actual property arrays.

## Track save schema

`sub_18033DE10` emits repeated `track` nodes. The important asymmetric types are:

- `play_id`, `music_id`, `music_type`, `score`, `exscore`, `volforce`, clear/grade/count/rate values: u32.
- `track_no`, online/local counts, and frame-drop counters: u16.
- `mode`, `start_option`, `gauge_type`, `notes_option`, and `challenge_type`: u8.
- `retry_cnt` and `mix_id`: s32.
- `mix_like`: bool.
- `judge`: s32 array with seven fixed client result slots.
- `matching`: up to three repeated nodes, each containing `code:str` and `score:u32`.

The full `sv7_save` request additionally includes profile currency deltas, Variant Gate state, settings, item deltas, 256-slot parameter records, story progress, course results, festival/arena state, print state, and the same repeated tracks. Conditional groups are represented as `Option<T>` rather than omitted from the Rust schema.

## Local data

The update ships authoritative local catalogs. `tools/extract_sdvx_nebula_data.py` pins and converts:

| Source | SHA-256 |
|---|---|
| `music_db.xml` | `80de93020f05393e8aa004508fb7c75b17a8badf2281837058725f042b088230` |
| `appeal_card.xml` | `84d626285f7ff5d9cab007f40aca0b5f3825b748da035d017bd2036003f99699` |
| `akaname_parts.xml` | `9a16dcfb0b174931bc502efd2803a2d50c03f480e526b7bd02d14c210b99c46e` |

Generated data lives in `data/sdvx_nebula/catalog.json`; server-controlled deltas live separately in `info.json`. Tests use anchor records and never freeze mutable catalog counts or a last ID.

`sv7_common` is delta-oriented because the game already loads these local databases. The codec is nevertheless tested with the entire catalog tree and can represent it.

## Kbin limit correction

The previous codec default treated 65,535 nodes as if it were a protocol limit. It is not: official music databases exceed it. The default resource guard is now 500,000 nodes and remains configurable.

Variable-value lengths are u32. The old artificial `0x00ff_ffff` encoder rejection was removed; checked u32 conversions now guard individual values and schema/data section lengths. Regression tests cover both a value immediately above 24 bits and the complete official SDVX catalog tree.

## Persistence architecture

Bootstrap supplies only the generic document database. The module owns its models and collections:

- `sdvx7_players`: profile/settings/items/parameters/story state, keyed by global `dataid`.
- `sdvx7_scores`: unique `(user_id, music_id, music_type)` records.
- `sdvx7_plays`: play lifecycle/history.
- `sdvx7_cabinets`: shop/cabinet state.
- `sdvx7_automations`: automation metadata.
- `sdvx7_counters`: atomic play IDs.

Authentication is resolved through core `card_sessions`; game availability is recorded in core `game_bindings`. Neither card ID nor ref ID is used as a save-data key. `sv7_new` is idempotent and creates the profile only once, so logging out and back in does not recreate the profile.
