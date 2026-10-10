#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fixed negative controls for proof integrity and coverage certification."""

import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-name-resolution-proofs.py")
SPEC = importlib.util.spec_from_file_location("proof_gate", SCRIPT)
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)
REPOSITORY = SCRIPT.resolve().parents[2]
DIRECTORY = gate.DIRECTORY


class ProofGateTests(unittest.TestCase):
    def fixture(self, root):
        directory = root / DIRECTORY
        directory.mkdir(parents=True)
        for name in ["schema.json", "record-template.json"]:
            (directory / name).write_bytes((REPOSITORY / DIRECTORY / name).read_bytes())
        record = json.loads((directory / "record-template.json").read_text())
        record.update(
            id="naming.fixture.actual-result",
            document=str(DIRECTORY / "fixture.md"),
            problem_statement="A synthetic fixture tests whether changed evidence can keep a valid digest claim.",
            question="Does the synthetic captured row return A?",
            conclusion="Only the synthetic recorded row returns A.",
            scope="Validator unit-test fixture; not a native observation.",
        )
        path = root / "rust/tcl-syntax/tests/data/native_fixture/manifest.json"
        path.parent.mkdir(parents=True)
        path.write_text('{"rows":[{"result":"A"}]}\n')
        record["evidence"] = [
            {
                "id": "row",
                "role": "observation",
                "path": str(path.relative_to(root)),
                "sha256": gate.digest(path),
                "description": "Synthetic row",
                "json_pointer": "/rows/0",
            }
        ]
        record["providers"][0].update(
            status="observed",
            answer="The synthetic fixture returns A.",
            evidence=["row"],
        )
        text = "# Synthetic fixture\n\n" + record["id"] + "\n\n"
        for heading, field in [
            ("Problem statement", "problem_statement"),
            ("Question", "question"),
            ("Conclusion", "conclusion"),
            ("Scope", "scope"),
        ]:
            text += f"## {heading}\n\n{record[field]}\n\n"
        text += "\n".join(provider["answer"] for provider in record["providers"])
        (root / record["document"]).write_text(text)
        (directory / "manifest.json").write_text(
            json.dumps({"schema_version": 1, "proofs": [record]})
        )
        census = gate.inventory(root)
        census["files"][0].update(
            classification="native-observation",
            proof_ids=[record["id"]],
            reason="Synthetic observation used only by gate tests.",
        )
        (directory / "corpus-inventory.json").write_text(json.dumps(census))
        return record, path, census

    def write_record_fixture(self, root, record, census):
        document = "# Synthetic validator fixture\n\n" + record["id"] + "\n\n"
        for heading, field in [
            ("Problem statement", "problem_statement"),
            ("Question", "question"),
            ("Conclusion", "conclusion"),
            ("Scope", "scope"),
        ]:
            document += f"## {heading}\n\n{record[field]}\n\n"
        document += "\n".join(provider["answer"] for provider in record["providers"])
        document += "\n".join(item["answer"] for item in record.get("environments", []))
        for capture in record.get("attempt", {}).get("captures", []):
            document += "\n" + capture["failure_stage"]
        (root / record["document"]).write_text(document)
        (root / DIRECTORY / "manifest.json").write_text(
            json.dumps({"schema_version": 1, "proofs": [record]})
        )
        refreshed = gate.inventory(root, census)
        roles = {item["path"]: item["role"] for item in record["evidence"]}
        for row in refreshed["files"]:
            if row["path"] in roles:
                row.update(
                    classification=(
                        "native-observation"
                        if roles[row["path"]] == "observation"
                        else "provider-scaffolding"
                    ),
                    proof_ids=[record["id"]],
                    reason="Fixed synthetic validator input; no original provider observation.",
                )
        (root / DIRECTORY / "corpus-inventory.json").write_text(json.dumps(refreshed))
        return gate.validate(root)[0]

    def native_attempt_fixture(self, root):
        record, original, census = self.fixture(root)
        limits = (
            "The intended question was not reached. The setup captures cannot be used "
            "as a positive answer or as execution, teardown or implementation equivalence evidence."
        )
        record.update(
            kind="native-attempt",
            question="Does the callback execute during the intended teardown?",
            conclusion="The synthetic driver records a setup error. " + limits,
        )
        for provider in record["providers"]:
            provider.update(status="not-tested", evidence=[])
        record["providers"][0].update(version="synthetic", evidence=["probe", "input", "receipt", "stdout", "stderr"])
        probe = original.with_name("probe.c")
        probe.write_text("/* Synthetic setup driver; no original provider claim. */\n")
        stdout = original.with_name("execute.stdout")
        row = "ORIGINAL|1|" + b"setup refused".hex()
        rows = ["VERSION|0|" + b"synthetic".hex(), row]
        stdout.write_text("\n".join(rows) + "\n")
        stderr = original.with_name("execute.stderr")
        stderr.write_text("")
        receipt = original.with_name("receipt.json")
        receipt.write_text(json.dumps({
            "exit": 0,
            "rows": rows,
            "stdout_sha256": gate.digest(stdout),
            "stderr_sha256": gate.digest(stderr),
            "source_sha256": {"original": gate.digest(original)},
            "required_sha256": {"probe": gate.digest(probe)},
        }))
        record["evidence"] = [
            {
                "id": key,
                "role": role,
                "path": str(path.relative_to(root)),
                "sha256": gate.digest(path),
                "description": "Fixed synthetic setup evidence; not an original provider capture.",
            }
            for key, role, path in [
                ("probe", "input", probe),
                ("input", "input", original),
                ("receipt", "provider", receipt),
                ("stdout", "observation", stdout),
                ("stderr", "observation", stderr),
            ]
        ]
        record["attempt"] = {
            "intended_question": record["question"],
            "intended_question_reached": False,
            "probe": "probe",
            "limitations": limits,
            "captures": [{
                "provider": "tcl8.4",
                "input": "input",
                "receipt": "receipt",
                "observation": "stdout",
                "stderr": "stderr",
                "guest_code": 1,
                "completion_row": row,
                "failure_stage": "Setup refuses before registering the callback or entering teardown.",
            }],
        }
        return record, census

    def test_changed_bytes_and_missing_rows_are_rejected(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, path, _ = self.fixture(root)
            self.assertEqual(gate.validate(root)[0], [])
            path.write_text('{"rows":[{"result":"B"}]}\n')
            self.assertTrue(
                any("digest changed" in error for error in gate.validate(root)[0])
            )
            record["evidence"][0]["sha256"] = gate.digest(path)
            record["evidence"][0]["json_pointer"] = "/rows/1"
            (root / DIRECTORY / "manifest.json").write_text(
                json.dumps({"schema_version": 1, "proofs": [record]})
            )
            self.assertTrue(gate.validate(root, True)[0])

    def test_source_evidence_uses_lf_coordinates_and_retains_carriage_returns(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, _, census = self.fixture(root)
            source = root / "rust/tcl-syntax/tests/data/native_fixture/source.c"
            source.write_bytes(
                b"prefix\fbytes\r\nint target(void) {\r\n    return 1;\r\n}\r\nsuffix"
            )
            snippet = "int target(void) {\r\n    return 1;\r\n}\r\n"
            record["kind"] = "source-anchor"
            record["providers"][0]["status"] = "inspected"
            record["evidence"].append(
                {
                    "id": "source",
                    "role": "source-anchor",
                    "path": str(source.relative_to(root)),
                    "sha256": gate.digest(source),
                    "description": "Synthetic LF coordinate and original CR fixture.",
                    "lines": [2, 4],
                }
            )
            record["source_anchors"] = [
                {
                    "provider": "tcl8.4",
                    "version": "synthetic",
                    "revision": "Validator fixture; no native source claim.",
                    "original_path": "synthetic.c",
                    "function": "target",
                    "lines": [2, 4],
                    "source_sha256": gate.digest(source),
                    "snippet": snippet,
                    "snippet_sha256": hashlib.sha256(snippet.encode()).hexdigest(),
                    "evidence": "source",
                }
            ]
            manifest = root / DIRECTORY / "manifest.json"

            def validate():
                manifest.write_text(
                    json.dumps({"schema_version": 1, "proofs": [record]})
                )
                refreshed = gate.inventory(root, census)
                for row in refreshed["files"]:
                    if row["path"] == str(source.relative_to(root)):
                        row.update(
                            classification="provider-scaffolding",
                            proof_ids=[record["id"]],
                            reason="Synthetic source for LF coordinate validator coverage.",
                        )
                (root / DIRECTORY / "corpus-inventory.json").write_text(
                    json.dumps(refreshed)
                )
                return gate.validate(root)[0]

            self.assertEqual(validate(), [])
            record["evidence"][-1]["lines"] = [1, 3]
            self.assertTrue(
                any("pinned snippet is absent" in error for error in validate())
            )
            record["evidence"][-1]["lines"] = [5, 5]
            record["source_anchors"][0].update(
                lines=[5, 5],
                snippet="suffix",
                snippet_sha256=hashlib.sha256(b"suffix").hexdigest(),
            )
            self.assertEqual(validate(), [])
            record["evidence"][-1]["lines"] = [6, 6]
            self.assertTrue(
                any("invalid evidence line extent" in error for error in validate())
            )

    def test_pending_capture_cannot_certify_complete_coverage(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, _, census = self.fixture(root)
            census["files"][0]["classification"] = "pending"
            (root / DIRECTORY / "corpus-inventory.json").write_text(json.dumps(census))
            self.assertTrue(
                any("still require" in error for error in gate.validate(root)[0])
            )
            errors, summary = gate.validate(root, True)
            self.assertEqual(errors, [])
            self.assertEqual(summary["pending_candidates"], 1)

    def test_pointer_root_and_escaped_keys_remain_distinct(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        value = {"": "empty-key", "a/b": {"x~y": ["row"]}}
        self.assertIs(gate.pointer(value, ""), value)
        self.assertEqual(gate.pointer(value, "/"), "empty-key")
        self.assertEqual(gate.pointer(value, "/a~1b/x~0y/0"), "row")
        with self.assertRaises(ValueError):
            gate.pointer(value, "/a~1b/x~0y/00")

    def test_json_capture_cannot_escape_review_by_clearing_candidate(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, _, census = self.fixture(root)
            census["files"][0].update(candidate=False, classification="pending")
            (root / DIRECTORY / "corpus-inventory.json").write_text(json.dumps(census))
            for audit in [False, True]:
                errors, _ = gate.validate(root, audit)
                self.assertTrue(
                    any("candidate scope changed" in error for error in errors)
                )

    def test_paths_cannot_escape_the_repository(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repository"
            root.mkdir()
            outside = root.parent / "outside.txt"
            outside.write_text("A")
            (root / "escape").symlink_to(outside)
            for name in ["../outside.txt", str(outside), "escape"]:
                with self.assertRaises(ValueError):
                    gate.relative_file(root, name)

    def test_missing_capture_is_retained_as_unavailable(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, path, census = self.fixture(root)
            original = gate.digest(path)
            path.unlink()
            refreshed = gate.inventory(root, census)
            self.assertEqual(len(refreshed["files"]), 1)
            row = refreshed["files"][0]
            self.assertEqual(row["sha256"], original)
            self.assertEqual(row["classification"], "unavailable")
            (root / DIRECTORY / "corpus-inventory.json").write_text(
                json.dumps(refreshed)
            )
            self.assertTrue(
                any("unavailable" in error for error in gate.validate(root, True)[0])
            )

    def test_generated_python_cache_is_not_a_capture(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, path, _ = self.fixture(root)
            cache = path.parent / "__pycache__" / "replay.cpython-312.pyc"
            cache.parent.mkdir()
            cache.write_bytes(b"generated Python bytecode")
            retained = cache.parent / "failed-capture.json"
            retained.write_text('{"status":1,"stderr":"actual refusal"}')
            files = gate.corpus_files(root)
            self.assertNotIn(cache, files)
            self.assertIn(retained, files)
            self.assertIn(path, files)

    def test_adjacent_function_cannot_donate_its_proof_comment(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        source = 'fn first() { /* naming.fixture.first */ let s = "}"; }\n#[test]\nfn second() { assert!(true); }'
        self.assertNotIn("naming.fixture.first", gate.test_body(source, "second"))
        self.assertIsNone(gate.test_body("// fn absent() {}\nfn actual() {}", "absent"))
        self.assertIsNone(
            gate.test_body(
                "/* nested /* fn absent() {} */ comment */ fn actual() {}", "absent"
            )
        )
        self.assertIsNone(
            gate.test_body(
                'const S: &str = r###"fn absent() {}"###; fn actual() {}', "absent"
            )
        )
        self.assertIsNotNone(
            gate.test_body("fn actual() { let c = '}'; assert!(true); }", "actual")
        )
        self.assertIsNone(
            gate.test_body("mod a { fn same() {} } mod b { fn same() {} }", "same")
        )

    def test_missing_provider_and_native_donation_to_rust_contract_fail(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, _, _ = self.fixture(root)
            record["kind"] = "implementation-contract"
            (root / DIRECTORY / "manifest.json").write_text(
                json.dumps({"schema_version": 1, "proofs": [record]})
            )
            self.assertTrue(
                any("cannot issue" in error for error in gate.validate(root)[0])
            )
            record["providers"].pop()
            (root / DIRECTORY / "manifest.json").write_text(
                json.dumps({"schema_version": 1, "proofs": [record]})
            )
            self.assertTrue(
                any("wrong array length" in error for error in gate.validate(root)[0])
            )

    def test_classification_merge_cannot_accept_changed_input_or_unknown_question(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        spec = importlib.util.spec_from_file_location(
            "proof_merge", SCRIPT.with_name("update-name-resolution-proof-index.py")
        )
        merger = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(merger)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, path, census = self.fixture(root)
            update = dict(census["files"][0])
            update["proof_ids"] = ["naming.fixture.unissued"]
            before = (root / DIRECTORY / "corpus-inventory.json").read_bytes()
            with self.assertRaises(ValueError):
                merger.merge_classifications(root, [update], gate)
            self.assertEqual(
                (root / DIRECTORY / "corpus-inventory.json").read_bytes(), before
            )
            update["proof_ids"] = [record["id"]]
            path.write_text('{"rows":[{"result":"changed"}]}')
            with self.assertRaises(ValueError):
                merger.merge_classifications(root, [update], gate)
            self.assertEqual(
                (root / DIRECTORY / "corpus-inventory.json").read_bytes(), before
            )

    def test_unindexed_question_page_and_rust_annotation_block_certification(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            page = root / DIRECTORY / "unindexed.md"
            page.write_text(
                "# naming.fixture.unindexed\n\nA question without an index entry.\n"
            )
            source = root / "rust/tcl-syntax/src/fixture.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "// Native proof: naming.fixture.unindexed\n#[test]\nfn result() { assert!(true); }\n"
            )
            errors, _ = gate.validate(root, True)
            self.assertTrue(any("document is not indexed" in error for error in errors))
            self.assertTrue(
                any("annotation is not indexed" in error for error in errors)
            )
            source.write_text(
                "// Proof naming.fixture.unindexed:\n#[test]\nfn result() { assert!(true); }\n"
            )
            self.assertTrue(
                any(
                    "annotation is not indexed" in error
                    for error in gate.validate(root, True)[0]
                )
            )
            page.unlink()
            source.write_text(
                "// Proof naming.fixture.actual-result:\n#[test]\nfn result() { assert!(true); }\n"
            )
            self.assertEqual(gate.validate(root)[0], [])

    def test_scope_and_conclusion_cannot_borrow_partial_heading_names(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, _, _ = self.fixture(root)
            path = root / record["document"]
            original = path.read_text()
            path.write_text(original.replace("## Scope\n", "## Scoped conclusion\n"))
            self.assertTrue(
                any(
                    "omits exact scope" in error
                    for error in gate.validate(root, True)[0]
                )
            )
            path.write_text(
                original.replace("## Conclusion\n", "## Conclusion details\n")
            )
            self.assertTrue(
                any(
                    "omits exact conclusion" in error
                    for error in gate.validate(root, True)[0]
                )
            )
            path.write_text(original)
            self.assertEqual(gate.validate(root)[0], [])

    def test_source_inspection_cannot_borrow_execution_or_another_provider(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, path, _ = self.fixture(root)
            record["kind"] = "source-anchor"
            record["source_anchors"] = [
                {
                    "provider": "tcl8.4",
                    "version": "synthetic",
                    "revision": "Synthetic validator fixture; no native source claim.",
                    "original_path": "synthetic.c",
                    "function": "synthetic",
                    "lines": [1, 1],
                    "source_sha256": gate.digest(path),
                    "snippet": "A",
                    "snippet_sha256": hashlib.sha256(b"A").hexdigest(),
                    "evidence": "row",
                }
            ]
            manifest = root / DIRECTORY / "manifest.json"
            manifest.write_text(json.dumps({"schema_version": 1, "proofs": [record]}))
            self.assertTrue(
                any(
                    "cannot issue a guest execution answer" in error
                    for error in gate.validate(root)[0]
                )
            )
            record["providers"][0]["status"] = "inspected"
            manifest.write_text(json.dumps({"schema_version": 1, "proofs": [record]}))
            self.assertEqual(gate.validate(root)[0], [])
            record["source_anchors"][0]["provider"] = "tcl8.5"
            manifest.write_text(json.dumps({"schema_version": 1, "proofs": [record]}))
            self.assertTrue(
                any(
                    "lacks its own pinned excerpt" in error
                    for error in gate.validate(root)[0]
                )
            )


    def test_failed_native_setup_cannot_answer_the_intended_question(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, census = self.native_attempt_fixture(root)
            # Host exit zero closes the driver, while its reached guest row is a setup error.
            self.assertEqual(self.write_record_fixture(root, record, census), [])
            for status in ["observed", "unsupported", "inspected"]:
                with self.subTest(status=status):
                    record["providers"][0]["status"] = status
                    self.assertTrue(any(
                        "cannot answer the intended provider question" in error
                        for error in self.write_record_fixture(root, record, census)
                    ))
            record["providers"][0]["status"] = "not-tested"
            record["kind"] = "native-observation"
            self.assertTrue(any(
                "attempt fields require their own record kind" in error
                for error in self.write_record_fixture(root, record, census)
            ))
            record["kind"] = "native-attempt"
            record["attempt"]["captures"][0].update(guest_code=0, completion_row="ORIGINAL|0|")
            self.assertTrue(self.write_record_fixture(root, record, census))

    def test_unreached_native_question_requires_boolean_false_not_integer_zero(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, census = self.native_attempt_fixture(root)
            self.assertEqual(self.write_record_fixture(root, record, census), [])
            for reached in [0, True]:
                with self.subTest(reached=reached):
                    record["attempt"]["intended_question_reached"] = reached
                    self.assertTrue(self.write_record_fixture(root, record, census))
            record["attempt"]["intended_question_reached"] = False
            self.assertEqual(self.write_record_fixture(root, record, census), [])

    def test_measured_environment_cannot_issue_a_native_provider_answer(self):
        # Implementation contract: naming.proof-records.exact-evidence-and-coverage
        # docs/design/analysis/name-resolution-proofs/exact-evidence-and-coverage.md
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record, path, census = self.fixture(root)
            record["kind"] = "environment-observation"
            for provider in record["providers"]:
                provider.update(status="not-tested", evidence=[])
            receipt = path.with_name("environment-receipt.json")
            receipt.write_text('{"exit":0,"target":"synthetic-environment"}\n')
            record["evidence"].append({
                "id": "environment-receipt",
                "role": "provider",
                "path": str(receipt.relative_to(root)),
                "sha256": gate.digest(receipt),
                "description": "Synthetic environment receipt; not a Tcl or Jim provider result.",
            })
            record["environments"] = [{
                "id": "synthetic-environment",
                "version": "synthetic",
                "build": "Synthetic validator fixture only.",
                "target": "synthetic target",
                "input_channel": "Synthetic retained row.",
                "status": "observed",
                "answer": "Only this synthetic environment's captured result is described.",
                "evidence": ["row", "environment-receipt"],
            }]
            for outcome in ["observed", "failed"]:
                record["environments"][0]["status"] = outcome
                self.assertEqual(self.write_record_fixture(root, record, census), [])
                for status in ["observed", "unsupported", "inspected"]:
                    with self.subTest(outcome=outcome, native_status=status):
                        record["providers"][0].update(status=status, evidence=["row", "environment-receipt"])
                        self.assertTrue(any(
                            "environment result cannot issue a measured native-provider answer" in error
                            for error in self.write_record_fixture(root, record, census)
                        ))
                record["providers"][0].update(status="not-tested", evidence=[])
            record["kind"] = "native-observation"
            self.assertTrue(any(
                "environment answers require their own record kind" in error
                for error in self.write_record_fixture(root, record, census)
            ))

if __name__ == "__main__":
    unittest.main()
