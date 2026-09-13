#!/usr/bin/env python3
"""Acceptance checks for automatic design-skill activation."""

from route import route


def skills(prompt: str) -> list[str]:
    return route(prompt)["skills"]  # type: ignore[return-value]


def main() -> None:
    assert skills("optimalkan transaksi SQLite dan indeks database") == []
    assert skills("design database schema dan API contract") == []
    assert "taste-skill" in skills("buat desain logo yang sederhana")

    diagram = skills("buat diagram arsitektur lease dan recovery")
    assert diagram == ["excalidraw"]

    redesign = skills("redesign tampilan dashboard React dan rapikan design token")
    for expected in (
        "onlook",
        "design-md-tokens",
        "ui-ux-pro-max",
        "taste-skill",
        "axe-core-a11y",
        "visual-regression-qa",
    ):
        assert expected in redesign, (expected, redesign)

    component = skills("implement UI dialog dan form yang responsive")
    assert "shadcn-ui-engine" in component
    assert "axe-core-a11y" in component

    assistant = skills("buat assistant UI dengan streaming tool card")
    assert "assistant-generative-ui" in assistant

    assert skills("parse Figma export menjadi token") == ["understand-figma"]
    assert skills("buat video animasi dengan Remotion") == ["remotion-best-practices"]

    release_audit = skills("audit UI sampai profesional dan release-ready")
    for expected in ("hallmark", "taste-skill", "axe-core-a11y", "visual-regression-qa"):
        assert expected in release_audit, (expected, release_audit)

    opted_out = route("redesign dashboard tetapi jangan gunakan skill desain")
    assert opted_out["opted_out"] is True
    assert opted_out["skills"] == []

    print("design-auto-routing=passed cases=11")


if __name__ == "__main__":
    main()
