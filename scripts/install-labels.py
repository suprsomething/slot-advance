#!/usr/bin/env python3
"""Copy original labels with exact ROM stems. No ROMs or existing labels are changed."""
import argparse
import json
import re
import shutil
from pathlib import Path


def game_key(name):
    name = re.sub(r"\s*[\[(].*?[\])]", "", name).casefold()
    name = name.replace("fire red", "firered")
    name = re.sub(r"[^a-z0-9]+", " ", name).strip()
    if name.startswith("the "):
        name = name[4:] + " the"
    return name


def install(roms, output, dry_run=False):
    pack = Path(__file__).resolve().parents[1] / "artwork/gba-originals"
    entries = json.loads((pack / "manifest.json").read_text())["labels"]
    lookup = {}
    for entry in entries:
        for title in [entry["title"], *entry.get("aliases", [])]:
            key = game_key(title)
            if key in lookup and lookup[key] != entry:
                raise ValueError(f"Ambiguous artwork title: {title}")
            lookup[key] = entry
    copied = skipped = missing = 0
    for rom in sorted(roms.iterdir()):
        if not rom.is_file() or rom.suffix.casefold() != ".gba" or rom.name.startswith("."):
            continue
        entry = lookup.get(game_key(rom.stem))
        # Hacks must not silently receive artwork from their retail base game.
        if not entry or "patched" in rom.stem.casefold() or re.search(r"\[(?:t-|hack)", rom.stem, re.I):
            print(f"No label: {rom.name}")
            missing += 1
            continue
        target = output / f"{rom.stem}.png"
        if target.exists():
            print(f"Kept existing: {target.name}")
            skipped += 1
            continue
        if not dry_run:
            output.mkdir(parents=True, exist_ok=True)
            # Exclusive creation protects labels even if another process writes one.
            with (pack / entry["file"]).open("rb") as source, target.open("xb") as dest:
                shutil.copyfileobj(source, dest)
        print(f"{'Would add' if dry_run else 'Added'}: {target.name}")
        copied += 1
    print(f"{copied} {'planned' if dry_run else 'added'}, {skipped} kept, {missing} without artwork")
    return copied, skipped, missing


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("card", type=Path, help="Slot games-card root (contains Games/GBA)")
    parser.add_argument("--dry-run", action="store_true", help="Preview without writing")
    args = parser.parse_args()
    roms = args.card / "Games/GBA"
    if not roms.is_dir():
        parser.error(f"GBA folder not found: {roms}")
    install(roms, args.card / "Labels/GBA", args.dry_run)


if __name__ == "__main__":
    main()
