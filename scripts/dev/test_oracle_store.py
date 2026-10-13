#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fixed controls for lossless storage, safe paths and F5 address materialisation."""

import gzip
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from zipfile import ZipFile
from types import SimpleNamespace

from oracle_store import STORE, body, retained_file

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "materialise", ROOT / "experiments/oracles/f5/materialise.py"
)
materialise = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(materialise)
PACK_SPEC = importlib.util.spec_from_file_location(
    "pack_oracle", ROOT / "scripts/dev/pack-oracle-evidence.py"
)
pack_oracle = importlib.util.module_from_spec(PACK_SPEC)
PACK_SPEC.loader.exec_module(pack_oracle)


class OracleStoreTests(unittest.TestCase):
    def fixture(self, directory, payload=b"NHO|case\x00\xff\r\n"):
        root = Path(directory)
        store = root / STORE
        store.mkdir(parents=True)
        sha = hashlib.sha256(payload).hexdigest()
        with ZipFile(store / "artefacts.zip", "w") as archive:
            archive.writestr(sha, payload)
        names = ["old/run/stdout", "old/run/alias"]
        rows = {
            name: {"sha256": sha, "bytes": len(payload), "mode": 0o644}
            for name in names
        }
        (store / "catalogue.json.gz").write_bytes(
            gzip.compress(json.dumps({"files": rows}).encode())
        )
        return root, names, sha

    def test_binary_and_alias_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root, names, _ = self.fixture(directory)
            self.assertEqual(
                retained_file(root, names[0]).read_bytes(), b"NHO|case\x00\xff\r\n"
            )
            self.assertEqual(
                retained_file(root, names[1]).read_bytes(), b"NHO|case\x00\xff\r\n"
            )

    def test_changed_member_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root, names, sha = self.fixture(directory)
            with ZipFile(root / STORE / "artefacts.zip", "w") as archive:
                archive.writestr(sha, b"changed")
            body.cache_clear()
            with self.assertRaisesRegex(ValueError, "digest changed"):
                retained_file(root, names[0]).read_bytes()

    def test_traversal_and_absolute_paths_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root, _, _ = self.fixture(directory)
            for name in ["../old/run/stdout", "/old/run/stdout", ""]:
                with self.subTest(name=name), self.assertRaises(ValueError):
                    retained_file(root, name)

    def test_symlink_escape_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root, _, _ = self.fixture(directory)
            (root / "escape").symlink_to("/etc/hosts")
            with self.assertRaisesRegex(ValueError, "escapes repository"):
                retained_file(root, "escape")

    def test_subnet_and_individual_host_mapping(self):
        addresses = {"192.168.1.11", "192.168.1.33", "192.168.1.100"}
        result = materialise.mapping(
            addresses, materialise.network("192.168.9.0/24"), [".33=.75"]
        )
        self.assertEqual(
            result,
            {
                "192.168.1.11": "192.168.9.11",
                "192.168.1.33": "192.168.9.75",
                "192.168.1.100": "192.168.9.100",
            },
        )

    def test_collision_and_absent_source_are_rejected(self):
        addresses = {"192.168.1.33", "192.168.1.75"}
        for overrides in [[".33=.75"], [".44=.76"], [".33=.76", ".33=.77"]]:
            with self.subTest(overrides=overrides), self.assertRaises(ValueError):
                materialise.mapping(
                    addresses, materialise.network("192.168.9.0/24"), overrides
                )

    def test_invalid_network_and_reserved_targets_are_rejected(self):
        for value in ["192.168.9.1/24", "192.168.9.0/25", "2001:db8::/24"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                materialise.network(value)
        for target in [".0", ".255", "192.168.10.75"]:
            with self.subTest(target=target), self.assertRaises(ValueError):
                materialise.mapping(
                    {"192.168.1.33"},
                    materialise.network("192.168.9.0/24"),
                    [".33=" + target],
                )

    def test_atomic_exchange_keeps_ports_and_route_domains(self):
        source = b"ltm virtual /Common/a { destination 192.168.1.33%7:443 }\nltm virtual /Common/b { destination 192.168.1.75%7:8443 }\n"
        result = materialise.remap(
            source, {"192.168.1.33": "192.168.1.75", "192.168.1.75": "192.168.1.33"}
        )
        self.assertEqual(
            result,
            b"ltm virtual /Common/a { destination 192.168.1.75%7:443 }\nltm virtual /Common/b { destination 192.168.1.33%7:8443 }\n",
        )

    def test_pool_member_identity_and_address_agree(self):
        source = b"ltm pool /Common/p { members { 192.168.1.30:8080 { address 192.168.1.30 } } }\n"
        result = materialise.remap(source, {"192.168.1.30": "192.168.9.75"})
        self.assertEqual(
            result,
            b"ltm pool /Common/p { members { 192.168.9.75:8080 { address 192.168.9.75 } } }\n",
        )

    def test_comment_trigger_preserves_body_and_changes_source(self):
        original = b"when RULE_INIT { set static::v 1 }\n"
        result = materialise.trigger_source(original, "42")
        self.assertEqual(result, original + b"# oracle-trigger 42\n")
        self.assertNotEqual(
            hashlib.sha256(result).digest(), hashlib.sha256(original).digest()
        )
        self.assertEqual(original, b"when RULE_INIT { set static::v 1 }\n")

    def test_comment_nonce_cannot_inject_tcl(self):
        for nonce in ["", "42\nset x 1", "[exec id]", "-1"]:
            with self.subTest(nonce=nonce), self.assertRaises(ValueError):
                materialise.trigger_source(b"when RULE_INIT {}", nonce)

    def test_trigger_plan_changes_the_irule_body_not_the_scf_comment(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "input.scf"
            original = b"ltm rule /Common/r {\nwhen RULE_INIT { set static::v 1 }\n}\n"
            source.write_bytes(original)
            output = Path(directory) / "trigger"
            materialise.trigger_plan(
                SimpleNamespace(
                    input=source,
                    rule="/Common/r",
                    nonce="42",
                    output=output,
                    trigger_kind="comment",
                )
            )
            body = materialise.f5(
                [
                    "query",
                    '.ltm.rule["/Common/r"].body',
                    str(output / "trigger.scf"),
                    "--raw",
                ]
            )
            self.assertIn(b"when RULE_INIT { set static::v 1 }", body)
            self.assertTrue(body.rstrip().endswith(b"# oracle-trigger 42"))
            self.assertEqual(source.read_bytes(), original)
            self.assertFalse(
                json.loads((output / "trigger.json").read_text())["state_reset_proven"]
            )

    def test_repacking_replaces_aliases_and_discards_unreferenced_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root, names, old_sha = self.fixture(directory)
            for name in names:
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(b"reviewed replacement")
            output = root / "new-store"
            result = pack_oracle.pack(root, ["old"], output, root / STORE)
            self.assertEqual(result["logical_files"], 2)
            self.assertEqual(result["distinct_bodies"], 1)
            with ZipFile(output / "artefacts.zip") as archive:
                self.assertNotIn(old_sha, archive.namelist())
                self.assertEqual(
                    archive.read(archive.namelist()[0]), b"reviewed replacement"
                )

    def test_packing_rejects_symlinks_and_existing_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "captures").mkdir()
            (root / "captures/escape").symlink_to("/etc/hosts")
            with self.assertRaises(ValueError):
                pack_oracle.pack(root, ["captures"], root / "out")
            with self.assertRaises(ValueError):
                pack_oracle.pack(root, ["captures"], root / "captures")


if __name__ == "__main__":
    unittest.main()
