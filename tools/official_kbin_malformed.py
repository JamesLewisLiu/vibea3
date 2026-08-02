"""Differential malformed-kbin probe for the official and Rust decoders."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def be32(data: bytearray, at: int, value: int) -> None:
    data[at : at + 4] = value.to_bytes(4, "big", signed=False)


def main() -> int:
    if len(sys.argv) != 5:
        raise SystemExit("usage: official_kbin_malformed.py BASE HARNESS DLL RUST_DECODER")
    base = bytearray(Path(sys.argv[1]).read_bytes())
    harness = str(Path(sys.argv[2]).resolve())
    dll = str(Path(sys.argv[3]).resolve())
    rust = str(Path(sys.argv[4]).resolve())
    work = Path("target/official-malformed")
    work.mkdir(parents=True, exist_ok=True)

    cases: dict[str, bytes] = {"baseline": bytes(base)}
    for length in range(12):
        cases[f"truncate_{length}"] = bytes(base[:length])
    for at, value in [(0, 0), (0, 0xA1), (1, 0), (1, 0x45), (1, 0x43)]:
        data = base.copy()
        data[at] = value
        cases[f"header_{at}_{value:02x}"] = bytes(data)
    for encoding in [0x00, 0x20, 0x40, 0x60, 0x80, 0xA0, 0x01, 0xFF]:
        data = base.copy()
        data[2] = encoding
        data[3] = (~encoding) & 0xFF
        cases[f"encoding_{encoding:02x}"] = bytes(data)
    data = base.copy()
    data[3] ^= 1
    cases["bad_complement"] = bytes(data)

    schema_len = int.from_bytes(base[4:8], "big")
    for value in [0, 1, schema_len - 1, schema_len + 1, len(base), 0xFFFFFFFF]:
        data = base.copy()
        be32(data, 4, value)
        cases[f"schema_len_{value}"] = bytes(data)
    for value in [0, 1, 35, 37, 0xFFFFFFFF]:
        data = base.copy()
        be32(data, 44, value)
        cases[f"data_len_{value}"] = bytes(data)
    cases["trailing_zero"] = bytes(base + b"\0")
    cases["trailing_junk"] = bytes(base + b"junk")

    for at, value, name in [
        (8, 0x00, "root_type_zero"),
        (8, 0x02, "root_type_u8"),
        (8, 0x3D, "root_type_unknown"),
        (8, 0x41, "root_type_array_node"),
        (9, 0, "root_empty_name"),
        (13, 0x3D, "attribute_unknown_type"),
        (19, 0x3D, "nested_unknown_type"),
        (26, 0x3D, "leaf_unknown_type"),
        (31, 0x3E, "leaf_end_without_high_bit"),
        (31, 0xFF, "leaf_end_file_end"),
        (39, 0x3E, "array_end_without_high_bit"),
        (40, 0x00, "missing_nested_end"),
        (41, 0x00, "missing_root_end"),
        (42, 0x00, "missing_file_end"),
        (42, 0xFE, "file_end_node_end"),
        (43, 1, "nonzero_schema_padding"),
    ]:
        data = base.copy()
        data[at] = value
        cases[name] = bytes(data)
    data = base.copy()
    data[10] = 0xFF
    cases["packed_name_unused_bits"] = bytes(data)
    data = base.copy()
    data[9] = 36
    cases["packed_name_length_36_oob"] = bytes(data)
    data = base.copy()
    data[42] = 0xFF
    data[43] = 0x01
    cases["bytes_after_file_end"] = bytes(data)

    for at, value, name in [
        (48, 0xFF, "attribute_length_huge"),
        (51, 0x00, "attribute_length_zero"),
        (56, 0xFF, "string_length_huge"),
        (59, 0x00, "string_length_zero"),
        (68, 0xFF, "array_length_huge"),
        (71, 0x00, "array_length_zero"),
        (53, 1, "nonzero_data_padding"),
        (66, 1, "nonzero_string_padding"),
    ]:
        data = base.copy()
        data[at] = value
        cases[name] = bytes(data)

    print("case\tofficial\trust\tofficial_error\trust_error")
    for name, payload in cases.items():
        source = work / f"{name}.kbin"
        official_out = work / f"{name}.official"
        rust_out = work / f"{name}.rust"
        source.write_bytes(payload)
        official_out.unlink(missing_ok=True)
        rust_out.unlink(missing_ok=True)
        official = subprocess.run(
            [harness, dll, str(source.resolve()), str(official_out.resolve()), "0x4011", "0", "0", "8"],
            capture_output=True,
            timeout=10,
        )
        rust_result = subprocess.run(
            [rust, str(source.resolve()), str(rust_out.resolve())], capture_output=True, timeout=10
        )
        oe = official.stderr.decode("utf-8", "replace").strip().replace("\t", " ")
        re = rust_result.stderr.decode("utf-8", "replace").strip().replace("\t", " ")
        print(f"{name}\t{official.returncode}\t{rust_result.returncode}\t{oe}\t{re}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
