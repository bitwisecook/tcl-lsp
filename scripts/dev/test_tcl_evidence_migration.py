#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check that legacy migration does not invent or misattribute executions."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

import tcl_evidence_format as evidence

SCRIPT = Path(__file__).with_name("migrate-tcl-evidence.py")
SPEC = importlib.util.spec_from_file_location("migration", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise ImportError(f"cannot load {SCRIPT}")
migration = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(migration)


class EvidenceMigrationTests(unittest.TestCase):
    def fixture(self, root: Path) -> tuple[dict, bytes, bytes]:
        script = 'puts "Привет\u00a0мир"\n'.encode()
        stdout = "Привет\u00a0мир\n".encode()
        sources = {
            "run/script.tcl": script,
            "run/stdout": stdout,
            "run/stderr": b"",
        }
        for name, data in sources.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)

        def item(identifier: str, role: str, path: str) -> dict:
            return {
                "id": identifier,
                "role": role,
                "path": path,
            }

        receipt = {
            "exit": 0,
        }
        sources["run/receipt.json"] = json.dumps(receipt).encode()
        (root / "run/receipt.json").write_bytes(sources["run/receipt.json"])
        record = {
            "id": "naming.example",
            "kind": "native-observation",
            "problem_statement": "The VM needs the correct result for this input.",
            "question": "What does this exact script return on this Tcl environment?",
            "scope": "Only the retained run was measured.",
            "evidence": [
                item("script", "input", "run/script.tcl"),
                item("out", "observation", "run/stdout"),
                item("err", "observation", "run/stderr"),
                item("receipt", "provider", "run/receipt.json"),
            ],
            "providers": [
                {
                    "id": "tcl8.6",
                    "version": "8.6.18",
                    "status": "observed",
                    "input_channel": "source file",
                    "evidence": ["script", "out", "err", "receipt"],
                }
            ],
        }
        return record, script, stdout

    def test_migrates_one_unambiguous_capture_exactly(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            record, script, stdout = self.fixture(root)
            path = root / migration.LEGACY
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps({"proofs": [record]}))
            report = migration.migrate(root, root / "evidence/tcl", True)
            self.assertEqual(
                (report["converted_records"], report["needs_review"]), (1, 0)
            )
            document = next((root / "evidence/tcl").glob("*.md")).read_text()
            self.assertEqual(evidence.validate_document(document), [])
            self.assertEqual(evidence.script_from_markdown(document), script)
            results = evidence.sections(evidence.sections(document)["Results"][0], 3)
            block = evidence.fenced_blocks(results["tcl8.6"][0], "text-visible")[0]
            self.assertEqual(evidence.original_bytes(block), stdout)
            self.assertIn("- Exit status: 0", document)
            self.assertNotIn("sha256", document)

    def test_does_not_guess_a_missing_script(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            record, _, _ = self.fixture(root)
            (root / "run/script.tcl").unlink()
            path = root / migration.LEGACY
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps({"proofs": [record]}))
            report = migration.migrate(root, root / "evidence/tcl", True)
            self.assertEqual(
                (report["converted_records"], report["needs_review"]), (0, 1)
            )
            self.assertFalse((root / "evidence/tcl").exists())

    def test_missing_receipt_does_not_invent_an_exit_status(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            record, _, _ = self.fixture(root)
            (root / "run/receipt.json").unlink()
            refs = {e["id"]: e for e in record["evidence"]}
            captured, reason = migration.capture_for_provider(
                root, record["providers"][0], refs
            )
            self.assertIsNone(reason)
            self.assertIsNotNone(captured)
            self.assertEqual(captured[-1], "Not recorded")


if __name__ == "__main__":
    unittest.main()
