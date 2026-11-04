#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Authenticate three exact maintained proof-tool source snapshots.

This is a bounded source comparison, not a complete proof-gate run, test result,
native execution renewal, or general rendering-equivalence claim.
"""

import argparse
import ast
import copy
import hashlib
import importlib.machinery
import importlib.util
import json
import platform
import subprocess
import sys
from pathlib import Path

PROOF_DIRECTORY = "docs/design/analysis/name-resolution-proofs"
PROOF = "naming.proof-records.exact-evidence-and-coverage"
HELPER = "scripts/dev/check-authored-fixture-source-review.py"
HELPER_SHA256 = "4f553f7a5ec3d731bf9cc1db7a64c797c4a4f570d8ed5929673bc31b5484df4f"
COMMIT = "70f054226b9d0ea4ba488302b1b6150f2c6e6d61"
SOURCE_PINS = {
    "scripts/dev/check-name-resolution-proofs.py": {
        "original_blob": "db20648852f20dab49196ac0b536bdfe85fd3648",
        "original_sha256": "ecb7350b720b90da729816241fa5b68b2ee26d214e5f05de3f45220bbb03c808",
        "current_sha256": "52249cb5c5704cd76e685a01ae47eacef2e5bbfdb66e58f90d06331aac569910",
        "original_bytes": 37792,
        "current_bytes": 39376,
        "relation": "checker",
        "evidence_id": "check-name-resolution-proofs",
    },
    "scripts/dev/test_name_resolution_proofs.py": {
        "original_blob": "02e534a84f873d27a1784eb528fe9cd02e9d47fe",
        "original_sha256": "695880f9d547fa3ae1fbef7567e1c66c05cade1e51de411824014286f6c129cd",
        "current_sha256": "0036250cc8995fcbef45c1a6ff014468fbed929019c21cbacc0d2e59c6b8149b",
        "original_bytes": 36242,
        "current_bytes": 37682,
        "relation": "controls",
        "evidence_id": "test_name_resolution_proofs",
    },
    "scripts/dev/update-name-resolution-proof-index.py": {
        "original_blob": "fe5290a985068f4b4d25fbd359f4f13430d8251c",
        "original_sha256": "51b2dc7e96729fffa3a25c8dfd38254d097075836c836c0d93c9863ec1f54f33",
        "current_sha256": "e3a6728b81388b29b9f7403b97288e5dd8e251fabded039d7b77f81e4f516ab2",
        "original_bytes": 10029,
        "current_bytes": 10038,
        "relation": "merger",
        "evidence_id": "reviewed-classification-merger",
    },
}
GLOBAL_GUARD = """if SPEC is None or SPEC.loader is None:
    raise ImportError(f"cannot load {SCRIPT}")
"""
LOCAL_GUARD = """if spec is None or spec.loader is None:
    self.fail("cannot load update-name-resolution-proof-index.py")
"""


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def tree_pin(node):
    return ast.dump(node, include_attributes=False)


def claim_pin(record, evidence_id, original_sha256):
    reviewed = copy.deepcopy(record)
    matching = [item for item in reviewed["evidence"] if item["id"] == evidence_id]
    if len(matching) != 1:
        raise ValueError("reviewed source evidence ID is absent or ambiguous")
    matching[0]["sha256"] = original_sha256
    reviewed["evidence"] = matching
    return sha(json.dumps(reviewed, sort_keys=True, ensure_ascii=False).encode())


def require_attached_premise(tree):
    parents = {
        id(child): parent
        for parent in ast.walk(tree)
        for child in ast.iter_child_nodes(parent)
    }
    validate = next(
        node
        for node in tree.body
        if isinstance(node, ast.FunctionDef) and node.name == "validate"
    )
    helpers = [
        node
        for node in ast.walk(validate)
        if isinstance(node, ast.FunctionDef) and node.name == "attached"
    ]
    if len(helpers) != 1:
        raise ValueError("reviewed attached helper is absent or ambiguous")
    helper = helpers[0]
    loop = parents[id(helper)]
    if (
        not isinstance(loop, ast.For)
        or tree_pin(loop.target) != tree_pin(ast.Name(id="anchor", ctx=ast.Store()))
        or tree_pin(loop.iter)
        != tree_pin(ast.parse('record["source_anchors"]', mode="eval").body)
    ):
        raise ValueError("attached helper is outside its original anchor loop")
    position = loop.body.index(helper)
    if position + 1 >= len(loop.body) or not isinstance(
        loop.body[position + 1], ast.If
    ):
        raise ValueError("attached helper is not immediately consumed")
    if tree_pin(loop.body[position + 1].test) != tree_pin(
        ast.parse("not attached(value)", mode="eval").body
    ):
        raise ValueError("attached helper immediate call differs")
    references = [
        node
        for node in ast.walk(validate)
        if isinstance(node, ast.Name) and node.id == "attached"
    ]
    if len(references) != 3:
        raise ValueError("attached helper has another reference or escapes")
    for reference in references:
        call = parents[id(reference)]
        if (
            not isinstance(call, ast.Call)
            or call.func is not reference
            or len(call.args) != 1
            or call.keywords
        ):
            raise ValueError(
                "attached helper reference is not its original one-argument call"
            )
    if any(
        isinstance(node, (ast.Yield, ast.YieldFrom, ast.Await, ast.AsyncFunctionDef))
        for node in ast.walk(helper)
    ):
        raise ValueError("attached helper can suspend across anchor iterations")
    future = any(
        isinstance(node, ast.ImportFrom)
        and node.module == "__future__"
        and any(item.name == "annotations" for item in node.names)
        for node in tree.body
    )
    if not future:
        raise ValueError("reviewed annotation evaluation premise differs")


class SourceNormalisation(ast.NodeTransformer):
    def __init__(self, kind):
        self.kind = kind
        self.scope = []
        self.changes = {
            "typing_import": 0,
            "identity_cast": 0,
            "anchor_default": 0,
            "global_loader_guard": 0,
            "local_loader_guard": 0,
            "render_trim": 0,
        }

    def visit_ClassDef(self, node):
        self.scope.append(node.name)
        node = self.generic_visit(node)
        self.scope.pop()
        return node

    def visit_FunctionDef(self, node):
        self.scope.append(node.name)
        if (
            self.kind == "checker"
            and self.scope == ["validate", "attached"]
            and len(node.args.args) == 2
        ):
            extra = node.args.args[-1]
            if (
                extra.arg != "anchor"
                or tree_pin(extra.annotation)
                != tree_pin(ast.Name(id="dict", ctx=ast.Load()))
                or len(node.args.defaults) != 1
                or tree_pin(node.args.defaults[0])
                != tree_pin(ast.Name(id="anchor", ctx=ast.Load()))
            ):
                raise ValueError("attached default does not capture the current anchor")
            node.args.args.pop()
            node.args.defaults.clear()
            self.changes["anchor_default"] += 1
        node = self.generic_visit(node)
        self.scope.pop()
        return node

    def visit_ImportFrom(self, node):
        if (
            self.kind == "checker"
            and node.module == "typing"
            and len(node.names) == 1
            and node.names[0].name == "cast"
            and node.names[0].asname is None
        ):
            self.changes["typing_import"] += 1
            return None
        return node

    def visit_If(self, node):
        if self.kind == "controls":
            if not self.scope and tree_pin(node) == tree_pin(
                ast.parse(GLOBAL_GUARD).body[0]
            ):
                self.changes["global_loader_guard"] += 1
                return None
            if self.scope == [
                "ProofGateTests",
                "test_classification_merge_cannot_accept_changed_input_or_unknown_question",
            ] and tree_pin(node) == tree_pin(ast.parse(LOCAL_GUARD).body[0]):
                self.changes["local_loader_guard"] += 1
                return None
        return self.generic_visit(node)

    def visit_Call(self, node):
        node = self.generic_visit(node)
        if (
            self.kind == "checker"
            and isinstance(node.func, ast.Name)
            and node.func.id == "cast"
        ):
            if (
                self.scope != ["schema_errors"]
                or len(node.args) != 2
                or node.keywords
                or tree_pin(node.args[0])
                != tree_pin(ast.Name(id="dict", ctx=ast.Load()))
                or not isinstance(node.args[1], ast.Call)
                or not isinstance(node.args[1].func, ast.Name)
                or node.args[1].func.id != "pointer"
            ):
                raise ValueError("typing cast exceeds the reviewed identity call")
            self.changes["identity_cast"] += 1
            return node.args[1]
        if (
            self.kind == "merger"
            and self.scope == ["render"]
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "rstrip"
            and tree_pin(node.func.value)
            == tree_pin(ast.parse("record[field]", mode="eval").body)
        ):
            if node.args or node.keywords:
                raise ValueError("render trim gained arguments or keyword effects")
            self.changes["render_trim"] += 1
            return node.func.value
        return node


def require_cast_premise(tree):
    for node in ast.walk(tree):
        if isinstance(node, ast.Import) and any(
            item.name in {"builtins", "typing"}
            or (item.asname or item.name.split(".")[0]) in {"cast", "dict"}
            for item in node.names
        ):
            raise ValueError("reviewed builtin/cast namespace can be replaced")
        if isinstance(node, ast.ImportFrom) and any(
            item.name == "*" for item in node.names
        ):
            raise ValueError("wildcard import can replace reviewed bindings")
        if isinstance(node, ast.ExceptHandler) and node.name in {"cast", "dict"}:
            raise ValueError("exception binding shadows reviewed builtin/cast")
        if isinstance(node, ast.Name) and node.id in {
            "globals",
            "locals",
            "vars",
            "eval",
            "exec",
            "__import__",
            "__builtins__",
        }:
            raise ValueError("dynamic namespace effects exceed reviewed premise")
        if (
            isinstance(node, ast.Name)
            and node.id in {"cast", "dict"}
            and isinstance(node.ctx, (ast.Store, ast.Del))
        ):
            raise ValueError("reviewed builtin/cast binding is reassigned")
        if isinstance(node, ast.arg) and node.arg in {"cast", "dict"}:
            raise ValueError("reviewed builtin/cast binding is shadowed")
        if isinstance(node, (ast.FunctionDef, ast.ClassDef)) and node.name in {
            "cast",
            "dict",
        }:
            raise ValueError("reviewed builtin/cast binding is redefined")
        if (
            isinstance(node, ast.ImportFrom)
            and any(
                (item.asname or item.name) in {"cast", "dict"} for item in node.names
            )
            and not (
                node.module == "typing"
                and len(node.names) == 1
                and node.names[0].name == "cast"
                and node.names[0].asname is None
            )
        ):
            raise ValueError("cast does not come from the reviewed typing import")


def require_merger_premise(tree):
    merger = next(
        node
        for node in tree.body
        if isinstance(node, ast.FunctionDef) and node.name == "merge_classifications"
    )
    if any(
        isinstance(node, ast.Name) and node.id == "render" for node in ast.walk(merger)
    ):
        raise ValueError("classification merge depends on rendering")
    main = next(
        node
        for node in tree.body
        if isinstance(node, ast.FunctionDef) and node.name == "main"
    )
    branches = [
        node
        for node in main.body
        if isinstance(node, ast.If)
        and tree_pin(node.test)
        == tree_pin(ast.parse("args.classifications", mode="eval").body)
    ]
    if (
        len(branches) != 1
        or not isinstance(branches[0].body[-1], ast.Return)
        or any(
            isinstance(node, ast.Name) and node.id == "render"
            for node in ast.walk(branches[0])
        )
    ):
        raise ValueError(
            "classification branch does not return independently of rendering"
        )


def compare_structure(kind, old, new):
    if sys.version_info[:2] != (3, 12):
        raise ValueError("reviewed AST protocol requires Python 3.12")
    before, after = (
        ast.parse(old, type_comments=True),
        ast.parse(new, type_comments=True),
    )
    for tree in (before, after):
        if kind == "checker":
            require_attached_premise(tree)
            require_cast_premise(tree)
        if kind == "merger":
            require_merger_premise(tree)
    prior, current = SourceNormalisation(kind), SourceNormalisation(kind)
    a, b = (
        tree_pin(prior.visit(copy.deepcopy(before))),
        tree_pin(current.visit(copy.deepcopy(after))),
    )
    if a != b:
        raise ValueError(
            "complete source AST delta exceeds the reviewed scoped transformations"
        )
    expected = {
        "checker": {"typing_import": 1, "identity_cast": 1, "anchor_default": 1},
        "controls": {"global_loader_guard": 1, "local_loader_guard": 1},
        "merger": {"render_trim": 1},
    }[kind]
    if (
        any(prior.changes.values())
        or {key: value for key, value in current.changes.items() if value} != expected
    ):
        raise ValueError("reviewed exact transformation roster differs")
    return {
        "normalised_complete_ast_sha256": sha(a.encode()),
        "changes": expected,
        "all_other_ast_nodes_identical": True,
    }


def compare_snapshot(old, new, pins):
    if (
        sha(old) != pins["original_sha256"]
        or len(old) != pins["original_bytes"]
        or sha(new) != pins["current_sha256"]
        or len(new) != pins["current_bytes"]
        or hashlib.sha1(b"blob " + str(len(old)).encode() + b"\0" + old).hexdigest()
        != pins["original_blob"]
    ):
        raise ValueError("authenticated original/current source snapshot differs")
    return compare_structure(pins["relation"], old, new)


def validate(root, contract_path, fetch=False):
    root = root.resolve()
    contract_raw = contract_path.read_bytes()
    contract = json.loads(contract_raw)
    if (
        contract["schema_version"] != 1
        or contract["kind"] != "maintained-proof-tool-scoped-source-snapshots"
        or contract["original_commit"] != COMMIT
        or contract["original_reader_sha256"] != HELPER_SHA256
        or len(contract["files"]) != 3
        or {row["path"]: row["pins"] for row in contract["files"]} != SOURCE_PINS
    ):
        raise ValueError("reviewed source roster or identity contract differs")
    reviews = {row["evidence_id"]: row for row in contract["bindings"]}
    if (
        len(contract["bindings"]) != 3
        or set(reviews) != {pins["evidence_id"] for pins in SOURCE_PINS.values()}
        or any(row["proof_id"] != PROOF for row in reviews.values())
    ):
        raise ValueError("reviewed three-binding roster differs")
    helper_path = root / HELPER
    helper_raw = helper_path.read_bytes()
    if sha(helper_raw) != HELPER_SHA256:
        raise ValueError("authenticated original reader changed")
    spec = importlib.util.spec_from_file_location("authored_source_reader", helper_path)
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    manifest_path = root / PROOF_DIRECTORY / "manifest.json"
    census_path = root / PROOF_DIRECTORY / "corpus-inventory.json"
    manifest_raw, census_raw = manifest_path.read_bytes(), census_path.read_bytes()
    manifest = json.loads(manifest_raw)
    snapshots = {}
    results = []
    for path, pins in SOURCE_PINS.items():
        source = root / path
        loader_spec = importlib.util.spec_from_file_location(
            "reviewed_source_premise", source
        )
        if loader_spec is None or not isinstance(
            loader_spec.loader, importlib.machinery.SourceFileLoader
        ):
            raise ValueError("original source-file loader premise is unavailable")
        old, availability = helper.original(
            root,
            {
                "path": path,
                "original_commit": COMMIT,
                "original_blob": pins["original_blob"],
            },
            fetch,
        )
        new = source.read_bytes()
        snapshots[path] = new
        comparison = compare_snapshot(old, new, pins)
        results.append(
            {
                "path": path,
                "original_commit": COMMIT,
                "original_blob": pins["original_blob"],
                "original_sha256": sha(old),
                "current_sha256": sha(new),
                "original_availability": availability,
                "source_file_loader_verified_without_execution": True,
                **comparison,
            }
        )
    observed = set()
    for record in manifest["proofs"]:
        for item in record["evidence"]:
            pins = SOURCE_PINS.get(item["path"])
            if pins is None:
                continue
            key = item["id"]
            if (
                record["id"] != PROOF
                or key != pins["evidence_id"]
                or key not in reviews
                or key in observed
                or reviews[key]["path"] != item["path"]
                or item["role"] != "implementation"
                or item["sha256"]
                not in {pins["original_sha256"], pins["current_sha256"]}
                or item.keys() & {"contains", "lines", "json_pointer"}
                or claim_pin(record, key, pins["original_sha256"])
                != reviews[key]["claim_sha256"]
                or any(key in provider["evidence"] for provider in record["providers"])
            ):
                raise ValueError("exact reviewed source-only proof binding differs")
            observed.add(key)
    if observed != set(reviews):
        raise ValueError("reviewed source binding is absent")
    if (
        any((root / path).read_bytes() != raw for path, raw in snapshots.items())
        or helper_path.read_bytes() != helper_raw
        or contract_path.read_bytes() != contract_raw
        or manifest_path.read_bytes() != manifest_raw
        or census_path.read_bytes() != census_raw
    ):
        raise ValueError("source review input changed during inspection")
    return {
        "schema_version": 1,
        "scope": "Authenticated exact maintained source review for three implementation-role proof-tool bindings. Standard typing.cast identity and nonescaping immediate JSON-anchor closure are compared; control loader guards are outside the verified SourceFileLoader premise; classification code and its early-return CLI path are unchanged while rendering alone trims trailing whitespace. No general renderer equivalence, proof-gate run, actual test execution, capture renewal or provider result.",
        "limits": "Ordinary unmodified Python standard-library/builtin semantics and plain retained JSON values are assumed. Import metadata, introspection/profiling, unavailable loaders, arbitrary render output and allocation-failure equivalence are not claimed. Comments/trivia remain bounded by exact whole-source identities; complete AST/type-comment comparisons preserve every other executable node.",
        "verifier_sha256": sha(Path(__file__).read_bytes()),
        "contract_sha256": sha(contract_raw),
        "original_reader_sha256": sha(helper_raw),
        "python_version": platform.python_version(),
        "input_manifest_sha256": sha(manifest_raw),
        "input_census_sha256": sha(census_raw),
        "files": results,
        "bindings": contract["bindings"],
        "source_or_native_execution": "not run",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--fetch-origin", action="store_true")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    try:
        result = validate(args.root, args.contract, args.fetch_origin)
        if args.receipt:
            args.receipt.write_text(json.dumps(result, indent=2) + "\n")
    except (
        ValueError,
        OSError,
        KeyError,
        IndexError,
        TypeError,
        SyntaxError,
        StopIteration,
        subprocess.TimeoutExpired,
    ):
        print(
            "Proof-tool source review failed; required originals, pins or exact scoped source premises are unavailable.",
            file=sys.stderr,
        )
        return 1
    print(
        json.dumps(
            {
                "source_files": 3,
                "implementation_bindings": 3,
                "source_or_native_execution": "not run",
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
