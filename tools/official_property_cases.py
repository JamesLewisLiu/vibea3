"""Run isolated XML corner cases against the official AVS property parser."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def xml(body: str, encoding: str = "UTF-8") -> bytes:
    document = f'<?xml version="1.0" encoding="{encoding}"?><root>{body}</root>'
    codec = {
        "SHIFT_JIS": "cp932",
        "EUC-JP": "euc_jp",
        "ISO-8859-1": "latin1",
        "ASCII": "ascii",
    }.get(encoding, "utf-8")
    return document.encode(codec)


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: official_property_cases.py HARNESS DLL")
    harness = Path(sys.argv[1]).resolve()
    dll = Path(sys.argv[2]).resolve()
    work = Path("target/official-cases")
    work.mkdir(parents=True, exist_ok=True)

    cases: dict[str, tuple[bytes, int, int]] = {
        "empty_root": (xml(""), 0x4011, 0),
        "empty_string": (xml('<x __type="str"></x>'), 0x4011, 0),
        "whitespace_string": (xml('<x __type="str"> \t\r\n </x>'), 0x4011, 0),
        "duplicate_nodes": (xml('<x __type="s32">1</x><x __type="s32">2</x>'), 0x4011, 0),
        "plus_integer": (xml('<x __type="s32">+1</x>'), 0x4011, 0),
        "leading_zero_integer": (xml('<x __type="s32">0001</x>'), 0x4011, 0),
        "hex_integer": (xml('<x __type="s32">0x10</x>'), 0x4011, 0),
        "integer_spaces": (xml('<x __type="s32"> 1 </x>'), 0x4011, 0),
        "negative_hex_integer": (xml('<x __type="s32">-0x10</x>'), 0x4011, 0),
        "uppercase_hex_integer": (xml('<x __type="u32">0XFF</x>'), 0x4011, 0),
        "integer_tab_newline": (xml('<x __type="s32">\t-7\n</x>'), 0x4011, 0),
        "s32_overflow": (xml('<x __type="s32">2147483648</x>'), 0x4011, 0),
        "s32_underflow": (xml('<x __type="s32">-2147483649</x>'), 0x4011, 0),
        "u32_negative": (xml('<x __type="u32">-1</x>'), 0x4011, 0),
        "numeric_empty": (xml('<x __type="s32"></x>'), 0x4011, 0),
        "numeric_invalid": (xml('<x __type="s32">nope</x>'), 0x4011, 0),
        "float_nan": (xml('<x __type="float">NaN</x>'), 0x4011, 0),
        "float_inf": (xml('<x __type="float">INF</x>'), 0x4011, 0),
        "float_neg_inf": (xml('<x __type="float">-INF</x>'), 0x4011, 0),
        "float_negative_zero": (xml('<x __type="float">-0</x>'), 0x4011, 0),
        "float_exponent": (xml('<x __type="float">1.25e2</x>'), 0x4011, 0),
        "float_leading_dot": (xml('<x __type="float">.5</x>'), 0x4011, 0),
        "float_trailing_dot": (xml('<x __type="float">1.</x>'), 0x4011, 0),
        "float_overflow": (xml('<x __type="float">1e999</x>'), 0x4011, 0),
        "double_precision": (xml('<x __type="double">1.2345678901234567</x>'), 0x4011, 0),
        "bool_two": (xml('<x __type="bool">2</x>'), 0x4011, 0),
        "bool_true_word": (xml('<x __type="bool">true</x>'), 0x4011, 0),
        "bool_negative": (xml('<x __type="bool">-1</x>'), 0x4011, 0),
        "bool_zero": (xml('<x __type="bool">0</x>'), 0x4011, 0),
        "bool_one": (xml('<x __type="bool">1</x>'), 0x4011, 0),
        "bool_leading_zero": (xml('<x __type="bool">01</x>'), 0x4011, 0),
        "bool_spaces": (xml('<x __type="bool"> 1 </x>'), 0x4011, 0),
        "array_zero": (xml('<x __type="s32" __count="0"></x>'), 0x4011, 0),
        "array_count_short": (xml('<x __type="s32" __count="2">1</x>'), 0x4011, 0),
        "array_count_long": (xml('<x __type="s32" __count="1">1 2</x>'), 0x4011, 0),
        "array_double_spaces": (xml('<x __type="s32" __count="2">1  2</x>'), 0x4011, 0),
        "array_commas": (xml('<x __type="s32" __count="2">1,2</x>'), 0x4011, 0),
        "array_negative_count": (xml('<x __type="s32" __count="-1">1</x>'), 0x4011, 0),
        "array_invalid_count": (xml('<x __type="s32" __count="x">1</x>'), 0x4011, 0),
        "array_missing_count": (xml('<x __type="s32">1 2</x>'), 0x4011, 0),
        "binary_normal": (xml('<x __type="bin" __size="3">00aaff</x>'), 0x4011, 0),
        "binary_upper": (xml('<x __type="bin" __size="3">00AAFF</x>'), 0x4011, 0),
        "binary_odd": (xml('<x __type="bin" __size="2">abc</x>'), 0x4011, 0),
        "binary_invalid": (xml('<x __type="bin" __size="1">gg</x>'), 0x4011, 0),
        "binary_spaces": (xml('<x __type="bin" __size="2">aa bb</x>'), 0x4011, 0),
        "binary_short": (xml('<x __type="bin" __size="3">aabb</x>'), 0x4011, 0),
        "binary_long": (xml('<x __type="bin" __size="1">aabb</x>'), 0x4011, 0),
        "binary_no_size": (xml('<x __type="bin">aabb</x>'), 0x4011, 0),
        "ip4_normal": (xml('<x __type="ip4">127.0.0.1</x>'), 0x4011, 0),
        "ip4_short": (xml('<x __type="ip4">1.2.3</x>'), 0x4011, 0),
        "ip4_long": (xml('<x __type="ip4">1.2.3.4.5</x>'), 0x4011, 0),
        "ip4_overflow": (xml('<x __type="ip4">256.0.0.1</x>'), 0x4011, 0),
        "ip4_hex": (xml('<x __type="ip4">0x7f.0.0.1</x>'), 0x4011, 0),
        "unknown_type": (xml('<x __type="wat">1</x>'), 0x4011, 0),
        "reserved_type_attr": (xml('<x __type="str" __evil="1">v</x>'), 0x4011, 0),
        "duplicate_attribute": (b'<?xml version="1.0"?><root a="1" a="2"/>', 0x4011, 0),
        "entity_numeric": (xml('<x __type="str">&#0;&#1;&#31;&#127;</x>'), 0x4011, 0),
        "entity_unknown": (xml('<x __type="str">&unknown;</x>'), 0x4011, 0),
        "comment_pi_cdata": (xml('<!--c--><?p i?><x __type="str"><![CDATA[a<b&c]]></x>'), 0x4011, 0),
        "hyphen_packed": (xml('<hand-trip __type="u8">1</hand-trip>'), 0x4011, 8),
        "hyphen_full": (xml('<hand-trip __type="u8">1</hand-trip>'), 0x1011, 0x1008),
        "ascii": (xml('<x __type="str">ASCII</x>', "ASCII"), 0x4011, 8),
        "latin1": (xml('<x __type="str">café</x>', "ISO-8859-1"), 0x4011, 8),
        "euc_jp": (xml('<x __type="str">日本語</x>', "EUC-JP"), 0x4011, 8),
        "shift_jis": (xml('<x __type="str">日本語</x>', "SHIFT_JIS"), 0x4011, 8),
        "utf8": (xml('<x __type="str">日本語</x>', "UTF-8"), 0x4011, 8),
        "unknown_encoding": (xml('<x __type="str">x</x>').replace(b"UTF-8", b"UTF-16"), 0x4011, 0),
    }

    for length in (1, 36, 37, 63, 64, 127, 255, 256):
        name = "a" * length
        cases[f"packed_name_{length}"] = (xml(f'<{name} __type="u8">1</{name}>'), 0x4011, 8)
    for length in (1, 64, 65, 255, 256, 4095, 4096, 4097):
        name = "a" * length
        cases[f"full_name_{length}"] = (xml(f'<{name} __type="u8">1</{name}>'), 0x1011, 0x1008)
    for depth in (32, 64, 128, 256, 512, 1024):
        body = "<n>" * depth + '<x __type="u8">1</x>' + "</n>" * depth
        cases[f"depth_{depth}"] = (xml(body), 0x4011, 8)

    print("case\tstatus\tresult\toutput_len\toutput_head")
    for name, (payload, create_mode, output_mode) in cases.items():
        input_path = work / f"{name}.input"
        output_path = work / f"{name}.output"
        input_path.write_bytes(payload)
        output_path.unlink(missing_ok=True)
        completed = subprocess.run(
            [
                str(harness),
                str(dll),
                str(input_path),
                str(output_path),
                hex(create_mode),
                "0",
                "0",
                hex(output_mode),
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=15,
        )
        stderr = completed.stderr.decode("utf-8", "replace").strip().replace("\t", " ")
        output = output_path.read_bytes() if output_path.exists() else b""
        status = "ok" if completed.returncode == 0 else f"exit:{completed.returncode}"
        print(f"{name}\t{status}\t{stderr}\t{len(output)}\t{output[:12].hex()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
