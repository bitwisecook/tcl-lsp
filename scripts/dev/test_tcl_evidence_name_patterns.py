#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Exercise byte comparisons and legacy capture parsing without digests."""

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent / "bigip-probes/name-patterns"


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class NamePatternEvidenceTests(unittest.TestCase):
    def test_staged_configuration_is_compared_as_exact_bytes(self):
        capture = load("capture")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            config = "literal $value\nTCL_LSP_EXPECTED_CONFIG\nnext\n"
            staged = root / "configuration with spaces"
            staged.write_text(config)
            shell = (
                f'E="{root}"\n'
                'run() { label=$1; shift; "$@" > "$E/$label.stdout" '
                '2> "$E/$label.stderr"; echo $? > "$E/$label.exit"; }\n'
                + capture.config_comparison(str(staged), config)
            )
            self.assertEqual(
                subprocess.run(["bash", "-c", shell], check=False).returncode, 0
            )
            staged.write_text(config.replace("next", "changed"))
            self.assertEqual(
                subprocess.run(["bash", "-c", shell], check=False).returncode, 4
            )

    def test_legacy_digest_does_not_override_exact_source_comparison(self):
        coverage = load("coverage")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "source.tcl").write_text("puts ok\n")
            (root / "actual.tcl").write_text("puts ok\n")
            (root / "wrapper.tcl").write_text("wrapper\n")
            (root / "cases.json").write_text(
                json.dumps(
                    [
                        {
                            "exact_source": True,
                            "adaptation": None,
                            "operation": "puts ok\n",
                        }
                    ]
                )
            )
            question = {
                "canonical_question_id": "q",
                "source": {"path": "source.tcl", "sha256": "stale"},
            }
            record = {
                "canonical_question_id": "q",
                "adaptation": None,
                "status": "observed",
                "setup": {"completion_code": 0},
                "lookup": {"reached": True, "completion_code": 0, "result_hex": ""},
                "case_metadata": {
                    "path": "cases.json",
                    "json_pointer": "/0",
                    "sha256": "stale",
                },
                "original_input": {"path": "source.tcl", "sha256": "stale"},
                "actual_input": {"path": "actual.tcl"},
                "wrapper": {"path": "wrapper.tcl"},
            }
            self.assertTrue(coverage._candidate(record, question, root))
            (root / "source.tcl").write_text("puts changed\n")
            self.assertFalse(coverage._candidate(record, question, root))

    def test_parser_preserves_result_bytes_without_source_digest(self):
        parser = load("parse_capture")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "log").write_text(
                "NHO|run|case|ctx|lookup|0|0:1|result|utf-8|0|1|610062\n"
            )
            result = parser.parse_capture(["log"], "ctx", "run", root)
            self.assertEqual(result["sources"], [{"path": "log"}])
            self.assertEqual(bytes.fromhex(result["fields"][0]["value_hex"]), b"a\x00b")


if __name__ == "__main__":
    unittest.main()
