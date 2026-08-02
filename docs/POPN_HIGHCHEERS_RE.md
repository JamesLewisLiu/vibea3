# popn_highcheers client audit

This note records facts verified against the High Cheers client rather than assumptions inherited from older pop'n servers.

## Target

- Client: `D:\M39\contents\modules\popn.dll`
- IDA database: `D:\M39\contents\modules\popn.dll.i64`
- SHA-256: `f594d5f4ca1f121c58e134652cbf1b09b81a8270a8c20a093e2f621e494f5a9e`
- Image base: `0x180000000`
- Module gate: product `M39`, datecode `>= 20251218`, no upper bound
- Services: `local2`, `local3`, `lobby2`

## Exact route inventory

The client constructs 26 routes:

- `info.common`
- `lobby24.getList`, `lobby24.entry`, `lobby24.update`, `lobby24.delete`
- `loctest24.log`
- `pcb.boot`, `pcb.error`, `pcb.dlstatus`, `pcb.write`
- `player.new`, `player.conversion`, `player.read`, `player.write`
- `player.start`, `player.logout`, `player.end`, `player.delete`
- `player.write_music`, `player.read_score`, `player.read_option`, `player.buy`
- `player.friend`, `player.write_course`, `player.update_ranking`, `player.tsumtsum`

## AVS property maps

`XCgsqzn00000b2` is `property_psmap_import`; `XCgsqzn00000b3` is the exporter. An x64 map entry is 24 bytes:

```text
u8 type | u8 have_default | u16 offset | i32 buffer_size | char *path | void *default
```

Type `0xff` terminates the map. A missing no-default entry makes import fail. Default values can be immediate integers stored in the pointer-sized default slot.

The psmap `type` byte is a separate enum from the AVS property/kbin type ID. It must be interpreted through `property_psmap_import`, never copied into a packet schema. In particular, the account map uses psmap code `0x0a` for `g_pm_id` and `name`, but the importer reads both from wire `str` properties; encoding them as kbin type `0x0a` (`bin`) makes the client reject `player.new`. The complete recovered mapping is in [AVS2_PSMAP_TYPES.md](AVS2_PSMAP_TYPES.md).

Paths such as `gpm_id_list#0` and `dialog#3` are psmap occurrence indexes. They are represented on the wire as repeated nodes named `gpm_id_list` or `dialog`; the `#N` text is not part of the XML/kbin field name.

Verified fixed-array psmap codes are `0x43=s8[]`, `0x44=u8[]`, `0x45=s16[]`, `0x46=u16[]`, `0x47=s32[]`, `0x48=u32[]`, and `0x6d=bool[]`. These codes cannot be decoded by simply masking an “array bit” from the scalar code.

Fixed destination buffers confirmed in M39 include the following. These are psmap copy capacities, not kbin property type IDs; every listed textual field is wire `str`:

- account `g_pm_id:str` and `name:str`, copied into 13-byte destinations
- lobby `gpm_id_list:str`, six indexed occurrences with 13-byte destinations
- news `title:str` and `main:str`, copied into 45-byte and 1024-byte destinations
- event `ensta/serial_code:str`, copied into a 64-byte destination
- lobby and course `license_data:s16[20]` (40 bytes)

## Common data

`info.common` accepts required string attribute `loc_id`. Its response parser treats every dataset as optional. It recognizes `phase`, `news`, `ranking_info`, `area`, `choco`, `goods`, `fes`, `popular`, `popular_music`, `recommend`, `mission_point`, `medal`, `chara_ranking`, `musicsub`, `tracksub`, `charasub`, `license_music`, and `license_music_new`.

The verified implemented schemas are:

- `phase`: `event_id:s16`, `phase:s16`; M39 accepts event IDs 0 through 7.
- `news`: `no:s16`, `type:u8`, `image_no:s16`, `title:str`, `main:str`.
- `goods`: `item_id:s32`, `item_type:s16`, `price:s32`, `goods_type:s16`.
- `musicsub` fixed arrays: `lv:u8[7]`, `track:u16[7]`, `tag_list:u16[32]`, `bpm_min:s16[7]`, `bpm_max:s16[7]`, `long:s8[7]`.
- `charasub/loc_btl_bmp:s16[2]`.

Event ID 3 is the network battle switch. The client additionally forces its phase to zero when `portfw/globalip` is unavailable. The module advertises phase 1 for event ID 3 and leaves unverified content phases absent.

## Lobby

`lobby24.entry` requires:

```text
ip:u32 local_ip:u32 time:u32 port:u16 music:s16 sheet:u8 is_ojama:u8
location_id:str net_version:u8 gpm_id:str staff:s8 item_type:s16 item_id:s16
is_random:s8 license_data:s16[20] is_ranking:s8
```

`time` is a lifetime in milliseconds. The response requires `no:u32`.

`lobby24.getList` requires `location_id:str` and `net_version:u8`. The client reads at most 30 repeated `list` records, each 156 bytes. A list record contains:

```text
no:u32=0 time:u32=0 ip:u32=0 port:u16=0 local_ip:u32=0
music:s16=-1 sheet:u8=255 is_ojama:u8=0
gpm_id_list:str occurrences 0..5
matching_num:u8=0 staff:s8=0 item_type:s16=-1 item_id:s16=-1
is_random:s8=0 license_data:s16[20]=0 is_ranking:s8=0
```

`lobby24.update` requires `room_no:u32`, `matched_cnt:u8`, `location_id:str`, `gpm_id:str`, and `staff:s8`. It refreshes the room and contributes the supplied ID to the six-entry participant list. `lobby24.delete` requires `no:u32`.

## Player profile

`player.read` requires `ref_id:str`, `data_id:str`, and `pref:s8`. `result:s8` is mandatory:

- `result == 0`: full profile.
- `result == 1`: conversion response with `name:str`, `chara:s16`, `con_type:s8`, and at most 13 repeated `medal_cnt { clear_type:s8, cnt:s16 }`.
- Other values stop profile import.

The full parser imports sections in this order: account, info, config/option, item, character parameters, customize, netvs, course data, extra data, and event data. Optional sections may be absent, but account, info, customize, and the netvs node are required for a successful full profile import.

### Account

Required:

```text
g_pm_id:str name:str tutorial:s16 read_news:s16 is_conv:s8
active_fr_num:u8 total_play_cnt:s16 latest_music:s16[30]
nice:s16[100] favorite_chara:s16[100] power_point:s32
power_point_list:s32[20]
```

The optional/defaulted account fields are `popn_class:s8=-1`, `option_tuto:bool=false`, `sc_news_no:s32=-1`, `read_policy:s16=0`, and `language:s8=-1`. An optional sibling `eaappli { relation:s8 }` is also recognized.

The account write map additionally contains `play_id:s32`, `start_type:s8`, `popn_class`, `sc_news_no`, `read_policy`, and `language`; these are explicit in the Rust request schema even when the handler does not need the transient play fields.

`player.write` also carries zero or more repeated `stage` records. M39 exports the
complete record as:

```text
no:s16 sheet:u8 clear_rank:u8 clear_type:u8 score:s32
cool:s16 great:s16 good:s16 bad:s16 combo:s16 highlight:s16 gauge:s16
gauge_type:s8 is_win:s8 matching:s8 ojama_vs:s8
```

The module persists these end-of-play records in `popn29_plays`, keyed
by global user ID and `play_id`. They are not used to increment popularity a
second time because `player.write_music` already accounts for each chart play.

### Card/profile binding

`cardmng.inquire` may issue a fresh session `refid` whenever `update=0`; that is
normal and does not identify the saved game profile. `dataid` remains the global
user ID. The client chooses the existing-profile path from `binded`, not from
whether the session ID is stable.

Bindings are stored in the generic `game_bindings` collection by model and
global user ID. Successful `player.new`/`player.conversion` profile creation
registers `M39`, and module startup backfills bindings for profiles created by
older Vibea3 builds. The earlier card-local `bound_models` list was incorrect:
`cardmng.bindmodel` occurs before the game profile is created and cannot itself
prove that profile data exists.

### Info, customize, and options

- Info requires `ep:u16`; `estatus:u16` defaults to 0.
- Customize requires twelve `u16` values: `seal_0` through `seal_6`, `seat`, `touch_th`, `lane_cover`, `stage_bk`, `highlight`.
- Config requires mode/chara/music/sheet and the eight category/banner/display fields. Defaults are `h_vol=1`, `hiscore_disp=false`, `lane_type=0`, `brightness=100`, `key_beam=100`, `lane_line=1`.
- Option defaults include `hidden_rate=-70`, `sudden_rate=-270`, and `guide_se_vol=3`; all other fields are persisted exactly with their signedness.

### Collections

- `item`: `type:u8`, `id:u16`, `param:u16`, `is_new:bool`, `get_time:u64`.
- `chara_param`: `chara_id:u16`, `friendship:u16`; this DLL's destination has 739 observed slots. The service deliberately does not treat that release-specific count as an update-stable cap. The read parser also accepts the optional legacy repeated branch `chara_param_old` with the same element schema.
- `ex_info`: `music_num:s16`, `point:u8`, `ex_gauge_lv:u8[4]`, `clear_lv:u8[4]`; this DLL contains normal music IDs 0..2373, while the service validates membership against the loaded catalog.

### NetVS

Read map:

```text
record:s16[6] dialog:str repeated up to 6 ojama_condition:s8[74]
set_ojama:s8[3] set_recommend:s8[3] netvs_play_cnt:u32
```

Every map field has a default, but the `netvs` node itself is required. The write map exports `ojama_condition` as a comma-separated string, repeated `rival_id` capped at 30, the three arrays, two selection counters, and the play counter.

### High Cheers event

The wire node is `event_p29`.

- Root: optional `basket_id:s16`, repeated `basket`, optional `ensta`.
- Basket: `id:s16`, `point:u32`, `is_cleared:bool`; accepted IDs are 1 through 22.
- Ensta write exports only `checked:bool`.
- Ensta read, when present, requires `serial_code:str` and `checked:bool`.

The whole event node is optional. Its absence is distinct from a malformed present node.

## Music, course, and social calls

- `player.read_score` returns optional repeated `music` records: `music_num:s16`, `sheet_num:u8`, `score:s32`, `clear_type:u8`, `clear_rank:u8`, `cnt:s16`, `score_ver:s32`, `clear_type_ver:u8`.
- `player.read_option` accepts absence of the `option` node. A present node uses the complete option map.
- `player.write_music` exports fixed-capacity string identity fields, chart/result counters, the complete option state, timing data, NetVS result data, and optional `my_graph:u8[]`. Guest requests have an empty `ref_id` and are acknowledged without persistence.
- `player.write_course` exports four stages, norms, result metrics, and optional `license { is_license:bool, license_data:s16[20] }`.
- `player.update_ranking` recognizes optional `all_ranking`, `pref_ranking`, and `location_ranking` nodes. Each contains `name:str`, `chara_num:s16`, `total_score:s32`, `clear_type:u8`, `clear_rank:u8`, `player_count:s16`, and `player_rank:s16`.
- `player.friend` has an optional `friend` node. `con_flg:bool` is inside that node; when true, the node also carries the complete base friend map, repeated attribute-style music records, and repeated course records.
- `player.tsumtsum` accepts an absent body; a present body reads `status:s8`.
- `player.buy`, `player.logout`, `player.end`, and successful writes accept empty response bodies.

## PCB and location test

`loctest24.log` contains a required `locTest` node with mode, character, stage count, location, and repeated stage results. Each stage includes chart identity, score/judgements, gauge/combo, full option state, and two ojama slots.

`pcb.error` carries `loc_id:str`, `code:str`, `scene:s8`, and `info:str`. `pcb.dlstatus` carries `lid:str` and `prg:s32`.

`pcb.boot` sends the complete root status map: `loc_id:str`, `loc_type:u8`, `loc_name:str`, `country:str`, `region:str`, `pref:s16`, `customer/company:str`, `gip:ip4`, `gp:u16`, `rom_number:str`, four `u64` drive values, `os_act:str`, and `etc:str`. A captured `M39:J:D:A:2026041500` boot packet established these wire types; the earlier static-analysis notes that described fixed-size `bin` values and a string `gip` were incorrect.

`pcb.write` requires nested `pcb_status`, the complete 32-field `pcb_setting`, three repeated `vc_setting` records, `pcb_card`, and `dlstatus:s32`. Its fixed settings arrays are `schedule:s8[8]`, `sw_cnt:u32[18]`, `c_enbl_w:bool[7]`, and `ch_w/cm_w:s16[7]`. High Cheers only consumes an acknowledgement; the module additionally persists the supplied fixed-capacity string cabinet name.

## Runtime common-data source

The module loads `info.json` and `music.json` during initialization and refuses publication when either is missing or invalid. Resolution order is `POPN_HIGHCHEERS_DATA_DIR`, then `$VIBEA3_DATA_DIR/popn_highcheers`, then `data/popn_highcheers`. `info.json` contains only server-operated common data and supplement overrides; the full client reference catalog lives separately in `music.json`.

### M39 embedded music table

The checked-in `music.json` catalog is extracted from the exact M39 `popn.dll` with SHA-256 `f594d5f4ca1f121c58e134652cbf1b09b81a8270a8c20a093e2f621e494f5a9e`. The hash is stored in the file and verified at module initialization. The reproducible extractor is `tools/extract_popn_m39_music.py`.

The normal catalog covers music IDs 0 through 2373. The validator also admits `-85..-1`, but those are pseudo-selector/system values: regular metadata accessors reject them and selected display-name lookups route through a separate table. They are not contiguous 312-byte music records and are deliberately excluded from `music_catalog`.

Each normal embedded record is 312 bytes and contains seven CP932 string pointers, character IDs, a 32-bit type mask (zero-extended into the RPC's `u64`), version fields, seven levels, seven track IDs, hariai data, 32 tags, two seven-entry BPM arrays, and seven long-note flags. The RPC `musicsub` schema exports six of the seven strings; the omitted pointer is a private Romanized genre-sort alias. Observed normal records reference track IDs through 2489 and character IDs through 2506, so neither field is incorrectly capped at the music count or friendship-array size.

The embedded table is loaded from `music.json`, not copied into `music_supplements`. This distinction is required: built-in strings can exceed the RPC supplement field widths (one observed `title_sort` is 134 CP932 bytes while the wire field is 128 bytes). Echoing the built-in table as supplements would truncate valid client data. `music_supplements` remains reserved for genuine server-provided overrides that satisfy the audited fixed-width wire contract.

`sub_180094210` is the actual playability gate. Songs with `mtype & 0x20000 != 0` must appear in the server-provided `license_music` array; songs without that bit bypass the list. The module derives the array from `music.json` on every load. This produces 18 M39 IDs, including `2272` (BOW AND ARROW), `2290` (アイドル), and `2319` (踊). `license_music_new` is checked separately by `sub_180094280` for new-song presentation and is intentionally left as operator-controlled common data rather than confused with the unlock list.

Server-operated phases, news, shop/event tables, rankings, and licenses are not contained in the DLL music database. Their checked-in values are therefore empty instead of fabricated.

## Deliberate bounds and malformed-input behavior

- Lobby results: 30; participant IDs: 6.
- Scores: this DLL contains 2,374 songs and seven sheet slots, but the service does not impose the resulting release-specific row count as a database limit.
- Items: 4,096 server safety cap.
- This DLL's observed destination counts are 739 characters, 2,374 extra records, 22 event baskets, and 30 rivals. They are reverse-engineering observations, not server-side mutable-content limits.
- Fixed arrays are truncated and padded to their client destination sizes.
- Unknown fields are skipped by the schema VM. Missing required Rust schema fields fail decoding before handler execution.
- Missing card sessions produce the same no-profile/conversion response as an unknown profile; there is no PCBID or machine-document gate.
