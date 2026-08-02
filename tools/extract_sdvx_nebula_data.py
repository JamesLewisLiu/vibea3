#!/usr/bin/env python3
"""Convert the pinned KFC 2026-07-14 update databases into Vibea3 JSON.

The source XML is CP932 in practice, including byte sequences that Python's
strict Shift-JIS codec rejects. The game accepts replacement characters for
those malformed source strings, so the extractor mirrors that behavior while
pinning every input hash to prevent mixing incompatible update revisions.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path


EXPECTED = {
    "music_db.xml": "80de93020f05393e8aa004508fb7c75b17a8badf2281837058725f042b088230",
    "appeal_card.xml": "84d626285f7ff5d9cab007f40aca0b5f3825b748da035d017bd2036003f99699",
    "akaname_parts.xml": "9a16dcfb0b174931bc502efd2803a2d50c03f480e526b7bd02d14c210b99c46e",
}

DIFFICULTIES = ("novice", "advanced", "exhaust", "infinite", "maximum", "ultimate")


def root(path: Path) -> ET.Element:
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    expected = EXPECTED[path.name]
    if digest != expected:
        raise ValueError(f"unsupported {path.name} SHA-256 {digest}; expected {expected}")
    return ET.fromstring(raw.decode("cp932", errors="replace"))


def text(parent: ET.Element, name: str, default: str = "") -> str:
    child = parent.find(name)
    value = default if child is None or child.text is None else child.text
    return value.replace("\ufffd", "?")


def integer(parent: ET.Element, name: str, default: int = 0) -> int:
    value = text(parent, name)
    return default if value == "" else int(value, 0)


def chart(node: ET.Element | None) -> dict[str, object] | None:
    if node is None:
        return None
    radar = node.find("radar")
    assert radar is not None
    return {
        "illustrator": text(node, "illustrator"),
        "effected_by": text(node, "effected_by"),
        "level": integer(node, "difnum") // 10,
        "price": integer(node, "price"),
        "limited": integer(node, "limited"),
        "jacket_print": integer(node, "jacket_print"),
        "jacket_mask": integer(node, "jacket_mask"),
        "max_exscore": integer(node, "max_exscore"),
        "radar": {
            "notes": integer(radar, "notes"),
            "peak": integer(radar, "peak"),
            "tsumami": integer(radar, "tsumami"),
            "tricky": integer(radar, "tricky"),
            "hand_trip": integer(radar, "hand-trip"),
            "one_hand": integer(radar, "one-hand"),
        },
    }


def music_records(path: Path) -> list[dict[str, object]]:
    records = []
    for node in root(path).findall("music"):
        info = node.find("info")
        difficulty = node.find("difficulty")
        assert info is not None and difficulty is not None
        record: dict[str, object] = {
            "music_id": int(node.attrib["id"]),
            "title_name": text(info, "title_name"),
            "title_yomigana": text(info, "title_yomigana"),
            "artist_name": text(info, "artist_name"),
            "artist_yomigana": text(info, "artist_yomigana"),
            "license_text": text(info, "license_text"),
            "ascii": text(info, "ascii"),
            "bpm_max": integer(info, "bpm_max"),
            "bpm_min": integer(info, "bpm_min"),
            "date": integer(info, "distribution_date"),
            "volume": integer(info, "volume"),
            "version": integer(info, "version"),
            "inf_ver": integer(info, "inf_ver"),
            "bg_no": integer(info, "bg_no"),
            "genre": integer(info, "genre"),
            "demo_pri": integer(info, "demo_pri"),
        }
        for name in DIFFICULTIES:
            record[name] = chart(difficulty.find(name))
        records.append(record)
    return records


def appeal_cards(path: Path) -> list[dict[str, object]]:
    cards = []
    for node in root(path).findall("card"):
        info = node.find("info")
        assert info is not None
        cards.append({
            "appeal_id": int(node.attrib["id"]),
            "texture": text(info, "texture"),
            "title": text(info, "title"),
            "illustrator": text(info, "illustrator"),
            **{f"message_{letter}": text(info, f"message_{letter}") for letter in "abcdefgh"},
            "date": integer(info, "distribution_date"),
            "rarity": integer(info, "rarity"),
            "generator_no": integer(info, "generator_no"),
            "is_default": integer(info, "is_default"),
            "sort_no": integer(info, "sort_no"),
            "genre": integer(info, "genre"),
            "limited": integer(info, "limited"),
        })
    return cards


def akaname_parts(path: Path) -> list[dict[str, object]]:
    return [{
        "part_id": int(node.attrib["id"]),
        "word": text(node, "word"),
        "is_default": integer(node, "default") != 0,
    } for node in root(path).findall("part")]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path, help="contents/data/others directory")
    parser.add_argument("output", type=Path, help="catalog.json to write")
    args = parser.parse_args()
    document = {
        "source_sha256": EXPECTED,
        "music": music_records(args.source / "music_db.xml"),
        "appeal_cards": appeal_cards(args.source / "appeal_card.xml"),
        "akaname_parts": akaname_parts(args.source / "akaname_parts.xml"),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"wrote {len(document['music'])} music, {len(document['appeal_cards'])} appeal cards, and {len(document['akaname_parts'])} akaname parts")


if __name__ == "__main__":
    main()
