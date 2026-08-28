#!/usr/bin/env python3
"""Validate and package a skill into a distributable .skill archive.

Usage:
    scripts/package_skill.py <path/to/skill-folder> [output-dir]

Validates the skill's SKILL.md frontmatter, naming, and structure, then (only if
validation passes) writes <output-dir>/<name>.skill — a zip archive of the skill
folder. Exits non-zero and prints the errors if validation fails.
"""

from __future__ import annotations

import re
import sys
import zipfile
from pathlib import Path

NAME_RE = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")


def parse_frontmatter(skill_md: Path) -> tuple[dict[str, str], list[str]]:
    """Return (fields, errors). Only the flat top-level `key: value` lines are read."""
    errors: list[str] = []
    text = skill_md.read_text(encoding="utf-8")
    if not text.startswith("---"):
        return {}, ["SKILL.md does not start with a '---' YAML frontmatter block"]
    end = text.find("\n---", 3)
    if end == -1:
        return {}, ["SKILL.md frontmatter block is not closed with '---'"]
    fields: dict[str, str] = {}
    for line in text[3:end].splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" in line:
            key, _, value = line.partition(":")
            fields[key.strip()] = value.strip()
    return fields, errors


def validate(skill_dir: Path) -> list[str]:
    errors: list[str] = []
    if not skill_dir.is_dir():
        return [f"{skill_dir} is not a directory"]

    if not NAME_RE.match(skill_dir.name):
        errors.append(f"directory name {skill_dir.name!r} is not kebab-case")

    skill_md = skill_dir / "SKILL.md"
    if not skill_md.is_file():
        errors.append("missing SKILL.md")
        return errors

    fields, fm_errors = parse_frontmatter(skill_md)
    errors.extend(fm_errors)

    name = fields.get("name")
    if not name:
        errors.append("frontmatter is missing `name`")
    elif not NAME_RE.match(name):
        errors.append(f"frontmatter `name` {name!r} is not kebab-case")
    elif name != skill_dir.name:
        errors.append(f"frontmatter `name` ({name}) does not match directory ({skill_dir.name})")

    description = fields.get("description", "")
    if not description:
        errors.append("frontmatter is missing `description`")
    elif len(description) < 20:
        errors.append("frontmatter `description` is too short to convey what/when (< 20 chars)")
    if "TODO" in description:
        errors.append("frontmatter `description` still contains a TODO placeholder")

    return errors


def package(skill_dir: Path, out_dir: Path) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    archive = out_dir / f"{skill_dir.name}.skill"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(skill_dir.rglob("*")):
            if path.is_file():
                zf.write(path, path.relative_to(skill_dir.parent))
    return archive


def main(argv: list[str]) -> int:
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0 if argv else 2

    skill_dir = Path(argv[0]).expanduser().resolve()
    out_dir = Path(argv[1]).expanduser() if len(argv) > 1 else skill_dir.parent

    errors = validate(skill_dir)
    if errors:
        print(f"Validation FAILED for {skill_dir.name}:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    archive = package(skill_dir, out_dir)
    print(f"Validated OK. Packaged {skill_dir.name} -> {archive}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
