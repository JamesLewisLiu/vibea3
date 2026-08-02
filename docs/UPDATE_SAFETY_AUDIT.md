# Update-safety audit

This audit records the repository-wide rules applied after the M39 catalog extraction. The executable, framework crate, proc-macro crate, module ABI, environment variables, and administrative API now use the `vibea3` identity as a clean break; no legacy aliases or compatibility exports remain.

## Mutable content assumptions removed

- `InfoData` does not validate the number of songs, phases, news entries, rankings, goods, areas, events, licenses, tracks, or characters.
- Score reads and played-music queries have no release-specific database row limit.
- Music writes and extra data use membership in the loaded `music.json` catalog instead of a compiled maximum ID.
- Character, item, basket, and rival collections are not truncated to the current release's observed item count.
- Tests assert stable anchor records and license-bit behavior without freezing the catalog's total count or last ID.

## Fixed protocol shapes retained

The constants in `modules/popn_highcheers/src/protocol.rs` are fixed client destination buffers or packet shapes recovered from `popn.dll`. They include history arrays, NetVS arrays, seven playable sheets, fixed text buffers, lobby capacities, supplement text buffers, and recommendation output capacity. These constants may normalize a wire value but must never be reused as catalog cardinalities or ID bounds.

The core codec names kbin node, attribute, type-mask, and array-flag values. Name-packing, transport-header, RC4, datecode, card-number, and module ABI widths likewise use descriptive constants at their narrowest shared scope.

## Data-driven license behavior

`music.json` contains the extracted catalog and source SHA-256. On load, `license_music` is derived from the audited M39 `mtype` license-required bit. `license_music_new` remains operator data because the client treats it as presentation state rather than the primary playability gate.

## Review harness

The root `AGENTS.md` requires every later change to search for hard-coded item counts, upper ID bounds, truncation, resize operations, database limits, and raw masks. Every occurrence must be classified as mutable content, fixed wire shape, a resource-safety policy, or an algorithm invariant before it is retained.
