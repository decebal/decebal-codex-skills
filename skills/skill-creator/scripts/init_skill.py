#!/usr/bin/env python3
"""Scaffold a new skill directory.

Usage:
    scripts/init_skill.py <skill-name> --path <output-directory>

Creates <output-directory>/<skill-name>/ with a SKILL.md template (frontmatter +
TODO placeholders) and example scripts/, references/, and assets/ resource
directories. Delete whichever example resources the skill does not need.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

NAME_RE = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")

SKILL_TEMPLATE = """\
---
name: {name}
description: TODO — one or two sentences on WHAT this skill does and WHEN to use \
it. Write in the third person and include concrete trigger phrases so the model \
can decide relevance.
---

# {title}

TODO — a short paragraph on what this skill is for.

## When to use

TODO — the situations that should trigger this skill.

## Instructions

TODO — the procedure. Use imperative form. Reference bundled resources by \
relative path, e.g. `references/workflows.md` or `scripts/do_thing.py`.
"""

EXAMPLE_SCRIPT = """\
#!/usr/bin/env python3
\"\"\"Example script for the {name} skill. Replace or delete.\"\"\"

if __name__ == "__main__":
    print("TODO: implement {name} script")
"""

EXAMPLE_REFERENCE = """\
# {title} — reference

TODO — deeper documentation the SKILL.md links to. Delete if unused.
"""

EXAMPLE_ASSET = (
    "TODO: replace this placeholder with a real asset (template, image, "
    "config, ...) or delete the assets/ directory.\n"
)


def title_from(name: str) -> str:
    return name.replace("-", " ").title()


def write(path: Path, content: str, *, executable: bool = False) -> None:
    path.write_text(content, encoding="utf-8")
    if executable:
        path.chmod(0o755)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Scaffold a new skill directory.")
    parser.add_argument("name", help="skill name (kebab-case: lowercase, digits, hyphens)")
    parser.add_argument(
        "--path",
        default=".",
        help="output directory the skill folder is created under (default: current dir)",
    )
    args = parser.parse_args(argv)

    name = args.name
    if not NAME_RE.match(name):
        parser.error(f"invalid skill name {name!r}: use kebab-case (e.g. my-skill)")

    skill_dir = Path(args.path).expanduser() / name
    if skill_dir.exists():
        parser.error(f"{skill_dir} already exists — refusing to overwrite")

    title = title_from(name)
    (skill_dir / "scripts").mkdir(parents=True)
    (skill_dir / "references").mkdir()
    (skill_dir / "assets").mkdir()

    write(skill_dir / "SKILL.md", SKILL_TEMPLATE.format(name=name, title=title))
    write(skill_dir / "scripts" / "example.py", EXAMPLE_SCRIPT.format(name=name), executable=True)
    write(skill_dir / "references" / "example.md", EXAMPLE_REFERENCE.format(title=title))
    write(skill_dir / "assets" / "example.txt", EXAMPLE_ASSET)

    print(f"Created skill at {skill_dir}")
    print("  SKILL.md            — fill in frontmatter + instructions")
    print("  scripts/example.py  — example executable (customize or delete)")
    print("  references/example.md — example reference doc (customize or delete)")
    print("  assets/example.txt  — example asset (customize or delete)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
