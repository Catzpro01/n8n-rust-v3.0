#!/usr/bin/env python3
"""Print a compact context packet for the repository Arena workflow.

This keeps discovery useful without flooding an agent's context with the whole
repository. It refreshes the incremental index, prints Git state, and emits a
small number of ranked snippets for the requested task.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

# The script is intentionally dependency-free and reuses the existing indexer.
from codebase_index import build_index, find_snippet, resolve_paths, search  # noqa: E402


def git_output(root: Path, *arguments: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), *arguments],
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable"
    output = (result.stdout or result.stderr).strip()
    return output or "clean"


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Print a compact Arena context packet")
    parser.add_argument("query", nargs="?", help="task terms, symbol, or feature to locate")
    parser.add_argument("--root", default=".", help="repository root (default: current directory)")
    parser.add_argument("--cache", help="cache path (default: cache.kv under root)")
    parser.add_argument("--limit", type=int, default=6, help="maximum snippets (default: 6)")
    parser.add_argument("--no-index", action="store_true", help="use the existing cache without refreshing")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    root, cache_path = resolve_paths(args.root, args.cache)
    if not root.is_dir():
        print(f"error: root is not a directory: {root}", file=sys.stderr)
        return 2

    if args.no_index:
        from codebase_index import Cache

        cache = Cache.load(cache_path)
        summary = {"changed": 0, "reused": 0, "removed": 0, "skipped": 0, "documents": len(cache.files)}
    else:
        cache, summary = build_index(root, cache_path)

    print("ARENA_CONTEXT")
    print(f"root={root}")
    print(
        "index="
        f"{summary['documents']} docs; "
        f"{summary['changed']} changed; "
        f"{summary['reused']} reused; "
        f"{summary['removed']} removed; "
        f"{summary['skipped']} skipped"
    )
    print(f"cache={cache_path}")
    print(f"git_branch={git_output(root, 'branch', '--show-current')}")
    print("git_status:")
    print(git_output(root, "status", "--short"))

    if not args.query:
        print("query=none; use a task phrase to retrieve focused files")
        return 0

    limit = max(1, min(args.limit, 20))
    hits = search(cache, args.query, limit=limit)
    print(f"query={args.query}")
    if not hits:
        print("matches=none")
        return 1

    print(f"matches={len(hits)}")
    for position, hit in enumerate(hits, start=1):
        relative_path = str(hit["path"])
        snippet = find_snippet(root, relative_path, args.query)
        location = f"{relative_path}:{snippet[0]}" if snippet else relative_path
        print(f"[{position}] {location} score={float(hit['score']):.2f}")
        if snippet and snippet[1]:
            print(f"    {snippet[1]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
