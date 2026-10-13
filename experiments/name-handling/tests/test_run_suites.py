# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Admission controls for original-suite oracle captures."""

import hashlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

RUNNER = Path(__file__).resolve().parents[1] / "run_suites.py"
SPEC = importlib.util.spec_from_file_location("run_suites", RUNNER)
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class OriginalInputs(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / "source"
        inputs = {
            "generic/tcl.h": b'#define TCL_PATCH_LEVEL "9.0.4"\n',
            "library/init.tcl": b"set auto_path {}\n",
            "library/tcltest/tcltest.tcl": b"package provide tcltest 2.5.11\n",
            "library/tcltest/pkgIndex.tcl": b"original package index\n",
            "tests/all.tcl": b"original orchestration\n",
            "tests/namespace.test": b"original namespace assertions\n",
            "tests/support/helper.tcl": b"original support\n",
        }
        blobs = {}
        for name, data in inputs.items():
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            blobs[name] = hashlib.sha1(
                b"blob " + str(len(data)).encode() + b"\0" + data
            ).hexdigest()
        self.receipt = self.root / "upstream.json"
        self.receipt.write_text(json.dumps({"git_blob_sha1": blobs}))

    def test_generated_build_files_do_not_change_original_admission(self):
        (self.source / "library/build-output").write_text("generated")
        _, hashes = runner.validate_upstream(self.source, self.receipt)
        self.assertIn("tests/support/helper.tcl", hashes)
        self.assertNotIn("library/build-output", hashes)

    def test_changed_original_harness_suite_and_support_are_rejected(self):
        for name in [
            "library/init.tcl",
            "library/tcltest/tcltest.tcl",
            "library/tcltest/pkgIndex.tcl",
            "tests/all.tcl",
            "tests/namespace.test",
            "tests/support/helper.tcl",
        ]:
            with self.subTest(name=name):
                path = self.source / name
                original = path.read_bytes()
                path.write_bytes(original + b"# modified\n")
                with self.assertRaisesRegex(ValueError, "modified upstream input"):
                    runner.validate_upstream(self.source, self.receipt)
                path.write_bytes(original)

    def test_missing_original_support_is_rejected(self):
        (self.source / "tests/support/helper.tcl").unlink()
        with self.assertRaisesRegex(ValueError, "missing original upstream input"):
            runner.validate_upstream(self.source, self.receipt)

    def test_wrong_binary_release_library_and_harness_startup_are_rejected(self):
        binary = self.source / "unix/tclsh"
        library = self.source / "library"
        outputs = [
            (f"9.0\n9.0.3\n{library}\n2.5.11\n".encode(), b""),
            (b"9.0\n9.0.4\n/wrong/library\n2.5.11\n", b""),
            (f"9.0\n9.0.4\n{library}\n2.5.11\n".encode(), b"harness error"),
        ]
        for stdout, stderr in outputs:
            with self.subTest(stdout=stdout, stderr=stderr):
                result = subprocess.CompletedProcess([str(binary)], 0, stdout, stderr)
                with (
                    patch.object(runner.subprocess, "run", return_value=result),
                    self.assertRaises(ValueError),
                ):
                    runner.validate_oracle(self.source, binary, "9.0", "9.0.4", 10)

    def test_interpreter_wrappers_are_rejected_before_execution(self):
        with patch.object(runner.subprocess, "run") as execute:
            with self.assertRaisesRegex(ValueError, "built unix/tclsh"):
                runner.validate_oracle(
                    self.source, self.root / "wrapper", "9.0", "9.0.4", 10
                )
            execute.assert_not_called()

    def invoke(self, output):
        return subprocess.run(
            [
                "python3",
                str(RUNNER),
                "--tcl-root",
                str(self.source),
                "--tclsh",
                str(self.source / "unix/tclsh"),
                "--version",
                "unsupported",
                "--output",
                str(output),
            ],
            capture_output=True,
            check=False,
        )

    def test_invalid_reference_creates_no_capture(self):
        output = self.root / "capture"
        result = self.invoke(output)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(output.exists())

    def test_prior_evidence_is_not_overwritten(self):
        output = self.root / "capture"
        output.mkdir()
        evidence = output / "stdout.txt"
        evidence.write_bytes(b"retained measurement\n")
        result = self.invoke(output)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"preserve existing measurements", result.stderr)
        self.assertEqual(evidence.read_bytes(), b"retained measurement\n")


if __name__ == "__main__":
    unittest.main()
