"""Regression coverage for the daemon-startup diagnostic.

When the daemon dies during startup, ``Daemon.__init__`` used to poll the health
endpoint for the full 20s budget and only then report whatever it had, which
made a startup failure look like a slow timeout and hid the reason the daemon
gave. That is exactly what made the ``eco-acceptance`` CI failure unreadable:
the step ended after 24s against a 356s healthy run, and nothing reachable
explained why.

The harness must fail as soon as the process is gone and carry the daemon's own
explanation, including its exit code.
"""

from __future__ import annotations

import stat
import tempfile
import time
import unittest
from pathlib import Path

from tests.acceptance import test_run_manual_trigger as trigger


class DaemonStartupDiagnostics(unittest.TestCase):
    def setUp(self) -> None:
        self._directory = tempfile.TemporaryDirectory()
        self.addCleanup(self._directory.cleanup)
        root = Path(self._directory.name)
        self.state = root / "state"
        self.state.mkdir()
        self.key = root / "master.key"
        self.key.write_text("regression-test-key")
        self._original_bin = trigger.BIN
        self.addCleanup(self._restore_bin)

    def _restore_bin(self) -> None:
        trigger.BIN = self._original_bin

    def _fake_daemon(self, body: str) -> None:
        """Point the harness at a stand-in daemon that behaves as described."""
        script = Path(self._directory.name) / "fake-workflowd.sh"
        script.write_text(body)
        script.chmod(script.stat().st_mode | stat.S_IEXEC)
        trigger.BIN = script

    def test_dead_daemon_fails_fast_and_reports_its_reason(self) -> None:
        self._fake_daemon(
            "#!/bin/sh\n"
            'echo "workflowd: cannot bind 127.0.0.1:41234: address already in use" >&2\n'
            "exit 3\n"
        )
        started = time.monotonic()
        with self.assertRaises(AssertionError) as caught:
            trigger.Daemon(self.state, self.key)
        elapsed = time.monotonic() - started
        message = str(caught.exception)

        self.assertIn("address already in use", message, message)
        self.assertIn("exited with code 3", message, message)
        self.assertLess(
            elapsed,
            5.0,
            "a daemon that is already dead must not burn the 20s health budget "
            f"(took {elapsed:.1f}s)",
        )


if __name__ == "__main__":
    unittest.main()
