# AVS2 property psmap type mapping

This mapping was recovered from M39 `avs2-core.dll`, SHA-256
`862504d420b3ed42c950fc2297f467f7f3558814f6ee8694fc4ed0c23834af5d`.
The importer is export `XCgsqzn00000b2`; the exporter is
`XCgsqzn00000b3`. Both dispatch through the 124-byte lookup table at
`0x1800b4b40` in this build.

The psmap enum is not the AVS property/kbin enum. Most integer codes happen to
match, but strings, attributes, booleans, vectors, and every array code do not.

## Entry control values

| Psmap | Meaning |
|---:|---|
| `0x00` | invalid |
| `0x01` | nested node; `d` points to another psmap |
| `0x42` | invalid/reserved separator |
| `0xff` | end of map |

## Scalar and fixed-vector values

| Psmap | Wire ID | Wire type |
|---:|---:|---|
| `0x02` | `0x02` | `s8` |
| `0x03` | `0x03` | `u8` |
| `0x04` | `0x04` | `s16` |
| `0x05` | `0x05` | `u16` |
| `0x06` | `0x06` | `s32` |
| `0x07` | `0x07` | `u32` |
| `0x08` | `0x08` | `s64` |
| `0x09` | `0x09` | `u64` |
| `0x0a` | `0x0b` | `str` |
| `0x0b` | `0x0c` | `ip4` |
| `0x0c` | `0x0d` | `time` |
| `0x0d` | `0x0e` | `float` |
| `0x0e` | `0x0f` | `double` |
| `0x0f..0x2c` | `psmap + 1` | `2s8` through `4d` in normal property order |
| `0x32` | `0x34` | `bool` |
| `0x33` | `0x35` | `2b` |
| `0x34` | `0x36` | `3b` |
| `0x35` | `0x37` | `4b` |
| `0x37` | `0x30` | `vs8` |
| `0x38` | `0x31` | `vu8` |
| `0x39` | `0x32` | `vs16` |
| `0x3a` | `0x33` | `vu16` |
| `0x3b` | `0x28` | `vs32` (`4s32` wire representation) |
| `0x3c` | `0x29` | `vu32` (`4u32` wire representation) |
| `0x3d` | `0x16` | `vs64` (`2s64` wire representation) |
| `0x3e` | `0x17` | `vu64` (`2u64` wire representation) |
| `0x3f` | `0x2c` | `vf` (`4f` wire representation) |
| `0x40` | `0x19` | `vd` (`2d` wire representation) |
| `0x41` | `0x38` | `vb` |

There is no ordinary psmap code that maps to wire `bin` (`0x0a`). In
particular, psmap `0x0a` copies a NUL-terminated wire string into the fixed C
destination described by `buffer_size`.

## Attribute conversions

These codes create/read an AVS attribute (`0x2e`) as decimal text and convert
between that text and the C destination:

| Psmap | C destination | Export format |
|---:|---|---|
| `0x2d` | character buffer | direct attribute string |
| `0x2e` | `u32` | `%u` |
| `0x2f` | `s32` | `%d` |
| `0x30` | `u64` | `%lu` |
| `0x31` | `s64` | `%ld` |
| `0x36` | `bool` | `%u` (`0` or `1`) |

## Arrays

Psmap arrays are not formed by applying the kbin array bit to a scalar psmap
code. The importer maps the psmap code to a scalar wire element type, then
multiplies `buffer_size` (the element count) by that type's byte width.

| Psmap range | Wire element mapping |
|---:|---|
| `0x43..0x4a` | `s8[]` through `u64[]` (`0x02..0x09`) |
| `0x4b..0x6c` | `ip4[]` through `4d[]` (`0x0c..0x2d`) |
| `0x6d..0x70` | `bool[]`, `2b[]`, `3b[]`, `4b[]` |
| `0x71..0x7b` | vector aliases `vs8[]` through `vb[]`, using the same wire representations as scalar vectors |

String collections are represented as repeated `str` nodes. Psmap paths such
as `dialog#3` select an occurrence; `#3` is not serialized in the node name.

## M39 audit consequences

- Account `g_pm_id` and `name`, NetVS `dialog`, news text, supplement text,
  event serial codes, lobby IDs, and player/course names are wire `str`.
- The 13/45/64/128/etc. psmap sizes are destination capacities including the
  terminating NUL, not binary payload lengths.
- Lobby and course `license_data` are `s16[20]`: 20 elements and 40 destination
  bytes. The earlier `s16[10]` interpretation incorrectly treated the psmap
  element count as a byte count.
