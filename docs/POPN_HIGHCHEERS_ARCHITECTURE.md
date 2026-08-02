# popn_highcheers architecture

## Scope

`popn_highcheers` is a hot-reloadable XRPC module for product `M39` from datecode `20251218` onward. The bootstrap remains schema-free: it owns the MongoDB connection and generic document operations, while this module owns collections, indexes, models, limits, and game behavior.

The module exports all 26 handlers through compile-time registration. It has no manual route list and one DLL may serve the `info`, `lobby24`, `loctest24`, `pcb`, and `player` classes.

## Source layout

```text
modules/popn_highcheers/src/
  lib.rs                 module gate and initialization
  info.rs                common data
  lobby.rs               lobby wire schemas and handlers
  pcb.rs                 compact PCB orchestration/error handlers
  pcb/                   boot, write, and location-test wire schemas
  player.rs              player orchestration only
  player/                compact wire-schema/state files
  database.rs            collection ownership and initialization
  database/model.rs      MongoDB models
  database/player.rs     profile/score/course persistence
  database/usage.rs      popularity counters and recommendations
  database/lobby.rs      expiring lobby persistence
```

No source file contains the whole service or a generated enterprise-style layer stack. Wire structs stay next to the behavior that uses them.

## Identity flow

The core module creates a short-lived card session mapping:

```text
ref_id -> card_sessions.user_id -> popn29_players._id
```

High Cheers data is keyed by the global `dataid`/`user_id`, never by a physical card ID or rotating `ref_id`. A session validates that the request's claimed `dataid` belongs to the authenticated user. Multiple cards may point at the same user and therefore share one profile, score set, inventory, and course history. A profile receives a unique 12-digit `g_pm_id` from an atomic MongoDB counter. Missing or mismatched sessions are not transport errors and no machine/PCBID registration is required.

Module initialization removes the obsolete `card_id` score/course indexes before creating the `user_id` replacements. Old card-keyed game documents are intentionally not read or migrated.

## Collections

| Collection | Key and purpose |
| --- | --- |
| `card_sessions` | Core-owned `refid -> user_id` authentication lookup; read only by this module. |
| `game_bindings` | Core-readable `{model}:{user_id}` marker created only after a game profile actually exists. |
| `popn29_players` | `_id = user_id`; profile, options, collections, NetVS, event state, and lumina. |
| `popn29_scores` | `_id = user_id:music:sheet`; best score, clear state, count, and per-chart option. |
| `popn29_courses` | `_id = user_id:course_id`; complete four-stage submission, norms, license data, and result. |
| `popn29_plays` | `_id = play sequence`; complete `player.write/stage` records for play history and analysis. |
| `popn29_rooms` | Numeric room ID, participant IDs, network fields, and expiration. |
| `popn29_cabinets` | Optional observed cabinet name by `srcid`; never gates play. |
| `popn29_errors` | Client-reported PCB error telemetry. |
| `popn29_counters` | Atomic `g_pm_id`, play ID, and lobby room sequences. |
| `popn29_music_usage` | Global per-song play counters used for popular and recommended music. |
| `popn29_character_usage` | Global per-character selection counters used for character popularity. |

Indexes are created by module initialization: unique player ID, unique chart, unique player/course, lobby query order, lobby expiration lookup, and descending usage counters.

## Profile writes

`player.write` is a patch for scalar sections but a snapshot for repeated collections. Only fixed client destination arrays are normalized to their audited widths. Music references are checked against the loaded catalog; mutable item, character, basket, and rival collections are not capped by the current release's cardinality. Event data is isolated under `event_p29`, which permits later High Cheers event changes without changing the core framework.

New profiles use the defaults observed in the M39 property maps, including brightness/key-beam 100, volume/lane-line 1, hidden rate -70, sudden rate -270, and guide volume 3.

Wire schemas retain every audited optional compatibility branch (`eaappli`, `chara_param_old`, friend/ranking variants, and `my_graph`) rather than hiding unsupported fields outside the Rust type. Content for `info.common` and `player.start` is loaded once from the module data directory into immutable shared state.

## Score behavior

Chart identity is `(user_id, music_num, sheet_num)`. Writes for songs absent from the loaded catalog or sheets outside the fixed seven-chart wire shape are acknowledged but ignored. Guest writes with an empty reference ID are also acknowledged without storage.

For valid players, play count saturates at `i16::MAX`, the best numerical score is retained, clear type/rank retain their maxima, and chart options change only when `is_option_save` is true. Reads are sorted by music and sheet without a release-specific row limit.

Each accepted score write increments module-owned music and nonnegative character usage documents. Character IDs are not capped by the current release's character table. `info.common` returns the global top 500 songs, top 20 characters, and top 30 songs as recommendations. `player.start` uses the same global ranking but removes songs already played by that card before constructing its fixed 30-entry recommendation array. With no recorded usage, all three sections are omitted/empty; no sequential fallback IDs are invented.

The immutable catalog in `music.json` is extracted from the client’s built-in M39 table. `info.json` remains compact and contains only common-response configuration and explicit supplement overrides. The catalog is service-side reference data and is not emitted as `musicsub`, because the built-in representation contains strings wider than the supplement protocol can carry.

License playability is derived, not hand-maintained: every catalog record whose type mask contains `0x20000` is included in `license_music`. The M39 catalog yields 18 IDs. `license_music_new` remains an optional configured subset because the client uses it for new-song treatment, not for the primary playability gate.

Data validation never treats collection cardinality as a schema invariant. Game updates may add common-data records or catalog entries without requiring a Rust code change. Validation is limited to per-record structure, fixed-width protocol fields, nonnegative/unique IDs, and catalog provenance syntax.

## Lobby behavior

The `time:u32` request value is interpreted as the client-supplied lifetime in milliseconds. Expired documents are removed opportunistically before list queries; the expiry index makes cleanup cheap without requiring bootstrap-specific TTL features.

The request `tag` is stored and used only to keep a client’s own room out of its list. It is matching metadata, not machine authorization. Updates refresh expiry, update the matched count/staff fields, and add a distinct participant ID up to the six IDs the client can import.

## Failure policy

- Malformed required wire data is rejected by the compiled schema VM.
- Database failures become XRPC status 1 with a short `database_error:` description.
- Unknown profiles and missing card sessions return the client’s conversion/no-profile result rather than a transport error.
- Telemetry-style methods are acknowledged even when the service deliberately does not retain their full payload.
- Module initialization fails before publication if required indexes cannot be established, so hot reload keeps the previous working generation.

## ABI note

This module needs `find_many`, `delete_one`, and `delete_many` from `DocumentDatabase`. The host contract fingerprint is therefore `document-api-v2`. Bootstrap and all dynamic modules must be rebuilt together; the loader rejects an older DLL before initialization rather than allowing an ABI mismatch.
