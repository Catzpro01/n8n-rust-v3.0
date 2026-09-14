#!/usr/bin/env python3
"""Deterministically select the smallest useful design-skill pipeline."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent

OPT_OUT = (
    "disable design skills",
    "disable design skill",
    "jangan gunakan skill design",
    "jangan gunakan skill desain",
    "tanpa skill design",
    "tanpa skill desain",
    "design skills off",
)

GROUPS = {
    "diagram": ("diagram", "arsitektur", "architecture", "flowchart", "sequence", "wireframe", "sketch", "peta sistem"),
    "figma": ("figma", "figma export", "figma plugin", "file desain", "design file"),
    "penpot": ("penpot", "design handoff", "handoff desain", "vector handoff"),
    "video": ("remotion", "programmatic video", "video", "animation", "animasi", "composition", "timeline"),
    "a11y": ("accessibility", "aksesibilitas", "a11y", "wcag", "aria", "keyboard", "contrast", "kontras"),
    "visual_qa": ("visual regression", "screenshot", "pixel diff", "viewport", "resolution", "resolusi", "baseline image"),
    "ui": (" ui ", "ui/ux", " ux ", "user interface", "tampilan", "layout", "responsive", "website", "web page", "halaman", "dashboard", "frontend", "component", "komponen", "form", "dialog", "menu", "table", "tabel", "button", "design system", "sistem desain", "design token", "token desain", "theme", "tema", "palette", "palet", "typography", "tipografi", "css", "react", "next.js", "preact"),
    "generic_visual": (" design ", " desain ", "redesign", "perindah", "make it beautiful", "aesthetic", "estetika", "visual design", "desain visual", "polish tampilan"),
    "tokens": ("design system", "sistem desain", "design token", "token desain", "theme", "tema", "palette", "palet", "typography", "tipografi", "warna", "spacing"),
    "source_edit": ("frontend", "react", "next.js", "preact", "css", "website", "web page", "halaman", "dashboard", "editor ui", "ubah tampilan", "edit ui", "implement ui"),
    "component": ("component", "komponen", "form", "dialog", "menu", "table", "tabel", "button", "input", "select"),
    "assistant": ("assistant ui", "chat ui", "generative ui", "agent canvas", "streaming card", "tool card", "multi-agent ui"),
    "audit": ("design audit", "audit desain", "audit ui", "professional", "profesional", "release-ready", "final polish", "review tampilan", "polish ui"),
}


def contains(text: str, group: str) -> bool:
    padded = f" {text.casefold()} "
    return any(needle.casefold() in padded for needle in GROUPS[group])


def route(prompt: str) -> dict[str, object]:
    text = prompt.casefold()
    if any(phrase in text for phrase in OPT_OUT):
        return {"design_intent": False, "opted_out": True, "skills": [], "reasons": {}}

    selected: set[str] = set()
    reasons: dict[str, list[str]] = {}

    def add(skill: str, reason: str) -> None:
        selected.add(skill)
        reasons.setdefault(skill, []).append(reason)

    direct_names = {
        skill["name"]
        for skill in json.loads((ROOT / "registry.json").read_text())["skills"]
        if skill["name"].casefold() in text
    }
    for name in direct_names:
        add(name, "explicit skill name")

    if contains(text, "diagram"):
        add("excalidraw", "diagram or architecture intent")
    if contains(text, "figma"):
        add("understand-figma", "Figma ingestion intent")
    if contains(text, "penpot"):
        add("penpot", "Penpot or design-handoff intent")
    if contains(text, "video"):
        add("remotion-best-practices", "programmatic video or animation intent")
    if contains(text, "a11y"):
        add("axe-core-a11y", "accessibility intent")
    if contains(text, "visual_qa"):
        add("visual-regression-qa", "screenshot or visual-regression intent")

    technical_only = any(
        term in text
        for term in ("database", "schema", "backend", "api contract", "storage engine", "compiler design")
    ) and not contains(text, "ui")
    generic_visual = contains(text, "generic_visual") and not technical_only and not contains(text, "diagram")
    ui_intent = contains(text, "ui") or generic_visual
    if ui_intent:
        add("ui-ux-pro-max", "UI/UX design intent")
        add("taste-skill", "automatic visual-quality gate")
        add("axe-core-a11y", "accessibility is part of UI quality")
    if contains(text, "tokens"):
        add("design-md-tokens", "design-system or token intent")
    if contains(text, "source_edit"):
        add("onlook", "browser-guided frontend source work")
        add("visual-regression-qa", "source changes need visual regression evidence")
    if contains(text, "component"):
        add("shadcn-ui-engine", "component design or implementation intent")
    if contains(text, "assistant"):
        add("assistant-generative-ui", "assistant or generative interface intent")
    if contains(text, "audit"):
        add("hallmark", "release-quality design audit")
        add("taste-skill", "aesthetic audit")
        add("axe-core-a11y", "accessibility audit")
        add("visual-regression-qa", "cross-viewport visual evidence")

    order = [skill["name"] for skill in json.loads((ROOT / "registry.json").read_text())["skills"]]
    skills = [name for name in order if name in selected]
    return {
        "design_intent": bool(skills),
        "opted_out": False,
        "skills": skills,
        "reasons": {name: reasons[name] for name in skills},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("prompt", nargs="+", help="User request to route")
    parser.add_argument("--json", action="store_true", help="Emit structured JSON")
    args = parser.parse_args()
    result = route(" ".join(args.prompt))
    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    elif result["opted_out"]:
        print("design-skills=off reason=explicit-opt-out")
    elif not result["skills"]:
        print("design-skills=inactive")
    else:
        print("design-skills=active " + ",".join(result["skills"]))


if __name__ == "__main__":
    main()
