#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check BIG-IP fixture and response handling without evidence digests."""

from __future__ import annotations

import base64
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts/dev/bigip-probes/resolution-2286"


class BigIPEvidenceTests(unittest.TestCase):
    def test_appliance_cleanup_compares_current_configuration_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixtures = root / "fixtures"
            fixtures.mkdir()
            journal = root / "journal"
            state = root / "state"
            (fixtures / "rules.tsv").write_text(
                "probe.conf\t/Common/__tcl_lsp_probe_2286_test\n"
            )
            (fixtures / "probe.conf").write_text("original configuration\n")
            bin_dir = root / "bin"
            bin_dir.mkdir()
            tmsh = bin_dir / "tmsh"
            tmsh.write_text(
                "#!/usr/bin/env bash\n"
                "if [[ $1 == list ]]; then\n"
                '  if [[ -f "$TEST_RULE_STATE" ]]; then cat "$TEST_RULE_STATE"; '
                'else echo "was not found" >&2; exit 1; fi\n'
                'elif [[ $1 == load ]]; then cp "$6" "$TEST_RULE_STATE"\n'
                'elif [[ $1 == delete ]]; then rm "$TEST_RULE_STATE"\n'
                "else exit 9; fi\n"
            )
            tmsh.chmod(0o755)
            environment = dict(
                os.environ,
                PATH=f"{bin_dir}:{os.environ['PATH']}",
                TEST_RULE_STATE=str(state),
            )
            driver = str(SCRIPTS / "appliance-rules.sh")
            load = ["bash", driver, "load", str(fixtures), str(journal), "probe"]
            cleanup = ["bash", driver, "cleanup", str(fixtures), str(journal)]
            subprocess.run(load, env=environment, check=True, capture_output=True)
            self.assertEqual(
                (journal / "owned.tsv").read_text(),
                "/Common/__tcl_lsp_probe_2286_test\tprobe\n",
            )
            state.write_text("changed configuration\n")
            refused = subprocess.run(
                cleanup, env=environment, check=False, capture_output=True
            )
            self.assertEqual(refused.returncode, 3)
            self.assertEqual(state.read_text(), "changed configuration\n")
            state.write_text("original configuration\n")
            # Existing journals stored a digest in this column; the fixture map
            # and captured original configuration still permit exact comparison.
            (journal / "owned.tsv").write_text(
                "/Common/__tcl_lsp_probe_2286_test\t" + "a" * 64 + "\n"
            )
            subprocess.run(cleanup, env=environment, check=True, capture_output=True)
            self.assertFalse(state.exists())

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

    def test_chunk_inventory_retains_bytes_and_marks_incomplete_sequences(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "log.txt"
            source.write_text(
                "R2286LBC|run=a|ctx=RULE_INIT|group_rc=0|group=0|unit_rc=0|unit=1|session=one|seq=0/2|hex=6100\n"
                "R2286LBC|run=a|ctx=RULE_INIT|group_rc=0|group=0|unit_rc=0|unit=1|session=one|seq=1/2|hex=62\n"
                "R2286LBC|run=a|ctx=RULE_INIT|group_rc=0|group=0|unit_rc=0|unit=1|session=two|seq=0/2|hex=63\n"
            )
            out = root / "inventory.json"
            subprocess.run(
                [
                    "python3",
                    str(SCRIPTS / "inventory-lexical-byte-class-logs.py"),
                    str(source),
                    str(out),
                ],
                check=True,
                capture_output=True,
            )
            rows = {row["session"]: row for row in json.loads(out.read_bytes())}
            self.assertTrue(rows["one"]["complete"])
            self.assertEqual(bytes.fromhex(rows["one"]["assembled_hex"]), b"a\x00b")
            self.assertFalse(rows["two"]["complete"])
            self.assertIsNone(rows["two"]["assembled_hex"])


if __name__ == "__main__":
    unittest.main()
