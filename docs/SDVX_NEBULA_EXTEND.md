# SDVX Nebula `extend/info` reference

This document describes the `extend/info` consumers in the pinned KFC
2026-07-14 `sv7.dll`. It is based on client code, not names guessed from server
implementations.

The most important distinction is:

- `extend_type` selects one of 24 outer tables.
- `param_num_1` frequently selects the behavior inside a table.
- `extend_id` is not a universal subtype. Depending on the table it is an
  item/music/resource identity, an ordering key, a fallback value, a sentinel,
  or completely ignored.

There is consequently no finite list of every valid `extend_id`. Most IDs are
data-defined by the server. The sections below document every hard comparison
and every observed use of the field.

## Wire and client storage

The wire record is:

| Field | Wire type | Client storage |
|---|---:|---:|
| `extend_type` | `u32` | selects table `0..23`; not copied into the table record |
| `extend_id` | `u32` | offset `0x000` |
| `param_num_1` | `s32` | offset `0x004` |
| `param_num_2` | `s32` | offset `0x008` |
| `param_num_3` | `s32` | offset `0x00c` |
| `param_num_4` | `s32` | offset `0x010` |
| `param_num_5` | `s32` | offset `0x014` |
| `param_str_1` | string | offset `0x018`, 1024-byte buffer |
| `param_str_2` | string | offset `0x418`, 1024-byte buffer |
| `param_str_3` | string | offset `0x818`, 1024-byte buffer |
| `param_str_4` | string | offset `0xc18`, 1024-byte buffer |
| `param_str_5` | string | offset `0x1018`, 1024-byte buffer |

Each stored record is `0x1418` (5144) bytes. Each type has physical space for
256 records and a separate count, but `sub_18030FEE0` also caps the input walk
at 256 `info` nodes in total. A server should therefore treat 256 as the total
safe response limit, not promise 256 entries for every type.

Unknown or presently unused types still parse and occupy their table. They do
not fail `sv7_common` merely because this build has no consumer for them.

## Complete outer-type matrix

| Type | Live in this build | `extend_id` role | Primary effect |
|---:|:---:|---|---|
| 0 | no | ignored | Stored, but no direct consumer was found. |
| 1 | yes | category-dependent identity and final sort key | Menu, card-entry, demo, shop ticker, and game-over presentation records. |
| 2 | yes | ignored | Appeal-card generator station definitions. |
| 3 | yes | sheet/reward/config identity, depending on subtype | Stamp, event, reward, and result presentation. |
| 4 | yes | ignored | CSV-driven card-entry/profile presentation tables. |
| 5 | no | ignored | Stored, but no direct consumer was found. |
| 6 | yes | ignored | Global runtime command strings. |
| 7 | yes | ignored | Custom music-select folders and their song expressions. |
| 8 | no | ignored | Stored, but no direct consumer was found. |
| 9 | no | ignored | Stored, but no direct consumer was found. |
| 10 | no | ignored | Stored, but no direct consumer was found. |
| 11 | no | ignored | Stored, but no direct consumer was found. |
| 12 | yes | ignored | Result-bonus and Blaster command strings. |
| 13 | yes | fallback counter/version value | Last-record-only music-select default/forced-selection rule. |
| 14 | yes | panel identity | One total-result extended-event panel. |
| 15 | yes | ignored | Aka-name dictionary overrides. |
| 16 | yes | ignored | Generator/factory goods and inventory presentation overrides. |
| 17 | yes | ignored | Comma-separated integer relation/arena lists. |
| 18 | yes | ignored | Server-defined music/chart presentation groups. |
| 19 | yes | one-shot item identity | Scripted operation total-result entries. |
| 20 | no | ignored | Stored, but no direct consumer was found. |
| 21 | yes | ignored | Paired movie paths. |
| 22 | yes | ignored | Operation-result configuration keyed by `param_num_2`. |
| 23 | yes | `-1` sentinel only | Total-result structured text/body configuration. |

### Stored but inert types

Types `0`, `5`, `8`, `9`, `10`, `11`, and `20` have no direct consumer in
this binary. This conclusion comes from enumerating all references to the
`CExtendGameData` instance and mapping every table-count/base-offset access,
not from the absence of names in the response schema. Their records are still
decoded and stored, so a later client can assign them behavior without a wire
format change.

## Type 1: category-dispatched presentation records

The common accessor filters on `param_num_1`, then sorts ascending by:

1. `param_num_3`
2. `param_num_2`
3. `extend_id`

This means field-name or response order is not the final presentation order.

### Category 1: card-entry/menu unlock panels

- `extend_id` is the displayed/unlocked item identity.
- `param_num_2`, `param_num_3`, `param_str_1`, and `param_str_2` are passed to
  the panel constructor.
- `param_num_5` selects the gate:

| Value | Behavior |
|---:|---|
| 0 | Normal unlock tracking. If missing, the ID is appended to player parameter type `4`, ID `6`. |
| 1 | Requires common gate `7` and the corresponding runtime condition. |
| 2 | Requires the common campaign/catalog entry and its runtime condition. |
| 3 | Requires internal event command `21` (`FACTORY`) and its runtime condition. |
| 4 | Uses the alternate unlock table. If absent, the ID is appended to the next free player parameter type `9` slot. |
| other | The extra gate is skipped and the record remains displayable. |

`param_num_4` and `param_str_3..5` are not consumed by this path.

### Category 3: game-over presentation

- At most 50 records are considered.
- `param_num_4` is the random-selection weight.
- Only records with `param_num_5 == 1` perform the extra common-state gate;
  other values remain eligible.
- `param_str_1..5` are combined into the selected game-over configuration.
- Recognized directives include `base:`, `info_win:`, and `characters:`.

`extend_id`, `param_num_2`, and `param_num_3` do not participate in the
selection performed by this consumer.

### Category 4: Station shop information ticker

Only `param_str_2` is retained. All retained strings are rendered as the
scrolling shop-information list, with the active entry changing every 300
ticks. All numeric fields, `extend_id`, and the other strings are ignored.

### Category 5: demo information entries

- `param_num_2` is copied to the entry.
- `param_num_3` is multiplied by 120 before storage.
- `param_num_4` is copied.
- `param_str_1`, `param_str_2`, and `param_str_5` are used.
- `param_str_3` and `param_str_4` are ignored by this builder.

The presence of category 5 also enables an additional demo/information scene
mode.

### Category 6: single display/layout override

Only the first sorted record is used.

- `param_str_1` is the formatting/template text.
- `param_num_2` and `param_num_3` become global display dimensions/state.
- `param_num_4` is converted to a float by multiplying it by `0.01`.
- `param_str_2` may contain five comma-separated values parsed as
  `%d,%d,%d,%d,%x` for an optional screen rectangle/overlay.

`extend_id`, `param_num_5`, and `param_str_5` are ignored. No caller of category
2, or of categories outside `1,3,4,5,6`, was found in this build.

## Type 2: appeal-card generator stations

Only records with `param_num_1 == 1` are active.

- `param_str_1` is the station label/message.
- `param_str_2..5` define four generator slots.
- `extend_id` and all other numeric operands are ignored.

Each slot uses a compact selector language containing `:`, `/`, `@`, and `-`:

- A terminal number selects one appeal-card ID.
- `low-high` selects an inclusive ID range.
- A number before `:` overrides the returned slot type/price; the default is
  `10`.
- `/value` overrides the appeal-card field stored at card offset `+14`.
- `@value` overrides the appeal-card field stored at card offset `+12`.
- Bit `3` (`value & 8`) of the `@` value is the availability flag checked by
  the client.

The client also hashes/accumulates `param_str_2` from active records to detect
whether the station configuration changed.

## Type 3: stamp, reward, and result subtypes

Here `param_num_1` is the subtype. Several consumers share the same table.

The central parser accepts subtypes `5`, `7`, `8`, and `10` and interprets:

- `extend_id` as the sheet/item identity.
- `param_num_2 % 10000` as a count.
- `(param_num_2 / 10000) % 10` as a variant.
- `param_str_5` as a command string recognizing `imgbg:`, `img:`, `req:`, and
  `boost:`.
- `img:` as two comma-separated operands.
- `req:` as a requirement expression. Its expression language can reference
  another type-3 `extend_id` and player item progress.
- `param_str_1` versus `param_str_2`, and `param_str_3` versus `param_str_4`,
  as language/display alternatives.

Subtype-specific numeric behavior:

| Subtype | Behavior |
|---:|---|
| 5 | `param_num_4` and `param_num_5` are clamped to at least 1 and used as dimensions/counts. |
| 7 | `param_num_3..5` are clamped to at least 1. The client uses player parameter type `4`, ID `7`, and player item type `2`, ID `extend_id`, to maintain dated stock/progress. |
| 8 | `param_num_3` is clamped to at least 0; `param_num_4` selects a presentation code; `param_num_5` is clamped to at least 1. The client builds a date-derived bitmask and removes bits already owned in player item type `2`, ID `extend_id`. |
| 10 | `param_num_4` is clamped to at least 1 and `param_num_5` is forced to 1. |

Other observed subtype consumers are:

| Subtype | `extend_id` use | Effect |
|---:|---|---|
| 1 | highest ID wins in the fallback path | Fallback total-result event configuration; `param_num_2/3` feed the panel. |
| 2 | reward/item ID | Total-result reward entries, sorted by `extend_id` then `param_num_3`; ownership is player item type `2`, ID `extend_id`. |
| 3 | highest ID wins | Skill Analyzer congratulations configuration. Localized message comes from strings 1/2; strings 4/5 are integer lists indexed by skill level. |
| 4 | not used as a selector | Returns `param_str_5` from the first matching record when its feature gate is enabled, otherwise the built-in text. |
| 9 | record/sheet lookup identity | `param_str_1` is a decimal-ID list with `#` grouping. `param_num_3 == 1` adds a player-state rejection gate. |

Records can also be found by exact `extend_id`. General subtype lists sort by
`extend_id` and then `param_num_3`. A subtype-10 record that resolves to
`param_num_5 == 1` is rejected when its extra runtime feature gate is false.

## Type 4: CSV-driven presentation tables

Every record is processed; `extend_id` is ignored. The client uses a robust CSV
scanner that respects quoted commas, Shift-JIS multibyte sequences, and nested
parentheses/brackets.

The exact passes are:

- parse `param_num_2` rows from `param_str_2` once;
- parse `param_num_3` rows from `param_str_3` three times into three internal
  collections;
- parse `param_num_4` rows from `param_str_4` once.

`param_num_1`, `param_num_5`, `param_str_1`, and `param_str_5` are ignored.
The resulting collections are consumed by card-entry/profile-load presentation
code. The three `param_str_3` passes deliberately build distinct internal
views; they are not accidental duplicate parsing.

## Type 6: global command programs

Only `param_str_1..5` are consumed. Every non-empty string is run through the
same command dispatcher; all numeric fields and `extend_id` are ignored.

The parser accepts a direct `:command...` form and a chained
`prefix[condition...]...` form. The registered commands are:

| Command | Arguments and effect |
|---|---|
| `kac:` | Parses a KAC version followed by up to four comma-separated IDs. It replaces the client KAC version/list used by the player KAC eligibility check. |
| `coursemask:` | Parses a course key followed by course IDs. Key `-1` clears the 32-entry mapping table; otherwise pairs are inserted until the fixed table is full. |
| `rankstr:` | Starts with rank bank `0..2`, then alignment `L`, `R`, or another value, followed by numeric operands and text. It replaces one of the three rank-display string/config banks. |
| `ranklimit:` | Requires exactly two integers but stores neither in this build. It is a compatibility validator/no-op. |
| `matchingver:` | Parses one integer and stores its low byte as the matching protocol/version selector used by arena/matching code. |
| `rot2effect:` | Parses a sequence of integer music IDs and marks each through the music metadata mutation routine. |

## Type 7: custom music-select folders

- `param_num_2` is the folder/category ID and must fit below 256. A new record
  replaces/removes an existing folder with the same ID.
- `param_num_4` is a secondary folder integer.
- `param_num_3` is the linked unlock-item ID.
- `param_num_1` is retained as ordering/configuration state.
- `extend_id` is ignored.

`param_num_5` controls availability:

| Value | Gate |
|---:|---|
| 1 | Requires player item type `6`, ID `param_num_3`. |
| 2 | Evaluates the condition encoded by `param_str_3/4`; the client includes the folder when that evaluator returns zero. |
| other | No additional gate. |

Strings:

- `param_str_1` is the primary display text/template. `[img]...` markup is
  stripped when the plain form is needed.
- `param_str_2` is secondary text.
- `param_str_3` is the music/list expression.
- `param_str_4` must provide six comma-separated weights/parameters.
- `param_str_5` is a command expression recognizing a leading space, `b:`,
  `i:`, `g:`, `m:`, `museca`, and `p:`. `museca` imports eligible linked
  MUSECA entries; the other opcodes construct or filter music entries.

Invalid or empty definitions are removed instead of leaving a broken folder.

## Type 12: bonus and Blaster command programs

As with type 6, only the five strings are executed. `extend_id` and all numeric
operands are ignored.

| Command | Arguments and effect |
|---|---|
| `createrbonus:` | Two integers. They are the per-hit reward values used for 1/2/3 qualifying creator-result hits; the client multiplies both values by the hit count. The misspelling is official. |
| `staffbonus:` | Two integers. Same scaling pattern for qualifying staff-result hits, with the staff result label/type. |
| `kacbonus:` | Requires four integers, but this build stores only the second and third. They are multiplied by two separate KAC-result counters and emitted as result bonus types 23 and 24. The first and fourth integers are compatibility operands and are ignored after parsing. |
| `blasterenergy:` | Two integers are required. The first replaces the Blaster-energy multiplier; its default is 10 and the result formula scales by this value divided by 10. The second integer is accepted but ignored. |
| `blasterstr:` | `id,text`; forwards the ID and remaining text to the Blaster/result-label override table. |

## Type 13: last-record music-select rule

This table is live, but only the final record is read. Earlier records are
shadowed by later records.

- `param_num_2 == 0` restricts the rule to a logged-in player;
  nonzero also permits the guest path.
- `param_num_3` is the music ID. The rule is inert if that music does not exist.
- `param_num_4 == 0` selects the catalog/record-availability branch.
- `param_num_4 != 0` selects a counter branch controlled by `param_num_5`.
- In the counter branch, `param_num_5 % 10` selects one of eight player
  parameter counter pairs. `param_num_5 / 10` is the expected generation/key;
  when it is zero, `extend_id` is used as the expected value instead.
- `param_num_4` is also the maximum counter threshold. Once the counter exceeds
  it, the forced selection is no longer applied.

When the selected branch succeeds, the client calls its music-selection setter
for `param_num_3` while `CMusicSelectScene` is loading. `param_num_1` and all
five strings are ignored.

## Type 14: one total-result extended-event panel

Only the first record is used.

- `extend_id` is copied as the panel/event identity.
- `param_num_2 == 0` disables the panel.
- `param_num_3` is a display mode and is valid for `0..2`.
- `param_num_4` is multiplied by 60 and used as a tick/frame duration.
- `param_num_5 == 1` adds the common-state/runtime gate; other values do not.
- `param_str_1` is evaluated as the panel condition/value expression.
- `param_str_2/3` are language alternatives.
- Mode 0 emits no rendered text, mode 1 uses literal text, and mode 2 formats
  the text with the derived value.

`param_num_1`, `param_str_4`, and `param_str_5` are ignored.

## Type 15: aka-name dictionary override

Each record is five independent `(number, string)` pairs:

- `(param_num_1, param_str_1)`
- `(param_num_2, param_str_2)`
- `(param_num_3, param_str_3)`
- `(param_num_4, param_str_4)`
- `(param_num_5, param_str_5)`

For every positive numeric ID, the client inserts a missing aka-name part or
replaces its text when different. `extend_id` is ignored. These strings are
dictionary entries, not join-format templates.

## Type 16: generator/factory goods overrides

The client first loads the built-in common goods definitions, then applies type
16 records.

- `param_num_2` is the `factory_id`/goods subtype byte.
- `param_num_3` is the goods ID.
- `param_num_5 == 1` removes every existing entry matching that goods ID and
  subtype.
- Otherwise `param_str_1..5` are concatenated inside an `<info>` XML wrapper
  and parsed as one definition.
- `extend_id`, `param_num_1`, and `param_num_4` are ignored.

The XML definition recognizes:

- `factory_id`
- `goodsindex`
- `goodsname`
- `imagetexture`
- `reserve`

`imagetexture` is split into directory and final resource name. A parsed entry
is applied only when its identity is present in the server common goods list.

An optional `<display_adj>` child recognizes:

- `include_reserve:u32` as a boolean;
- `inventory_str`;
- `overorder_str`.

## Type 17: integer relation and arena lists

All numeric operands and `extend_id` are ignored.

There are two consumers:

1. The global relation cache splits both `param_str_1` and `param_str_2` on
   commas, parses integers, sorts them, and seeds its relation keys. Derived
   lookups validate keys against music data and are used by several music,
   result, arena, and screen-check paths. Calls are observed for keys `12`,
   `23`, and runtime-selected keys; those are data keys, not type-17 subtypes.
2. The arena room/event-day scene independently flattens `param_str_1` from all
   records into its UI integer list before building `event_day_usr` panels.

`param_str_3..5` are ignored. Because the same `param_str_1` feeds both paths,
servers should not attach unrelated text to this type.

## Type 18: music/chart presentation groups

Only records with `param_num_1 == 0` are processed.

- `param_num_4` is copied to every generated entry as the group/list tag.
- Each of `param_str_1..5` is split on commas.
- Every token is a decimal integer:
  - `music_id = token / 100`
  - `slot_mask = token % 100`
- The mask is tested against the client's five chart slots. The resolved chart
  itself may carry any of the display names `NOVICE`, `ADVANCED`, `EXHAUST`,
  `INFINITE`, `GRAVITY`, `HEAVENLY`, `MAXIMUM`, `VIVID`, `EXCEED`, `ULTIMATE`,
  or `NABLA`.
- Entries are emitted only for charts that exist in music data and whose slot
  bit is set.

`extend_id`, `param_num_2`, `param_num_3`, and `param_num_5` are ignored.

## Type 19: scripted operation total-result entries

- `extend_id == 0xffffffff` disables the record.
- Otherwise `extend_id` is both the operation entry identity and player item
  type `2` identity used for one-shot ownership/state.
- `param_str_1..5` are concatenated and parsed as one structured expression.
- Records whose parsed condition is already satisfied/consumed are omitted;
  remaining records are queued for the `operation_total_result` component.
- A profile-recovery/reset path clears the corresponding player item type `2`
  state for every active type-19 ID.

All five numeric operands are ignored.

## Type 21: movie pairs

For every record:

- `param_str_1` becomes `/data/movie/<param_str_1>`.
- `param_str_2` becomes `/data/movie/<param_str_2>`.
- The pair is retained only when the first path is non-empty and the client
  movie/resource probe succeeds.

`extend_id`, all numeric operands, and `param_str_3..5` are ignored.

## Type 22: operation-result configuration map

Records are keyed by `param_num_2`; a later duplicate replaces the stored
configuration for that key.

- `param_num_3` is clamped to `1..5`.
- `param_num_4` and `param_num_5` are copied unchanged.
- `param_str_1` is a comma-separated command program layered on the built-in
  operation-result definition for the same key.
- `extend_id`, `param_num_1`, and `param_str_2..5` are ignored.

The `param_str_1` tokens are a one-letter opcode followed by a signed decimal
integer:

| Opcode | Structural effect |
|---|---|
| `d` | Replaces the six-bit destination/group mask. The initial mask is `63`. |
| `s` | Clears the current source-ID list and replaces it with the supplied single ID. |
| `g` | For every current source ID and every enabled mask bit `0..5`, records the supplied value in that source/group slot. |

The resulting map drives `operation_result`, bonus display, and related result
eligibility. Player parameter type `12`, ID `20`, stores the currently selected
key; if it is absent or invalid the client falls back to the last available
configured key.

## Type 23: total-result structured text/body records

- `extend_id == 0xffffffff` disables the record. Other ID values are not used
  as lookup keys by this consumer.
- `param_num_1` selects the record mode.
- `param_num_2` is the destination key in the result-data map.

Mode `0`:

- `param_str_1` becomes the destination's base string.
- `param_num_3` becomes its numeric property.

Mode `1`:

- `param_str_1..5` are concatenated and parsed as one structured script.
- Valid parsed elements are appended under destination key `param_num_2`.
- The parser recognizes structured `text`, `music`, and `body` elements and
  accepts normal scalar/object/array values used by the client's result model.

Other `param_num_1` values have no action. `param_num_4` and `param_num_5` are
ignored.

## Server-authoring rules

- Do not treat `extend_id` as the subtype. Dispatch on `extend_type`, then on
  the exact numeric operand documented for that type.
- Keep the entire response at or below 256 `extend/info` records for this
  client build.
- Preserve signed values. `-1` is a real sentinel in types 19 and 23.
- Do not populate ignored operands with accidental control values and assume
  they are harmless across future builds. Prefer zero for fields unused by the
  targeted client.
- For type 13, remember that only the last record wins.
- For types 6 and 12, commands may be spread across any of the five strings and
  execute in record/string order.
- For string-script types, splitting a payload across the five fields is only
  valid where the consumer explicitly concatenates them (types 16, 19, and
  type-23 mode 1). Other types treat the fields independently.

## Principal client evidence

These addresses are for the pinned KFC 2026-07-14 binary and make the findings
above reproducible in IDA:

| Area | Function |
|---|---|
| wire parser and table storage | `sub_18030FEE0` |
| post-`sv7_common` initialization | `sub_18060EEF0` |
| type 1 accessor/sort | `sub_1805C8070`, `sub_1805C81D0` |
| type 2 station selectors | `sub_1802CE8B0`, `sub_1805DA540`, `sub_1805DA6A0` |
| type 3 central parser | `sub_180586FE0`, `sub_180587A80` |
| type 4 CSV tables | `sub_180135820` |
| type 6 command execution | `sub_1805BD8C0` and callbacks `sub_1805BDB30..sub_1805BE360` |
| type 7 folder builder | `sub_1803157A0`, `sub_1803143A0` |
| type 12 command execution | `sub_1805BE410` and callbacks `sub_1805BE1C0..sub_1805BE310` |
| type 13 last-record rule | `sub_1804EFA40` |
| type 14 event panel | `sub_1802E3190` |
| type 15 aka-name overrides | `sub_1805AD490` |
| type 16 goods overrides | `sub_180317DD0`, `sub_1803177B0`, `sub_180318790` |
| type 17 relation cache | `sub_180526D20`, `sub_180527560` |
| type 17 arena consumer | `sub_1804D0A70` |
| type 18 music/chart groups | `sub_180133770` |
| type 19 operation-result scripts | `sub_18058A7E0`, `sub_1804C09A0` |
| type 21 movie pairs | `sub_180216AF0` |
| type 22 operation configuration | `sub_18021BC30`, `sub_18021B600` |
| type 23 result data | `sub_180114680` |
