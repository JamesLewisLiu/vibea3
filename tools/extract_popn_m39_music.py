#!/usr/bin/env python3
"""Extract the authoritative High Cheers music table from popn.dll.

The layout was recovered from CMusicDataTable accessors and the musicsub
application routine in the M39 2025-12-18 client.  This script deliberately
pins the input hash: the RVAs are build-specific and silently applying them to
another DLL would be worse than refusing to run.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path


EXPECTED_SHA256 = "f594d5f4ca1f121c58e134652cbf1b09b81a8270a8c20a093e2f621e494f5a9e"
MUSIC_TABLE_RVA = 0x7061C0
MUSIC_COUNT = 2_374
RECORD_SIZE = 312


class PeImage:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.data = path.read_bytes()
        pe = struct.unpack_from("<I", self.data, 0x3C)[0]
        if self.data[pe : pe + 4] != b"PE\0\0":
            raise ValueError(f"{path} is not a PE image")
        section_count = struct.unpack_from("<H", self.data, pe + 6)[0]
        optional_size = struct.unpack_from("<H", self.data, pe + 20)[0]
        optional = pe + 24
        if struct.unpack_from("<H", self.data, optional)[0] != 0x20B:
            raise ValueError(f"{path} is not PE32+")
        self.image_base = struct.unpack_from("<Q", self.data, optional + 24)[0]
        section_table = optional + optional_size
        self.sections: list[tuple[int, int, int, int]] = []
        for index in range(section_count):
            offset = section_table + index * 40
            virtual_size, virtual_address, raw_size, raw_offset = struct.unpack_from(
                "<IIII", self.data, offset + 8
            )
            self.sections.append(
                (virtual_address, max(virtual_size, raw_size), raw_offset, raw_size)
            )

    def file_offset(self, virtual_address: int) -> int:
        rva = virtual_address - self.image_base
        for section_rva, virtual_size, raw_offset, raw_size in self.sections:
            if section_rva <= rva < section_rva + virtual_size:
                relative = rva - section_rva
                if relative >= raw_size:
                    raise ValueError(f"unbacked virtual address {virtual_address:#x}")
                return raw_offset + relative
        raise ValueError(f"unmapped virtual address {virtual_address:#x}")

    def read_rva(self, rva: int, size: int) -> bytes:
        offset = self.file_offset(self.image_base + rva)
        value = self.data[offset : offset + size]
        if len(value) != size:
            raise ValueError(f"short read at RVA {rva:#x}")
        return value

    def cp932_string(self, virtual_address: int, maximum: int) -> str:
        offset = self.file_offset(virtual_address)
        end = self.data.find(b"\0", offset, offset + maximum + 1)
        if end < 0:
            raise ValueError(
                f"unterminated CP932 string at {virtual_address:#x} (max {maximum})"
            )
        raw = self.data[offset:end]
        value = raw.decode("cp932", errors="strict")
        return value


def unpack_array(fmt: str, record: bytes, offset: int, count: int) -> list[int]:
    return list(struct.unpack_from(f"<{count}{fmt}", record, offset))


def extract_music(image: PeImage) -> list[dict[str, object]]:
    music: list[dict[str, object]] = []
    for music_id in range(MUSIC_COUNT):
        record = image.read_rva(MUSIC_TABLE_RVA + music_id * RECORD_SIZE, RECORD_SIZE)
        strings = struct.unpack_from("<7Q", record, 0)
        music.append(
            {
                "music_id": music_id,
                "name_sort": image.cp932_string(strings[0], 4096),
                "title_sort": image.cp932_string(strings[1], 4096),
                "artist_sort": image.cp932_string(strings[2], 4096),
                # strings[3] is the client's private Romanized genre-sort alias.
                "name": image.cp932_string(strings[4], 4096),
                "title": image.cp932_string(strings[5], 4096),
                "artist": image.cp932_string(strings[6], 4096),
                "chr": struct.unpack_from("<h", record, 56)[0],
                "chr2": struct.unpack_from("<h", record, 58)[0],
                "mtype": struct.unpack_from("<I", record, 60)[0],
                "ac_ver": struct.unpack_from("<i", record, 64)[0],
                "cs_ver": struct.unpack_from("<i", record, 68)[0],
                "bm_from_u32": struct.unpack_from("<I", record, 72)[0],
                "lv": unpack_array("B", record, 76, 7),
                "track": unpack_array("H", record, 84, 7),
                "sp_hariai": image.cp932_string(
                    struct.unpack_from("<Q", record, 104)[0], 4096
                ),
                "sp_x": struct.unpack_from("<h", record, 112)[0],
                "sp_y": struct.unpack_from("<h", record, 114)[0],
                "tag_list": unpack_array("H", record, 122, 32),
                "bpm_min": unpack_array("h", record, 186, 7),
                "bpm_max": unpack_array("h", record, 200, 7),
                "long": unpack_array("b", record, 214, 7),
            }
        )

    first = music[0]
    last = music[-1]
    if first["name"] != "ポップス" or first["title"] != "I REALLY WANT TO HURT YOU":
        raise ValueError("music table sanity check failed at music 0")
    if last["title_sort"] != "トウメイハマダラニセカイヲツゲテ":
        raise ValueError("music table sanity check failed at music 2373")
    return music


def document(music: list[dict[str, object]]) -> dict[str, object]:
    return {
        "source_sha256": EXPECTED_SHA256,
        "music": music,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dll", type=Path, help="M39 popn.dll")
    parser.add_argument("output", type=Path, help="music.json to write")
    args = parser.parse_args()

    digest = hashlib.sha256(args.dll.read_bytes()).hexdigest()
    if digest != EXPECTED_SHA256:
        raise SystemExit(
            f"unsupported popn.dll SHA-256 {digest}; expected {EXPECTED_SHA256}"
        )
    music = extract_music(PeImage(args.dll))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document(music), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {len(music)} M39 music records to {args.output}")


if __name__ == "__main__":
    main()
