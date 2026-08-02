# Repository coding rules

These rules apply to every change in this workspace.

## Mutable game data

- Never treat a collection's item count as a schema invariant. Game updates may add or remove songs, characters, tracks, phases, news, rankings, events, goods, licenses, or other content records.
- Do not reject, truncate, pad, or cap a data collection unless the limit is an audited wire-format or client-memory invariant. Document the source of every such invariant next to its named constant.
- Prefer IDs and membership loaded from the versioned data files over compiled numeric ranges. Validation may enforce per-record shape, fixed field widths, encoding, uniqueness, and internally consistent references.
- Tests may assert known anchor records, but must not freeze a complete mutable item count or assume that the current last ID remains last.

## Constants and provenance

- Do not introduce unexplained numeric or string literals in production logic.
- Give every nontrivial protocol value, bit mask, field width, timeout, capacity, status code, model/date gate, path, collection name, and algorithm threshold a descriptive constant.
- Put constants at the narrowest shared scope that avoids duplication. Add a short provenance comment when the meaning is not obvious from protocol documentation or audited client behavior.
- Ordinary control values such as `0`, `1`, empty values, and loop increments are exempt when their meaning is self-evident.

## AVS property-map schemas

- Never treat a `property_psmap.type` byte as a kbin/property type ID. Recover its wire type through the audited `property_psmap_import` mapping in `docs/AVS2_PSMAP_TYPES.md`.
- Psmap `0x0a` is a fixed-capacity C string destination and maps to wire `str` (`0x0b`), not wire `bin` (`0x0a`). The capacity includes the terminating NUL; validate payload text against `capacity - 1` encoded bytes.
- Psmap array codes are a separate range beginning at `0x43`. Its `buffer_size` is the element count; multiply by the mapped wire element width only when reasoning about destination bytes.
- A psmap path containing `#N` selects the Nth repeated node. Do not serialize the `#N` suffix as part of the field name.
- For every psmap-derived schema change, add a kbin round-trip or encoded-type regression test covering the corrected field.

## Kbin resource limits

- Do not infer a protocol field-width limit from a decoder safety default. Official property trees can exceed 65,535 nodes; `DecodeOptions.max_nodes` is a configurable resource guard, not a u16 wire field.
- Kbin variable-value lengths and section lengths are u32. Reject only values that cannot be represented by u32 or that exceed an explicit configured resource policy.
- When changing a codec bound, test both the smallest value above the old bound and a real official large-tree fixture.

## Reverse-engineered XRPC modules

- A route inventory must be derived from the target client registration table and asserted exactly in a unit test.
- Request and response structs must include every observed field. Conditional fields are `Option<T>`; repeated nodes and property arrays must use explicit `#[kbin(repeated)]` and `#[kbin(array)]` annotations.
- Audit request builders and response consumers separately. A successful HTTP/XRPC status does not prove the client receiver accepted the body; required empty container nodes must be emitted when the consumer treats absence as failure.
- Persist player data by the core global user/data ID. Card IDs and ref IDs are authentication/session handles, never game-save primary keys.

## Review checklist

- Search the changed area for hard-coded item counts, upper ID bounds, `.take(...)`, `resize(...)`, database query limits, and raw hexadecimal masks.
- Classify each occurrence as mutable content, fixed wire shape, safety/resource policy, or algorithm invariant.
- Remove mutable-content assumptions. Name and document the remaining invariants.
- Add tests for data-driven behavior and at least one known anchor value without freezing the whole release dataset.
