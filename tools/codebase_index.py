#!/usr/bin/env python3
"""Fast, incremental codebase indexer backed by a portable ``cache.kv`` file.

The index is deliberately local and dependency-free. It borrows the useful part
of editor indexes such as Cursor: unchanged files are reused from a persistent
cache, while changed files are tokenized and added to an inverted index for
fast symbol-aware search.

Usage:
    python3 tools/codebase_index.py index
    python3 tools/codebase_index.py search "workflow execution"
    python3 tools/codebase_index.py stats
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import math
import os
import re
import sys
import time
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, Iterator


CACHE_FORMAT = "codebase-index-v1"
DEFAULT_CACHE_NAME = "cache.kv"
MAX_FILE_BYTES = 5 * 1024 * 1024

# These directories contain generated/dependency data and would add noise and
# latency without improving code navigation.
EXCLUDED_DIRS = {
    ".git",
    ".agents",
    ".cache",
    ".idea",
    ".next",
    ".nuxt",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    ".vite",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "out",
    "target",
    "vendor",
}

TEXT_EXTENSIONS = {
    ".c",
    ".cc",
    ".cfg",
    ".cjs",
    ".conf",
    ".cpp",
    ".cs",
    ".css",
    ".dart",
    ".dockerfile",
    ".ex",
    ".exs",
    ".go",
    ".graphql",
    ".h",
    ".hpp",
    ".html",
    ".ini",
    ".java",
    ".js",
    ".json",
    ".jsx",
    ".kt",
    ".kts",
    ".lua",
    ".md",
    ".mjs",
    ".php",
    ".proto",
    ".py",
    ".rb",
    ".rs",
    ".sass",
    ".scala",
    ".scss",
    ".sh",
    ".sql",
    ".svelte",
    ".swift",
    ".toml",
    ".ts",
    ".tsx",
    ".txt",
    ".vue",
    ".xml",
    ".yaml",
    ".yml",
}

SPECIAL_FILE_NAMES = {
    "AUTHORS",
    "CONTRIBUTING",
    "Dockerfile",
    "LICENSE",
    "Makefile",
    "README",
    "README.md",
    "SECURITY",
    ".env.example",
    ".gitignore",
    ".npmrc",
    "Justfile",
}

SYMBOL_KEYWORDS = {
    "class",
    "const",
    "def",
    "enum",
    "fn",
    "func",
    "function",
    "impl",
    "interface",
    "let",
    "mod",
    "namespace",
    "struct",
    "trait",
    "type",
    "var",
}


@dataclass
class FileRecord:
    """The part of a file's index that can be reused between runs."""

    size: int
    mtime_ns: int
    digest: str
    lines: int
    terms: dict[str, int] = field(default_factory=dict)
    symbols: list[str] = field(default_factory=list)


@dataclass
class Cache:
    """In-memory representation of ``cache.kv``."""

    root: str = ""
    indexed_at: int = 0
    files: dict[str, FileRecord] = field(default_factory=dict)
    postings: dict[str, dict[str, int]] = field(default_factory=dict)

    @classmethod
    def load(cls, path: Path) -> "Cache":
        if not path.exists():
            return cls()

        cache = cls()
        try:
            for raw_line in path.read_text(encoding="utf-8").splitlines():
                line = raw_line.strip()
                if not line or line.startswith("#"):
                    continue
                key, separator, value = line.partition("=")
                if not separator:
                    continue
                if key == "format":
                    if value != CACHE_FORMAT:
                        return cls()
                elif key == "root":
                    cache.root = _decode(value)
                elif key == "indexed_at":
                    cache.indexed_at = int(value)
                elif key == "file":
                    parts = value.split("|")
                    if len(parts) != 7:
                        continue
                    try:
                        relative_path = _decode(parts[0])
                        terms = json.loads(_decode(parts[4]))
                        symbols = json.loads(_decode(parts[5]))
                        if not isinstance(terms, dict) or not isinstance(symbols, list):
                            continue
                        cache.files[relative_path] = FileRecord(
                            size=int(parts[2]),
                            mtime_ns=int(parts[1]),
                            digest=parts[3],
                            lines=int(parts[6]),
                            terms={str(k): int(v) for k, v in terms.items()},
                            symbols=[str(symbol) for symbol in symbols],
                        )
                    except (ValueError, TypeError, json.JSONDecodeError):
                        continue
                elif key == "term":
                    parts = value.split("|", 1)
                    if len(parts) != 2:
                        continue
                    try:
                        term = _decode(parts[0])
                        documents = json.loads(_decode(parts[1]))
                        if isinstance(documents, dict):
                            cache.postings[term] = {
                                str(path): int(frequency)
                                for path, frequency in documents.items()
                            }
                    except (ValueError, TypeError, json.JSONDecodeError):
                        continue
        except (OSError, UnicodeError, ValueError):
            return cls()
        return cache

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        lines = [
            f"format={CACHE_FORMAT}",
            "# Persistent metadata/index only; source contents are never stored.",
            f"root={_encode(self.root)}",
            f"indexed_at={self.indexed_at}",
        ]
        for relative_path in sorted(self.files):
            record = self.files[relative_path]
            terms = json.dumps(record.terms, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
            symbols = json.dumps(record.symbols, ensure_ascii=False, separators=(",", ":"))
            fields = [
                _encode(relative_path),
                str(record.mtime_ns),
                str(record.size),
                record.digest,
                _encode(terms),
                _encode(symbols),
                str(record.lines),
            ]
            lines.append("file=" + "|".join(fields))

        # Persist the inverted index too. Search can therefore load postings
        # directly instead of rebuilding them from every file record.
        postings = self.postings or _postings_from_files(self.files)
        for term in sorted(postings):
            documents = json.dumps(postings[term], separators=(",", ":"), sort_keys=True)
            lines.append(f"term={_encode(term)}|{_encode(documents)}")

        temporary_path = path.with_name(path.name + ".tmp")
        temporary_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        os.replace(temporary_path, path)


def _encode(value: str) -> str:
    return base64.urlsafe_b64encode(value.encode("utf-8")).decode("ascii")


def _decode(value: str) -> str:
    return base64.urlsafe_b64decode(value.encode("ascii")).decode("utf-8")


def iter_files(root: Path) -> Iterator[Path]:
    """Yield indexable files without following symlinks or generated trees."""

    def walk(directory: Path) -> Iterator[Path]:
        try:
            entries = sorted(directory.iterdir(), key=lambda entry: entry.name.lower())
        except OSError:
            return

        for entry in entries:
            try:
                if entry.is_symlink():
                    continue
                if entry.is_dir():
                    if entry.name in EXCLUDED_DIRS:
                        continue
                    yield from walk(entry)
                elif entry.is_file() and is_indexable(entry):
                    yield entry
            except OSError:
                continue

    yield from walk(root)


def is_indexable(path: Path) -> bool:
    if path.name in {DEFAULT_CACHE_NAME, DEFAULT_CACHE_NAME + ".tmp"}:
        return False
    if path.name in SPECIAL_FILE_NAMES:
        return True
    return path.suffix.lower() in TEXT_EXTENSIONS


def _identifier_parts(value: str) -> list[str]:
    # Split both snake_case and CamelCase/PascalCase while retaining useful
    # words for natural-language and symbol queries.
    value = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1 \2", value)
    value = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", value)
    return [part.lower() for part in re.findall(r"[^\W_]+", value, flags=re.UNICODE) if len(part) >= 2]


def tokenize(text: str) -> dict[str, int]:
    counts: Counter[str] = Counter()
    for fragment in re.findall(r"[\w]+", text, flags=re.UNICODE):
        for token in _identifier_parts(fragment):
            counts[token] += 1
    return dict(counts)


def extract_symbols(text: str) -> list[str]:
    """Extract lightweight, language-agnostic definitions for symbol boosting."""

    symbols: set[str] = set()
    for line in text.splitlines():
        words = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", line)
        for index, word in enumerate(words[:-1]):
            if word.lower() not in SYMBOL_KEYWORDS:
                continue
            candidate = words[index + 1]
            if candidate.lower() in {"for", "in", "where", "from", "as"}:
                continue
            symbols.add(candidate)
            break
    return sorted(symbols, key=str.lower)


def read_file_record(path: Path, relative_path: str, stat_result: os.stat_result) -> FileRecord | None:
    if stat_result.st_size > MAX_FILE_BYTES:
        return None
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if b"\0" in data[:8192]:
        return None
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return None

    return FileRecord(
        size=len(data),
        mtime_ns=stat_result.st_mtime_ns,
        digest=hashlib.blake2b(data, digest_size=16).hexdigest(),
        lines=text.count("\n") + (1 if text else 0),
        terms=tokenize(text),
        symbols=extract_symbols(text),
    )


def build_index(root: Path, cache_path: Path, force: bool = False) -> tuple[Cache, dict[str, int]]:
    """Incrementally scan *root* and persist the resulting index."""

    root = root.resolve()
    previous = Cache.load(cache_path)
    root_marker = str(root)
    reusable = previous.files if previous.root == root_marker and not force else {}

    records: dict[str, FileRecord] = {}
    scanned = changed = reused = skipped = 0

    cache_absolute = cache_path.resolve()
    for path in iter_files(root):
        # Keep a custom cache location out of its own index as well as the
        # default ``cache.kv`` location handled by ``is_indexable``.
        if path.resolve() == cache_absolute:
            continue
        relative_path = path.relative_to(root).as_posix()
        scanned += 1
        try:
            stat_result = path.stat()
        except OSError:
            skipped += 1
            continue

        old_record = reusable.get(relative_path)
        if old_record and old_record.size == stat_result.st_size and old_record.mtime_ns == stat_result.st_mtime_ns:
            records[relative_path] = old_record
            reused += 1
            continue

        record = read_file_record(path, relative_path, stat_result)
        if record is None:
            skipped += 1
            continue
        records[relative_path] = record
        changed += 1

    removed = len(set(reusable) - set(records))
    cache = Cache(
        root=root_marker,
        indexed_at=int(time.time()),
        files=records,
        postings=_postings_from_files(records),
    )
    cache.save(cache_path)
    return cache, {
        "scanned": scanned,
        "changed": changed,
        "reused": reused,
        "removed": removed,
        "skipped": skipped,
        "documents": len(records),
    }


def _postings_from_files(files: dict[str, FileRecord]) -> dict[str, dict[str, int]]:
    postings: dict[str, dict[str, int]] = defaultdict(dict)
    for relative_path, record in files.items():
        for term, frequency in record.terms.items():
            postings[term][relative_path] = frequency
    return dict(postings)


def build_postings(cache: Cache) -> dict[str, dict[str, int]]:
    if not cache.postings:
        cache.postings = _postings_from_files(cache.files)
    return cache.postings


def _query_terms(query: str) -> list[str]:
    return list(tokenize(query).keys())


def search(cache: Cache, query: str, limit: int = 10) -> list[dict[str, object]]:
    """Search the inverted index with TF-IDF-style and symbol/path boosts."""

    query_terms = _query_terms(query)
    if not query_terms or not cache.files:
        return []

    postings = build_postings(cache)
    documents = len(cache.files)
    scores: defaultdict[str, float] = defaultdict(float)
    matched: defaultdict[str, set[str]] = defaultdict(set)

    for query_term in query_terms:
        candidates: list[tuple[str, dict[str, int], float]] = []
        exact = postings.get(query_term)
        if exact:
            candidates.append((query_term, exact, 1.0))
        else:
            # Prefix matching gives editor-like results for incomplete symbols
            # without needing a heavyweight fuzzy-search dependency.
            for actual_term, docs in postings.items():
                if len(query_term) >= 2 and actual_term.startswith(query_term):
                    candidates.append((actual_term, docs, 0.45))

        for actual_term, docs, match_weight in candidates:
            document_frequency = len(docs)
            inverse_document_frequency = math.log((documents + 1) / (document_frequency + 1)) + 1
            for relative_path, frequency in docs.items():
                scores[relative_path] += (
                    match_weight
                    * (1 + math.log(frequency))
                    * inverse_document_frequency
                )
                matched[relative_path].add(actual_term)

    for relative_path, record in cache.files.items():
        path_text = relative_path.lower()
        path_terms = set(tokenize(relative_path).keys())
        symbol_terms = set(tokenize(" ".join(record.symbols)).keys())
        for query_term in query_terms:
            if query_term in path_terms:
                scores[relative_path] += 2.5
                matched[relative_path].add(f"path:{query_term}")
            elif query_term in path_text:
                scores[relative_path] += 0.5
            if query_term in symbol_terms:
                scores[relative_path] += 2.0
                matched[relative_path].add(f"symbol:{query_term}")

    ranked = sorted(scores.items(), key=lambda item: (-item[1], item[0]))[:limit]
    return [
        {
            "path": relative_path,
            "score": score,
            "matched": sorted(matched[relative_path]),
            "symbols": cache.files[relative_path].symbols,
        }
        for relative_path, score in ranked
        if score > 0
    ]


def find_snippet(root: Path, relative_path: str, query: str) -> tuple[int, str] | None:
    query_terms = set(_query_terms(query))
    path = root / relative_path
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return None

    for line_number, line in enumerate(lines, start=1):
        line_terms = set(tokenize(line))
        if query_terms.intersection(line_terms) or query.lower() in line.lower():
            snippet = " ".join(line.strip().split())
            if len(snippet) > 220:
                snippet = snippet[:217] + "..."
            return line_number, snippet
    return None


def resolve_paths(root_argument: str, cache_argument: str | None) -> tuple[Path, Path]:
    root = Path(root_argument).expanduser().resolve()
    cache = Path(cache_argument or DEFAULT_CACHE_NAME).expanduser()
    if not cache.is_absolute():
        cache = root / cache
    return root, cache


def print_index_summary(cache_path: Path, summary: dict[str, int]) -> None:
    print(
        "Indexed {documents} files ({changed} changed, {reused} reused, "
        "{removed} removed, {skipped} skipped) → {cache}".format(
            cache=cache_path,
            **summary,
        )
    )


def command_index(args: argparse.Namespace) -> int:
    root, cache_path = resolve_paths(args.root, args.cache)
    if not root.is_dir():
        print(f"error: root is not a directory: {root}", file=sys.stderr)
        return 2
    _, summary = build_index(root, cache_path, force=args.force)
    if not args.quiet:
        print_index_summary(cache_path, summary)
    return 0


def command_search(args: argparse.Namespace) -> int:
    root, cache_path = resolve_paths(args.root, args.cache)
    if not root.is_dir():
        print(f"error: root is not a directory: {root}", file=sys.stderr)
        return 2

    if args.no_index:
        cache = Cache.load(cache_path)
    else:
        cache, summary = build_index(root, cache_path, force=False)
        if not args.quiet and summary["changed"] + summary["removed"] > 0:
            print_index_summary(cache_path, summary)

    hits = search(cache, args.query, limit=args.limit)
    if not hits:
        print(f'No matches for "{args.query}".')
        return 1

    for position, hit in enumerate(hits, start=1):
        relative_path = str(hit["path"])
        snippet = find_snippet(root, relative_path, args.query)
        location = f"{relative_path}:{snippet[0]}" if snippet else relative_path
        matched = ", ".join(str(item) for item in hit["matched"])
        print(f"{position}. {location}  score={float(hit['score']):.2f}  [{matched}]")
        if snippet and snippet[1]:
            print(f"   {snippet[1]}")
    return 0


def command_stats(args: argparse.Namespace) -> int:
    root, cache_path = resolve_paths(args.root, args.cache)
    cache = Cache.load(cache_path)
    if cache.root and cache.root != str(root):
        print("Cache belongs to a different root; run `index` first.")
        return 1
    terms = build_postings(cache)
    total_bytes = sum(record.size for record in cache.files.values())
    total_lines = sum(record.lines for record in cache.files.values())
    symbols = sum(len(record.symbols) for record in cache.files.values())
    print(f"cache:       {cache_path}")
    print(f"documents:   {len(cache.files)}")
    print(f"unique terms:{len(terms)}")
    print(f"symbols:     {symbols}")
    print(f"source size: {total_bytes:,} bytes")
    print(f"source lines:{total_lines:,}")
    if cache.indexed_at:
        print(f"indexed at:  {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(cache.indexed_at))}")
    return 0


def command_clear(args: argparse.Namespace) -> int:
    _, cache_path = resolve_paths(args.root, args.cache)
    try:
        cache_path.unlink()
    except FileNotFoundError:
        pass
    except OSError as error:
        print(f"error: cannot remove {cache_path}: {error}", file=sys.stderr)
        return 2
    print(f"Removed {cache_path}")
    return 0


def add_common_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--root", default=".", help="repository root to index (default: current directory)")
    parser.add_argument("--cache", help=f"cache path (default: {DEFAULT_CACHE_NAME} under root)")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Incremental, Cursor-style local codebase index")
    subparsers = parser.add_subparsers(dest="command", required=True)

    index_parser = subparsers.add_parser("index", aliases=["build"], help="scan files and update cache.kv")
    add_common_arguments(index_parser)
    index_parser.add_argument("--force", action="store_true", help="re-tokenize every file")
    index_parser.add_argument("--quiet", action="store_true", help="suppress the summary")

    search_parser = subparsers.add_parser("search", help="search indexed files")
    add_common_arguments(search_parser)
    search_parser.add_argument("query", help="natural-language query or symbol prefix")
    search_parser.add_argument("--limit", type=int, default=10, help="maximum results (default: 10)")
    search_parser.add_argument("--no-index", action="store_true", help="use cache without refreshing it")
    search_parser.add_argument("--quiet", action="store_true", help="suppress refresh summary")

    stats_parser = subparsers.add_parser("stats", help="show cache statistics")
    add_common_arguments(stats_parser)

    clear_parser = subparsers.add_parser("clear", help="remove cache.kv")
    add_common_arguments(clear_parser)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    commands = {
        "index": command_index,
        "build": command_index,
        "search": command_search,
        "stats": command_stats,
        "clear": command_clear,
    }
    try:
        return commands[args.command](args)
    except KeyboardInterrupt:
        print("Interrupted.", file=sys.stderr)
        return 130


if __name__ == "__main__":
    raise SystemExit(main())
