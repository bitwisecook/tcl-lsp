#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check BIG-IP fixture and response handling without evidence digests."""

from __future__ import annotations

import base64
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts/dev/bigip-probes/resolution-2286"


class BigIPEvidenceTests(unittest.TestCase):
    def test_context_generator_preserves_payload_without_a_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            payload = b'puts "a b"\n'
            source = root / "payload.tcl"
            source.write_bytes(payload)
            out = root / "fixtures"
            subprocess.run(
                [
                    "python3",
                    str(SCRIPTS / "generate-followup-contexts.py"),
                    "--run",
                    "test1",
                    "--payload",
                    str(source),
                    "--out",
                    str(out),
                    "--source-commit",
                    "test",
                ],
                check=True,
                capture_output=True,
            )
            self.assertEqual((out / "payload.tcl").read_bytes(), payload)
            manifest = json.loads((out / "manifest.json").read_bytes())
            self.assertEqual(manifest["payload"]["hex"], payload.hex())
            self.assertNotIn("sha256", json.dumps(manifest).lower())
            cli = (out / "cli.conf").read_text()
            self.assertIn(payload.hex(), cli)
            self.assertNotIn("payload_sha256", cli)

    def test_followup_summary_reads_bytes_with_a_stale_legacy_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            traffic = root / "traffic"
            traffic.mkdir()
            raw = b"HTTP/1.1 200 OK\r\n\r\n0 6162"
            record = {
                "response_base64": base64.b64encode(raw).decode("ascii"),
                "response_sha256": "stale historical value",
                "status": 200,
                "reported_units": ["0:1"],
            }
            (traffic / "case.jsonl").write_text(json.dumps(record) + "\n")
            out = root / "summary.json"
            subprocess.run(
                [
                    "python3",
                    str(SCRIPTS / "summarize-followup-evidence.py"),
                    "--traffic-dir",
                    str(traffic),
                    "--out",
                    str(out),
                ],
                check=True,
                capture_output=True,
            )
            summary = json.loads(out.read_bytes())
            body = summary["phases"][0]["unique_response_bodies"][0]
            self.assertEqual(bytes.fromhex(body["body_hex"]), b"0 6162")
            self.assertNotIn("sha256", json.dumps(summary).lower())


if __name__ == "__main__":
    unittest.main()
