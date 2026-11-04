#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Round-trip and command-line checks for the evidence script format."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import tcl_evidence_format as evidence

HERE = Path(__file__).resolve().parent


class EvidenceFormatTests(unittest.TestCase):
    def test_exact_bytes_round_trip(self) -> None:
        cases = [
            b"",
            b"puts hello\n",
            b"puts hello",
            b"a  \r\nb\t\rc\n",
            'puts "Привет, мир"\n'.encode(),
            'puts "a\u00a0а\u200b"\n'.encode(),
            'puts "é e\u0301\u2028"\n'.encode(),
            'puts "❤️\u200d"\n'.encode(),
            'puts "⟦NBSP⟧ ⟦EOF⟧"\n'.encode(),
            b"## This is a Tcl comment\nputs `abc`\n```\n",
            bytes(range(256)),
        ]
        for script in cases:
            with self.subTest(script=script[:40]):
                visible = evidence.visible_bytes(script)
                self.assertEqual(evidence.original_bytes(visible), script)
                block = evidence.markdown_block(script)
                document = "# Evidence\n\n## Script\n\n" + block + "\n## Results\n"
                self.assertEqual(evidence.script_from_markdown(document), script)

    def test_visible_notation_preserves_readable_language_characters(self) -> None:
        script = 'puts "Привет,\u00a0мир"  \r\n'.encode()
        self.assertEqual(
            evidence.visible_bytes(script),
            'puts "Привет,⟦NBSP⟧мир"⟦SPACE⟧⟦SPACE⟧⟦CRLF⟧\n',
        )

    def test_invisible_unicode_markers_are_visible(self) -> None:
        self.assertEqual(
            evidence.visible_bytes('puts "❤️\u200d"\n'.encode()),
            'puts "❤⟦U+FE0F⟧⟦U+200D⟧"\n',
        )

    def test_rejects_unknown_or_ambiguous_markers(self) -> None:
        for value in [
            "hello⟦UNKNOWN⟧\n",
            "hello⟦EOF⟧\nmore\n",
            "hello \n",
            "hello\t\n",
            "hello\u00a0\n",
            "hello⟦CRLF⟧more\n",
            "hello⟦U+D800⟧\n",
        ]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                evidence.original_bytes(value)

    def test_requires_exactly_one_script_block(self) -> None:
        with self.assertRaises(ValueError):
            evidence.script_from_markdown("## Script\nNo encoded script.\n")
        block = evidence.markdown_block(b"puts ok\n")
        with self.assertRaises(ValueError):
            evidence.script_from_markdown("## Script\n" + block + block)

    def test_evidence_requires_question_assumptions_and_each_result_field(self) -> None:
        sample = (
            HERE.parents[1] / "evidence/tcl/fresh-local-variable-existence.md"
        ).read_text()
        self.assertEqual(evidence.validate_document(sample), [])
        changes = [
            ("## Question", "## Context", "Question"),
            ("## Assumptions", "## Notes", "Assumptions"),
            ("### Tcl 8.4", "### Tcl 8.5", "duplicate environment"),
            ("- Exit status: `0`", "- Status: `0`", "exit status"),
            ("- Standard error: empty.", "- Standard error: fine.", "standard error"),
            (
                "```text-visible\n0 0\n```",
                "```text-visible\n0 0⟦UNKNOWN⟧\n```",
                "invalid standard output",
            ),
        ]
        for before, after, expected in changes:
            with self.subTest(before=before):
                changed = sample.replace(before, after, 1)
                errors = evidence.validate_document(changed)
                self.assertTrue(any(expected in error for error in errors), errors)

    def test_result_text_cannot_be_mistaken_for_evidence_metadata(self) -> None:
        sample = (
            HERE.parents[1] / "evidence/tcl/fresh-local-variable-existence.md"
        ).read_text()
        output = b"- Standard error: injected\n### Not an environment\n"
        replacement = evidence.markdown_block(output, "text-visible").rstrip("\n")
        changed = sample.replace("```text-visible\n0 0\n```", replacement, 1)
        self.assertEqual(evidence.validate_document(changed), [])

    def test_commands_write_and_extract_exact_script(self) -> None:
        script = 'puts "Привет\u00a0мир"  \r\n'.encode() + b"\xff\x00"
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source, block, document, result = (
                root / n for n in ("source", "block.md", "evidence.md", "result")
            )
            source.write_bytes(script)
            subprocess.run(
                [
                    sys.executable,
                    str(HERE / "tcl-script-to-block.py"),
                    str(source),
                    "-o",
                    str(block),
                ],
                check=True,
            )
            document.write_bytes(b"# Evidence\n\n## Script\n\n" + block.read_bytes())
            subprocess.run(
                [
                    sys.executable,
                    str(HERE / "extract-tcl-evidence.py"),
                    str(document),
                    "-o",
                    str(result),
                ],
                check=True,
            )
            self.assertEqual(result.read_bytes(), script)


if __name__ == "__main__":
    unittest.main()
