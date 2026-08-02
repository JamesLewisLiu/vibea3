# AVS2 core 2.17 API audit

This note records the ABI facts checked directly against the loaded
`avs2-core.dll` (`XCgsqzn` export family). `core.cpp` has the correct 2.17.0
and 2.17.3 export-name map, but several declarations in `core.h` are only
approximations and should not be copied into an FFI binding unchanged.

## Property API

- `XCgsqzn0000090` (`property_create`) is
  `property_create(flags, arena, arena_size)`. The arena is aligned up to an
  8-byte boundary and must leave at least 577 bytes after alignment.
- `XCgsqzn00000a1`, `00000a2`, `00000a6`, `00000ab`, `00000ac`, and
  `00000af` match the search, create, traversal, read, write, and refer roles
  assigned by `core.cpp`.
- `XCgsqzn00000b7` (`property_mem_read`) actually has the shape
  `property_mem_read(input, input_size, flags, arena, arena_size) -> property`.
  The `PROPERTY_MEM_READ_T` typedef in `core.h` is wrong.
- `XCgsqzn00000b5` (`property_insert_read_with_filename`) actually creates and
  fills a property in a caller-provided arena:
  `property_insert_read_with_filename(filename, flags, arena, arena_size) -> property`.
  Its typedef in `core.h` is wrong.
- `XCgsqzn0000099` and `XCgsqzn0000098` take buffer pointers. The corresponding
  `uint8_t buffer` parameters in `core.h` must be `uint8_t *buffer` (or `void *`).

## Cstream/LZ77 API

- `XCgsqzn0000130` creates operation `0` (inflate) or `1` (deflate).
- `XCgsqzn0000131` resets an existing stream. It is present in the DLL but is
  missing from the `core.cpp` import table and `core.h` declarations.
- `XCgsqzn0000132` operates and `XCgsqzn0000133` finishes the stream.
  Their results are integer stream states, not safely modeled as C++ `bool`:
  inflate returns `0` when it needs more input/output, `1` after the end token,
  and finish can return `2` for truncated input.
- `XCgsqzn0000134` destroys the stream.
- The public prefix of the object is correctly laid out as output pointer,
  input pointer, output bytes available, and input bytes available at offsets
  0, 8, 16, and 20 on x64.
- Inflate uses a 4096-byte zero-initialized history window, starts at ring
  position `0xFEE`, reads flag bits least-significant-bit first, uses a
  big-endian 16-bit `(distance << 4) | (length - 3)` token, and treats every
  token below `0x10` as end-of-stream. This agrees with the Rust decoder.

## Compatibility implications

The Rust framework does not link this ABI directly. These findings are used as
behavioral tests for its property/kbin and LZ77 implementations. If a native
AVS FFI layer is added later, it should use corrected declarations rather than
including `core.h` verbatim.
