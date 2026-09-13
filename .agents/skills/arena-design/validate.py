#!/usr/bin/env python3
"""Validate the local Arena design skill pack without third-party dependencies."""

from __future__ import annotations

import json
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent
REQUIRED_HEADINGS = ("## Use when", "## Workflow", "## Deliverables", "## Guardrails", "## Completion gate")


def main() -> None:
    registry = json.loads((ROOT / "registry.json").read_text())
    assert registry["schema"] == 1
    assert registry["mode"] == "workspace-instruction-pack"
    assert registry["upstream_code_installed"] is False
    assert registry["auto_activation"]["enabled"] is True
    assert registry["auto_activation"]["router"] == "route.py"
    assert (ROOT / registry["auto_activation"]["router"]).is_file()
    skills = registry["skills"]
    names = [skill["name"] for skill in skills]
    assert len(skills) == 13, f"expected 13 skills, got {len(skills)}"
    assert len(names) == len(set(names)), "skill names must be unique"

    for skill in skills:
        path = ROOT / skill["path"]
        assert path.is_file(), f"missing {path}"
        text = path.read_text()
        assert text.startswith("---\n"), f"missing front matter: {path}"
        assert f"name: {skill['name']}" in text
        assert skill["auto_activate"] is True
        assert skill["triggers"], f"triggers missing for {skill['name']}"
        assert "auto_activate: true" in text
        for heading in REQUIRED_HEADINGS:
            assert heading in text, f"{path} lacks {heading}"
        parsed = urlparse(skill["source"])
        assert parsed.scheme == "https" and parsed.netloc == "github.com"
        assert skill["source_license"], f"license note missing for {skill['name']}"

    print(f"design-skill-pack=valid skills={len(skills)} upstream-code-installed=false")


if __name__ == "__main__":
    main()
