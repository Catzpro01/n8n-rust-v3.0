import tempfile
import unittest
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

import codebase_index  # noqa: E402


class CodebaseIndexTests(unittest.TestCase):
    def test_tokenize_splits_common_identifier_styles(self):
        tokens = codebase_index.tokenize("CodeBaseMemory cache_key workflowExecution")
        self.assertIn("code", tokens)
        self.assertIn("base", tokens)
        self.assertIn("memory", tokens)
        self.assertIn("cache", tokens)
        self.assertIn("key", tokens)
        self.assertIn("workflow", tokens)
        self.assertIn("execution", tokens)

    def test_cache_round_trip(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            cache_path = Path(temporary_directory) / "cache.kv"
            original = codebase_index.Cache(
                root="/tmp/project",
                indexed_at=123,
                files={
                    "src/main.rs": codebase_index.FileRecord(
                        size=42,
                        mtime_ns=456,
                        digest="abc123",
                        lines=3,
                        terms={"workflow": 2, "run": 1},
                        symbols=["run_workflow"],
                    )
                },
            )
            original.save(cache_path)
            loaded = codebase_index.Cache.load(cache_path)
            self.assertEqual(loaded.root, original.root)
            self.assertEqual(loaded.indexed_at, original.indexed_at)
            self.assertEqual(loaded.files["src/main.rs"].terms, {"workflow": 2, "run": 1})
            self.assertEqual(loaded.files["src/main.rs"].symbols, ["run_workflow"])
            self.assertEqual(loaded.postings["workflow"], {"src/main.rs": 2})

    def test_incremental_index_reuses_unchanged_files_and_searches_symbols(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory) / "repo"
            source = root / "src" / "main.rs"
            source.parent.mkdir(parents=True)
            source.write_text("fn run_workflow() {\n    println!(\"ready\");\n}\n", encoding="utf-8")
            cache_path = root / "cache.kv"

            first_cache, first_summary = codebase_index.build_index(root, cache_path)
            self.assertEqual(first_summary["changed"], 1)
            self.assertEqual(first_summary["reused"], 0)

            cache_mtime = cache_path.stat().st_mtime_ns
            second_cache, second_summary = codebase_index.build_index(root, cache_path)
            self.assertEqual(second_summary["changed"], 0)
            self.assertEqual(second_summary["reused"], 1)
            self.assertEqual(cache_path.stat().st_mtime_ns, cache_mtime)

            hits = codebase_index.search(second_cache, "run workflow")
            self.assertEqual(hits[0]["path"], "src/main.rs")
            self.assertTrue(any(symbol.startswith("run") for symbol in hits[0]["symbols"]))
            self.assertEqual(first_cache.files.keys(), second_cache.files.keys())


if __name__ == "__main__":
    unittest.main()
