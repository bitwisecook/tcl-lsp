#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Authenticate two maintained replay-source snapshots and twenty exact bindings.

Original source stays in memory. Comparing receipt dictionary construction does
not run a replayer, compiler or provider, or renew any captured execution result.
"""

import argparse
import ast
import copy
import hashlib
import importlib.util
import io
import json
import platform
import subprocess
import sys
import tokenize
from pathlib import Path

PROOF_DIRECTORY = "docs/design/analysis/name-resolution-proofs"
HELPER = "scripts/dev/check-authored-fixture-source-review.py"
HELPER_SHA256 = "4f553f7a5ec3d731bf9cc1db7a64c797c4a4f570d8ed5929673bc31b5484df4f"
COMMIT = "9d4f72634fa1f27fe35135a0b62770e339e10dae"
SOURCE_PINS = {
    "scripts/dev/replay-array-oo-source-controls.py": {
        "original_blob": "40512b4e532b7fda5145f4561529096259118c95",
        "original_sha256": "637183f5183dcaccff98ea8e379e5cb8019426d71525dc5c5bbb5d53a4ed2570",
        "current_sha256": "690d6dbfc13ceb51a0d3d630e33d8ec2f4f08d8b2250c06dc0185925f3d1828d",
        "original_bytes": 8039,
        "current_bytes": 8068,
        "normalised_complete_ast_sha256": "356a6a8d364d67756b51786e99f413b49a238a87d5d91ff61a60eadf4c612f12",
        "dict_calls": 1,
    },
    "scripts/dev/replay-grammar-native-source.py": {
        "original_blob": "d5ef01e1110fa2e0de43e4ad080d63bec4634cf4",
        "original_sha256": "f958561e2f2f46e643c79d74ae8c4c2d6872f6525434f20d720a7e6b3e07fe3c",
        "current_sha256": "a19ace0e42037b34b6493703885b22a55167e88c4ef91c8ac097a33e45fa88e6",
        "original_bytes": 10631,
        "current_bytes": 10667,
        "normalised_complete_ast_sha256": "1e7dcf3d73dbab798f9ee7b14223c34d2c1ee2273e95cc7373d9f2ebd509f21c",
        "dict_calls": 3,
    },
}
BINDINGS = {
    ("naming.array-source.scalar-element-storage", "replay"),
    ("naming.nested-expression-syntax-failure-order", "maintained-source-replayer"),
    ("naming.array-source.combined-unset", "replay"),
    ("naming.tcloo.source-generated-zero-option-classification", "replay"),
    ("naming.mathop-binding-and-written-head-controls", "maintained-source-replayer"),
    ("naming.entered-child-source-completion", "maintained-source-replayer"),
    ("naming.array-source.combined-read-enumeration", "replay"),
    ("naming.quoted-tmm-operator-value", "maintained-source-replayer"),
    ("naming.tcl84-authored-math-function-surface", "maintained-source-replayer"),
    ("naming.array-source.combined-set-receiver", "replay"),
    ("naming.tcloo.source-superclass-slot-forms", "replay"),
    ("naming.tcl84-random-and-precision-controls", "maintained-source-replayer"),
    ("naming.tcloo.source-mixin-slot-forms", "replay"),
    ("naming.concat-malformed-list-result", "maintained-source-replayer"),
    ("naming.expression-abs-handler-replacement", "maintained-source-replayer"),
    ("naming.tcl84-function-child-syntax-order", "maintained-source-replayer"),
    ("naming.return-sole-result-option-spellings", "maintained-source-replayer"),
    ("naming.mathop-procedure-source-controls", "maintained-source-replayer"),
    ("naming.return-option-pairs-release-shape", "maintained-source-replayer"),
    ("naming.expression-result-normalisation-effects", "maintained-source-replayer"),
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def claim_pin(record, evidence_id, original_sha256):
    reviewed = copy.deepcopy(record)
    matching = [item for item in reviewed["evidence"] if item["id"] == evidence_id]
    if len(matching) != 1:
        raise ValueError("reviewed source evidence ID is absent or ambiguous")
    matching[0]["sha256"] = original_sha256
    reviewed["evidence"] = matching
    return sha(json.dumps(reviewed, sort_keys=True, ensure_ascii=False).encode())


def require_builtin_dict(tree):
    """Decline local rebinding and dynamic access to source/builtin namespaces."""
    dynamic = {"exec", "eval", "globals", "locals", "vars", "__import__"}
    namespace_attributes = {"f_globals", "f_builtins", "__globals__", "__builtins__"}
    for node in ast.walk(tree):
        shadowed = (
            isinstance(node, ast.Name)
            and node.id == "dict"
            and isinstance(node.ctx, (ast.Store, ast.Del))
            or isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))
            and node.name == "dict"
            or isinstance(node, ast.arg)
            and node.arg == "dict"
            or isinstance(node, (ast.Global, ast.Nonlocal))
            and "dict" in node.names
            or isinstance(node, ast.ExceptHandler)
            and node.name == "dict"
            or isinstance(node, (ast.MatchAs, ast.MatchStar))
            and node.name == "dict"
            or isinstance(node, ast.MatchMapping)
            and node.rest == "dict"
        )
        if shadowed:
            raise ValueError("builtin dict is shadowed or rebound")
        if isinstance(node, (ast.Import, ast.ImportFrom)) and (
            isinstance(node, ast.ImportFrom)
            and node.module == "builtins"
            or any(
                (item.asname or item.name.split(".")[0]) in {"dict", "builtins"}
                or item.name == "*"
                for item in node.names
            )
        ):
            raise ValueError("import changes the builtin dict premise")
        if (
            isinstance(node, ast.Name)
            and node.id in {"builtins", "__builtins__"}
            or isinstance(node, ast.Attribute)
            and node.attr in namespace_attributes
            or isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id in dynamic
        ):
            raise ValueError("dynamic source or builtin namespace access")


class DictionaryNormalisation(ast.NodeTransformer):
    """Only unique keyword strings, with every value kept in original order."""

    def __init__(self):
        self.changes = []

    def visit_Call(self, node):
        node = self.generic_visit(node)
        if not isinstance(node.func, ast.Name) or node.func.id != "dict":
            return node
        keys = [item.arg for item in node.keywords]
        if node.args or not keys or None in keys or len(set(keys)) != len(keys):
            raise ValueError("dict call exceeds the reviewed unique-key keyword form")
        self.changes.append(
            {
                "keys": keys,
                "ordered_value_asts_sha256": sha(
                    json.dumps(
                        [
                            ast.dump(item.value, include_attributes=False)
                            for item in node.keywords
                        ]
                    ).encode()
                ),
            }
        )
        return ast.copy_location(
            ast.Dict(
                keys=[ast.Constant(key) for key in keys],
                values=[item.value for item in node.keywords],
            ),
            node,
        )


def comments(raw):
    return [
        token.string
        for token in tokenize.tokenize(io.BytesIO(raw).readline)
        if token.type == tokenize.COMMENT
    ]


def compare_structure(old, new):
    """Compare complete source ASTs; no node is evaluated or compiled."""
    if sys.version_info[:2] != (3, 12):
        raise ValueError("reviewed AST fingerprint protocol requires Python 3.12")
    before, after = (
        ast.parse(old, type_comments=True),
        ast.parse(new, type_comments=True),
    )
    require_builtin_dict(before)
    require_builtin_dict(after)
    if comments(old) != comments(new):
        raise ValueError("reviewed source comments changed")
    prior, current = DictionaryNormalisation(), DictionaryNormalisation()
    a = ast.dump(prior.visit(before), include_attributes=False)
    b = ast.dump(current.visit(after), include_attributes=False)
    if a != b:
        raise ValueError("source AST delta exceeds ordered unique-key dictionaries")
    return {
        "normalised_complete_ast_sha256": sha(a.encode()),
        "original_keyword_dict_calls": prior.changes,
        "current_keyword_dict_calls": current.changes,
        "comments_identical": True,
    }


def compare(old, new, pins):
    if (
        sha(old) != pins["original_sha256"]
        or len(old) != pins["original_bytes"]
        or sha(new) != pins["current_sha256"]
        or len(new) != pins["current_bytes"]
    ):
        raise ValueError("reviewed complete source snapshot changed")
    comparison = compare_structure(old, new)
    if (
        len(comparison["original_keyword_dict_calls"]) != pins["dict_calls"]
        or comparison["current_keyword_dict_calls"]
        or comparison["normalised_complete_ast_sha256"]
        != pins["normalised_complete_ast_sha256"]
    ):
        raise ValueError("exact dictionary roster or complete AST fingerprint differs")
    return comparison


def validate(root, contract_path, fetch=False):
    root = root.resolve()
    contract_raw = contract_path.read_bytes()
    contract = json.loads(contract_raw)
    if (
        contract["schema_version"] != 1
        or contract["kind"] != "maintained-replay-ordered-dictionary-source-snapshots"
        or contract["original_commit"] != COMMIT
        or contract["original_reader_sha256"] != HELPER_SHA256
        or contract["ast_protocol"] != "Python 3.12, ast.dump without attributes"
        or len(contract["files"]) != 2
        or {row["path"]: row["pins"] for row in contract["files"]} != SOURCE_PINS
    ):
        raise ValueError("reviewed source roster or identity contract differs")
    reviews = {
        (row["proof_id"], row["evidence_id"]): row for row in contract["bindings"]
    }
    if len(contract["bindings"]) != 20 or set(reviews) != BINDINGS:
        raise ValueError("reviewed twenty-binding roster differs")
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
    source_bytes = {}
    results = []
    for path, pins in SOURCE_PINS.items():
        old, availability = helper.original(
            root,
            {
                "path": path,
                "original_commit": COMMIT,
                "original_blob": pins["original_blob"],
            },
            fetch,
        )
        if (
            hashlib.sha1(b"blob " + str(len(old)).encode() + b"\0" + old).hexdigest()
            != pins["original_blob"]
        ):
            raise ValueError("original Git blob identity differs")
        source_bytes[path] = (root / path).read_bytes()
        comparison = compare(old, source_bytes[path], pins)
        results.append(
            {
                "path": path,
                "original_commit": COMMIT,
                "original_blob": pins["original_blob"],
                "original_sha256": sha(old),
                "current_sha256": sha(source_bytes[path]),
                "original_availability": availability,
                **comparison,
            }
        )
    observed = set()
    for record in manifest["proofs"]:
        for item in record["evidence"]:
            pins = SOURCE_PINS.get(item["path"])
            if pins is None:
                continue
            pair = (record["id"], item["id"])
            if (
                pair not in reviews
                or pair in observed
                or reviews[pair]["path"] != item["path"]
                or item["role"] != "implementation"
                or item["sha256"]
                not in {pins["original_sha256"], pins["current_sha256"]}
                or item.keys() & {"contains", "lines", "json_pointer"}
                or claim_pin(record, item["id"], pins["original_sha256"])
                != reviews[pair]["claim_sha256"]
            ):
                raise ValueError("exact reviewed source-only proof binding differs")
            observed.add(pair)
    if observed != BINDINGS:
        raise ValueError("reviewed source binding is absent")
    if (
        any((root / path).read_bytes() != raw for path, raw in source_bytes.items())
        or helper_path.read_bytes() != helper_raw
        or contract_path.read_bytes() != contract_raw
        or manifest_path.read_bytes() != manifest_raw
        or census_path.read_bytes() != census_raw
    ):
        raise ValueError("source review input changed during inspection")
    return {
        "schema_version": 1,
        "scope": "Authenticated maintained source snapshots for twenty exact implementation-role replay bindings. Only ordered unique-key builtin keyword dict construction changes to dictionary literals; complete remaining AST, literals, docstrings and comments match. No replayer/compiler/provider execution, capture renewal, provider answer or global gate acceptance.",
        "limits": "Ordinary unmodified Python builtin dict semantics are assumed. Source locations, profiling/introspection, altered builtins and allocation-failure equivalence are not claimed. Source metadata does not renew any historical executable or result association.",
        "verifier_sha256": sha(Path(__file__).read_bytes()),
        "contract_sha256": sha(contract_raw),
        "original_reader_sha256": sha(helper_raw),
        "python": {
            "version": platform.python_version(),
            "executable_sha256": sha(Path(sys.executable).resolve().read_bytes()),
        },
        "input_manifest_sha256": sha(manifest_raw),
        "input_census_sha256": sha(census_raw),
        "files": results,
        "bindings": contract["bindings"],
        "replayer_compiler_or_provider_execution": "not run",
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
        tokenize.TokenError,
        subprocess.TimeoutExpired,
    ):
        print(
            "Maintained replay source review failed; required originals, pins or exact reviewed dictionary transformation are unavailable.",
            file=sys.stderr,
        )
        return 1
    print(
        json.dumps(
            {
                "source_files": 2,
                "implementation_bindings": 20,
                "replayer_compiler_or_provider_execution": "not run",
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
