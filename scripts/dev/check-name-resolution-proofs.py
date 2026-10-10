#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Validate per-question proof records and the complete retained corpus census.

This checks retained evidence, not native execution or Rust test results.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import re
import sys
from functools import lru_cache
from pathlib import Path

DIRECTORY = Path("docs/design/analysis/name-resolution-proofs")
PROVIDERS = {"tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim", "bigip"}
CLASSIFICATIONS = {
    "native-observation",
    "environment-observation",
    "authored-contract",
    "provider-scaffolding",
    "input-fixture",
    "duplicate-evidence",
    "unavailable",
    "pending",
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative_file(root: Path, name: str) -> Path:
    path = Path(name)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError(f"expected a repository-relative path: {name!r}")
    result = (root / path).resolve()
    if not result.is_relative_to(root.resolve()):
        raise ValueError(f"path escapes repository: {name!r}")
    if not result.is_file():
        raise ValueError(f"missing file: {name}")
    return result


def pointer(value: object, selector: str) -> object:
    if selector == "":
        return value
    if not selector.startswith("/"):
        raise ValueError(f"invalid RFC 6901 pointer: {selector!r}")
    for raw in selector[1:].split("/"):
        if re.search(r"~(?![01])", raw):
            raise ValueError(f"invalid RFC 6901 escape: {raw!r}")
        part = raw.replace("~1", "/").replace("~0", "~")
        if isinstance(value, list):
            if not re.fullmatch(r"0|[1-9][0-9]*", part):
                raise ValueError(f"invalid array index: {part!r}")
            value = value[int(part)]
        elif isinstance(value, dict):
            value = value[part]
        else:
            raise ValueError(f"pointer enters a scalar at {part!r}")
    return value


def schema_errors(value: object, schema: dict, document: dict, label: str) -> list[str]:
    """Validate the deliberately small, checked-in JSON Schema vocabulary."""
    if "$ref" in schema:
        return schema_errors(
            value, pointer(document, schema["$ref"][1:]), document, label
        )
    errors = []
    kinds = {"object": dict, "array": list, "string": str, "integer": int, "boolean": bool}
    kind = schema.get("type")
    if kind and (
        not isinstance(value, kinds[kind])
        or kind == "integer"
        and isinstance(value, bool)
    ):
        return [f"{label}: expected {kind}"]
    if "const" in schema and value != schema["const"]:
        errors.append(f"{label}: expected constant {schema['const']!r}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{label}: unsupported value {value!r}")
    if isinstance(value, str):
        if len(value.strip()) < schema.get("minLength", 0):
            errors.append(f"{label}: empty text")
        if "pattern" in schema and not re.search(schema["pattern"], value):
            errors.append(f"{label}: invalid spelling {value!r}")
    if isinstance(value, int) and value < schema.get("minimum", value):
        errors.append(f"{label}: below minimum")
    if isinstance(value, dict):
        required = set(schema.get("required", []))
        errors.extend(
            f"{label}: missing {name}" for name in sorted(required - value.keys())
        )
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            errors.extend(
                f"{label}: unknown field {name}"
                for name in sorted(value.keys() - properties.keys())
            )
        for name in value.keys() & properties.keys():
            errors.extend(
                schema_errors(
                    value[name], properties[name], document, f"{label}.{name}"
                )
            )
    if isinstance(value, list):
        if len(value) < schema.get("minItems", 0) or len(value) > schema.get(
            "maxItems", len(value)
        ):
            errors.append(f"{label}: wrong array length")
        for index, item in enumerate(value):
            prefix = schema.get("prefixItems", [])
            item_schema = (
                prefix[index] if index < len(prefix) else schema.get("items", {})
            )
            errors.extend(
                schema_errors(item, item_schema, document, f"{label}[{index}]")
            )
    return errors


def corpus_files(root: Path) -> list[Path]:
    directories = []
    for parent in [root / "rust", root / "runtime/rust"]:
        pattern = (
            "*/tests/data/*"
            if parent.name == "rust" and parent.parent == root
            else "tests/data/*"
        )
        directories.extend(
            p
            for p in parent.glob(pattern)
            if p.is_dir() and p.name.startswith(("native", "authored"))
        )
    appliance = root / "scripts/dev/bigip-probes/resolution-2286"
    if appliance.is_dir():
        directories.append(appliance)
    # Python's generated import cache is tooling output, never a retained input,
    # source anchor, capture or provider receipt. All other fixture files remain
    # visible, including failed runs and malformed evidence manifests.
    return sorted(
        {
            p
            for directory in directories
            for p in directory.rglob("*")
            if p.is_file() and not generated_cache(p)
        }
    )


def generated_cache(path: Path) -> bool:
    return "__pycache__" in path.parts and path.suffix == ".pyc"


def inventory(root: Path, previous: dict | None = None) -> dict:
    old = {row["path"]: row for row in (previous or {}).get("files", [])}
    exclusions = {
        row["path"]: row for row in (previous or {}).get("excluded_tooling_files", [])
    }
    rows = []
    for path in corpus_files(root):
        name = path.relative_to(root).as_posix()
        sha = digest(path)
        row = {
            "path": name,
            "sha256": sha,
            "bytes": path.stat().st_size,
            "candidate": path.suffix == ".json",
        }
        if row["candidate"]:
            row.update(
                classification="pending",
                proof_ids=[],
                reason="Requires inspection of the retained input, provider and observation fields.",
            )
            prior = old.get(name)
            if prior and prior.get("sha256") == sha:
                for field in ["classification", "proof_ids", "reason", "duplicate_of"]:
                    if field in prior:
                        row[field] = prior[field]
        rows.append(row)
    present = {row["path"] for row in rows}
    for name, prior in old.items():
        if name not in present:
            if generated_cache(Path(name)):
                exclusions[name] = dict(
                    prior,
                    reason="Generated Python import bytecode; neither an input nor a capture. Its exact prior inventory row is retained here for the scope audit.",
                )
                continue
            row = dict(prior)
            row.update(
                candidate=True,
                classification="unavailable",
                reason="Previously inventoried artifact is absent; its digest alone is not recovered evidence.",
            )
            rows.append(row)
    rows.sort(key=lambda row: row["path"])
    return {
        "schema_version": 1,
        "scope": "All exact native/authored fixture files and resolution-2286 appliance artifacts; JSON files are candidate records, not proof counts. Generated Python import bytecode is tooling output; previously inventoried cache rows remain in the explicit exclusions audit.",
        "excluded_tooling_files": sorted(
            exclusions.values(), key=lambda row: row["path"]
        ),
        "files": rows,
    }


def normalized(text: str) -> str:
    return " ".join(text.replace("\\|", "|").split())


@lru_cache(maxsize=32)
def rust_code(text: str) -> str:
    """Mask source comments/literals while preserving declaration offsets."""
    chars = list(text)
    index = 0
    raw_literal = re.compile(r'(?:br|r)(#*)"')
    character_literal = re.compile(
        r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'"
    )
    while index < len(text):
        end = index
        if text.startswith("//", index):
            end = text.find("\n", index)
            if end < 0:
                end = len(text)
        elif text.startswith("/*", index):
            depth, end = 1, index + 2
            while end < len(text) and depth:
                if text.startswith("/*", end):
                    depth += 1
                    end += 2
                elif text.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
        else:
            raw = raw_literal.match(text, index)
            if raw:
                close = '"' + raw.group(1)
                end = text.find(close, raw.end())
                end = len(text) if end < 0 else end + len(close)
            elif text[index] == '"':
                end = index + 1
                while end < len(text):
                    if text[end] == "\\":
                        end += 2
                    elif text[end] == '"':
                        end += 1
                        break
                    else:
                        end += 1
            elif text[index] == "'":
                # A lifetime has no closing quote; a character literal does.
                character = character_literal.match(text, index)
                if character:
                    end = character.end()
        if end > index:
            for position in range(index, min(end, len(text))):
                if text[position] != "\n":
                    chars[position] = " "
            index = end
        else:
            index += 1
    return "".join(chars)


def test_body(text: str, name: str, python: bool = False) -> str | None:
    if python:
        tree = ast.parse(text)
        matches = [
            node
            for node in ast.walk(tree)
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name == name
        ]
        if len(matches) != 1:
            return None
        node = matches[0]
        return "".join(
            text.splitlines(keepends=True)[node.lineno - 1 : node.end_lineno]
        )
    # Function boundaries are checked without interpreting braces in comments or
    # ordinary/raw Rust string literals. Identifiers inside them cannot issue a test.
    masked = rust_code(text)
    matches = list(
        re.finditer(r"\bfn\s+" + re.escape(name) + r"\s*\([^)]*\)[^{]*\{", masked)
    )
    if len(matches) != 1:
        return None
    match = matches[0]
    depth, end = 1, match.end()
    while end < len(masked) and depth:
        depth += (masked[end] == "{") - (masked[end] == "}")
        end += 1
    if depth:
        return None
    # Include only the function's attached comments/attributes, not a prior test.
    start = match.start()
    prefix = text[:start]
    boundary = max(prefix.rfind("}"), prefix.rfind(";")) + 1
    return text[boundary:end]


def marked_questions(root: Path) -> list[tuple[str, str]]:
    """Retain explicit Rust proof annotations even before their record is merged."""
    result = []
    marker = re.compile(
        r"(?m)^\s*//[^\n]*(?:Native proof:\s*|Implementation contract:\s*|Proof\s+)(naming\.[A-Za-z0-9_.-]+)"
    )
    for parent in [root / "rust", root / "runtime/rust"]:
        for path in parent.rglob("*.rs"):
            if "target" in path.parts:
                continue
            text = path.read_text()
            result.extend(
                (path.relative_to(root).as_posix(), match.group(1))
                for match in marker.finditer(text)
            )
    return result


def environment_errors(record: dict, evidence: dict, doc: str) -> list[str]:
    """Keep measured non-Tcl environments independent of native providers."""
    label = record["id"]
    environments = record.get("environments", [])
    if record["kind"] != "environment-observation":
        return [f"{label}: environment answers require their own record kind"] if "environments" in record else []
    errors = []
    if not environments:
        errors.append(f"{label}: environment observation has no environment answer")
    if not any(item["status"] in {"observed", "failed"} for item in environments):
        errors.append(f"{label}: environment observation has no captured result")
    identifiers = [item["id"] for item in environments]
    if len(identifiers) != len(set(identifiers)):
        errors.append(f"{label}: duplicate environment answer")
    if any(provider["status"] in {"observed", "unsupported", "inspected"} for provider in record["providers"]):
        errors.append(f"{label}: environment result cannot issue a measured native-provider answer")
    for item in environments:
        context = f"{label}/{item['id']}"
        measured = item["status"] in {"observed", "failed"}
        if measured and not item["evidence"]:
            errors.append(f"{context}: captured environment answer has no evidence")
        for key in item["evidence"]:
            if key not in evidence:
                errors.append(f"{context}: unknown evidence ID {key}")
        roles = {evidence[key]["role"] for key in item["evidence"] if key in evidence}
        if measured and "provider" not in roles:
            errors.append(f"{context}: captured environment answer has no provider record")
        if item["status"] == "observed" and "observation" not in roles:
            errors.append(f"{context}: observed environment answer has no observation")
        if item["status"] == "failed" and not roles.intersection({"observation", "limitation"}):
            errors.append(f"{context}: failed environment answer has no failure observation")
        if normalized(item["answer"]) not in normalized(doc):
            errors.append(f"{context}: document omits exact environment answer")
    return errors


def native_attempt_errors(record: dict, evidence: dict, selected: dict, doc: str) -> list[str]:
    """Retain failed setup captures without answering the intended native question."""
    label = record["id"]
    attempt = record.get("attempt")
    if record["kind"] != "native-attempt":
        return [f"{label}: attempt fields require their own record kind"] if attempt is not None else []
    errors = []
    if not isinstance(attempt, dict):
        return [f"{label}: native attempt has no explicit setup-failure record"]
    if any(provider["status"] != "not-tested" for provider in record["providers"]):
        errors.append(f"{label}: native attempt cannot answer the intended provider question")
    if "environments" in record:
        errors.append(f"{label}: native attempt cannot issue an environment answer")
    if attempt.get("intended_question_reached") is not False:
        errors.append(f"{label}: native attempt requires an unreached intended question")
    limits = (
        "The intended question was not reached. The setup captures cannot be used "
        "as a positive answer or as execution, teardown or implementation equivalence evidence."
    )
    if attempt.get("limitations") != limits:
        errors.append(f"{label}: native attempt lacks its strict setup-only limitation")
    if normalized(limits) not in normalized(record["conclusion"]) or normalized(limits) not in normalized(doc):
        errors.append(f"{label}: native attempt conclusion omits its setup-only limitation")
    intended = attempt.get("intended_question", "")
    if not intended or normalized(intended) not in normalized(doc):
        errors.append(f"{label}: document omits the exact unreached intended question")
    probe_id = attempt.get("probe")
    probe = evidence.get(probe_id)
    if not probe or probe["role"] != "input" or probe_id not in selected:
        errors.append(f"{label}: native attempt lacks the retained original probe input")
    captures = attempt.get("captures", [])
    if not captures:
        errors.append(f"{label}: native attempt has no reached setup capture")
    providers = {item["id"]: item for item in record["providers"]}
    seen = set()
    for capture in captures:
        provider_id = capture.get("provider")
        context = f"{label}/{provider_id}"
        provider = providers.get(provider_id)
        if provider is None:
            errors.append(f"{context}: setup capture has no provider answer")
            continue
        input_id = capture.get("input")
        pair = (provider_id, input_id)
        if pair in seen:
            errors.append(f"{context}: duplicate original setup input capture")
        seen.add(pair)
        linked = True
        for field, role in [("input", "input"), ("receipt", "provider"), ("observation", "observation"), ("stderr", "observation")]:
            key = capture.get(field)
            item = evidence.get(key)
            if not item or item["role"] != role or key not in selected:
                errors.append(f"{context}: setup capture lacks retained {field} evidence")
                linked = False
            elif key not in provider["evidence"]:
                errors.append(f"{context}: setup {field} is not attached to this provider")
        if probe_id not in provider["evidence"]:
            errors.append(f"{context}: original probe is not attached to this provider")
        stage = capture.get("failure_stage", "")
        if not stage or normalized(stage) not in normalized(doc):
            errors.append(f"{context}: document omits the reached setup failure stage")
        code = capture.get("guest_code")
        row = capture.get("completion_row", "")
        match = re.fullmatch(r"ORIGINAL\|([1-9][0-9]*)\|([0-9a-f]*)", row)
        if type(code) is not int or code <= 0 or not match or int(match[1]) != code or len(match[2]) % 2:
            errors.append(f"{context}: setup capture is not an original failed guest completion")
        if not linked or not probe:
            continue
        try:
            receipt = json.loads(selected[capture["receipt"]])
            stdout = selected[capture["observation"]]
            stderr = selected[capture["stderr"]]
            if not isinstance(receipt, dict) or not isinstance(stdout, str) or not isinstance(stderr, str):
                raise ValueError("expected complete original receipt and streams")
            rows = stdout.splitlines()
            if receipt.get("rows") != rows or row not in rows:
                raise ValueError("completion row differs from the retained receipt or stream")
            if receipt.get("exit") != 0:
                raise ValueError("setup capture lacks its closed driver process outcome")
            if receipt.get("stdout_sha256") != evidence[capture["observation"]]["sha256"] or receipt.get("stderr_sha256") != evidence[capture["stderr"]]["sha256"]:
                raise ValueError("original process stream digests do not match the receipt")
            if receipt.get("source_sha256", {}).get("original") != evidence[input_id]["sha256"]:
                raise ValueError("setup receipt does not retain the original source input")
            required = receipt.get("required_sha256", {})
            if not isinstance(required, dict) or probe["sha256"] not in required.values():
                raise ValueError("setup receipt does not retain the original probe input")
            versions = [line for line in rows if line.startswith("VERSION|0|")]
            if len(versions) != 1 or bytes.fromhex(versions[0].split("|", 2)[2]).decode() != provider["version"]:
                raise ValueError("setup version stream does not match this provider answer")
        except (ValueError, TypeError, KeyError, AttributeError, UnicodeError) as exc:
            errors.append(f"{context}: {exc}")
    return errors


def validate(root: Path, structure_only: bool = False) -> tuple[list[str], dict]:
    errors: list[str] = []
    schema = json.loads((root / DIRECTORY / "schema.json").read_text())
    manifest = json.loads((root / DIRECTORY / "manifest.json").read_text())
    errors.extend(schema_errors(manifest, schema, schema, "manifest"))
    if errors:
        return errors, {}
    proofs = manifest["proofs"]
    identifiers = [record["id"] for record in proofs]
    if len(identifiers) != len(set(identifiers)):
        errors.append("duplicate proof IDs")
    documents = [record["document"] for record in proofs]
    if len(documents) != len(set(documents)):
        errors.append("different questions share a proof document")
    actual_documents = {
        path.relative_to(root).as_posix()
        for path in (root / DIRECTORY).glob("*.md")
        if path.name not in {"README.md", "consumer-integrations.md"}
    }
    for name in sorted(actual_documents - set(documents)):
        errors.append(f"proof document is not indexed: {name}")
    for path, identifier in marked_questions(root):
        if identifier not in identifiers:
            errors.append(f"test proof annotation is not indexed: {identifier}: {path}")
    pending_tests = 0
    hashes: dict[str, str] = {}
    for record in proofs:
        label = record["id"]
        try:
            doc = relative_file(root, record["document"]).read_text()
            for heading, field in [
                ("Problem statement", "problem_statement"),
                ("Question", "question"),
                ("Conclusion", "conclusion"),
                ("Scope", "scope"),
            ]:
                if not re.search(
                    r"(?m)^## " + re.escape(heading) + r"[ \t]*$", doc
                ) or normalized(record[field]) not in normalized(doc):
                    errors.append(f"{label}: document omits exact {field}")
            if label not in doc:
                errors.append(f"{label}: document omits stable ID")
        except (ValueError, UnicodeError) as exc:
            errors.append(f"{label}: {exc}")
            doc = ""
        evidence = {item["id"]: item for item in record["evidence"]}
        if len(evidence) != len(record["evidence"]):
            errors.append(f"{label}: duplicate evidence IDs")
        selected = {}
        for item in evidence.values():
            try:
                path = relative_file(root, item["path"])
                if item["path"] not in hashes:
                    hashes[item["path"]] = digest(path)
                if hashes[item["path"]] != item["sha256"]:
                    raise ValueError(f"evidence digest changed: {item['path']}")
                data = path.read_bytes()
                value: object = data.decode("utf-8", errors="replace")
                if "json_pointer" in item:
                    value = pointer(json.loads(data), item["json_pointer"])
                if "lines" in item:
                    first, last = item["lines"]
                    # Source coordinates count LF only. Form feed and CR
                    # remain original content, including CR in a CRLF pair.
                    lines = data.decode().split("\n")
                    last_line = lines.pop()
                    lines = [line + "\n" for line in lines]
                    if last_line:
                        lines.append(last_line)
                    if last < first or last > len(lines):
                        raise ValueError(
                            f"invalid evidence line extent: {item['path']}"
                        )
                    value = "".join(lines[first - 1 : last])
                if "contains" in item and item["contains"] not in str(value):
                    raise ValueError(f"evidence omits asserted anchor: {item['path']}")
                selected[item["id"]] = value
            except (ValueError, UnicodeError, KeyError, IndexError) as exc:
                errors.append(f"{label}: {exc}")
        providers = record["providers"]
        if {provider["id"] for provider in providers} != PROVIDERS:
            errors.append(f"{label}: missing/duplicated provider answer")
        for provider in providers:
            measured = provider["status"] in {"observed", "unsupported", "inspected"}
            if measured and not provider["evidence"]:
                errors.append(
                    f"{label}/{provider['id']}: measured answer has no evidence"
                )
            for key in provider["evidence"]:
                if key not in evidence:
                    errors.append(
                        f"{label}/{provider['id']}: unknown evidence ID {key}"
                    )
            if normalized(provider["answer"]) not in normalized(doc):
                errors.append(
                    f"{label}/{provider['id']}: document omits exact provider answer"
                )
            if record["kind"] == "implementation-contract" and measured:
                errors.append(
                    f"{label}: Rust contract cannot issue a measured native-provider answer"
                )
            if record["kind"] == "source-anchor":
                if provider["status"] in {"observed", "unsupported"}:
                    errors.append(
                        f"{label}: source inspection cannot issue a guest execution answer"
                    )
                if provider["status"] == "inspected" and not any(
                    anchor["provider"] == provider["id"]
                    for anchor in record["source_anchors"]
                ):
                    errors.append(
                        f"{label}/{provider['id']}: source inspection lacks its own pinned excerpt"
                    )
        errors.extend(environment_errors(record, evidence, doc))
        errors.extend(native_attempt_errors(record, evidence, selected, doc))
        if record["kind"] == "native-observation" and not any(
            p["status"] in {"observed", "unsupported"} for p in providers
        ):
            errors.append(f"{label}: native observation has no observed provider")
        if record["kind"] == "source-anchor" and not record["source_anchors"]:
            errors.append(f"{label}: source inspection has no pinned excerpt")
        if record["kind"] == "source-anchor" and not any(
            p["status"] == "inspected" for p in providers
        ):
            errors.append(f"{label}: source inspection has no inspected provider")
        for anchor in record["source_anchors"]:
            actual = hashlib.sha256(anchor["snippet"].encode()).hexdigest()
            if actual != anchor["snippet_sha256"]:
                errors.append(f"{label}: source snippet digest changed")
            value = selected.get(anchor["evidence"])

            def attached(node: object) -> bool:
                if isinstance(node, str):
                    return anchor["snippet"] in node
                if isinstance(node, dict):
                    return any(attached(child) for child in node.values())
                if isinstance(node, list):
                    return any(attached(child) for child in node)
                return False

            if not attached(value):
                errors.append(
                    f"{label}: pinned snippet is absent from selected retained evidence"
                )
        for binding in record["tests"]:
            try:
                text = relative_file(root, binding["path"]).read_text()
                body = test_body(
                    text,
                    binding["selector"].split("::")[-1],
                    binding["path"].endswith(".py"),
                )
                if body is None:
                    raise ValueError(f"test selector is absent: {binding['selector']}")
                if binding["status"] == "unlinked":
                    pending_tests += 1
                elif label not in body or record["document"] not in body:
                    raise ValueError(f"test comment is missing: {binding['selector']}")
            except (ValueError, UnicodeError) as exc:
                errors.append(f"{label}: {exc}")
        for owner in record["owners"]:
            try:
                text = relative_file(root, owner["path"]).read_text()
                leaf = owner["symbol"].split("::")[-1]
                if owner["path"].endswith(".py"):
                    present = any(
                        isinstance(node, (ast.FunctionDef, ast.ClassDef))
                        and node.name == leaf
                        for node in ast.walk(ast.parse(text))
                    )
                else:
                    present = re.search(
                        r"\b(?:fn|struct|enum|trait|type|const)\s+"
                        + re.escape(leaf)
                        + r"\b",
                        rust_code(text),
                    )
                if not present:
                    raise ValueError(f"owner declaration is absent: {owner['symbol']}")
            except (ValueError, UnicodeError) as exc:
                errors.append(f"{label}: {exc}")
    census = json.loads((root / DIRECTORY / "corpus-inventory.json").read_text())
    rows = census.get("files", [])
    current = {p.relative_to(root).as_posix(): p for p in corpus_files(root)}
    retained = {row["path"]: row for row in rows}
    if len(retained) != len(rows):
        errors.append("census contains duplicate paths")
    for name in sorted(current.keys() ^ retained.keys()):
        errors.append(f"census path drift: {name}")
    pending = []
    for name, row in retained.items():
        if name in current and digest(current[name]) != row["sha256"]:
            errors.append(f"census digest changed: {name}")
        if name in current and row.get("candidate") is not (
            current[name].suffix == ".json"
        ):
            errors.append(f"census candidate scope changed: {name}")
        if not row.get("candidate"):
            continue
        classification = row.get("classification")
        if classification not in CLASSIFICATIONS:
            errors.append(f"census unsupported classification: {name}")
        if classification == "pending":
            pending.append(name)
        elif not row.get("reason", "").strip():
            errors.append(f"census classification lacks reason: {name}")
        ids = row.get("proof_ids", [])
        if (
            classification
            in {"native-observation", "environment-observation", "authored-contract", "duplicate-evidence"}
            and not ids
        ):
            errors.append(f"census question association is absent: {name}")
        if classification == "unavailable":
            errors.append(f"census required evidence unavailable: {name}")
        for identifier in ids:
            if identifier not in identifiers:
                errors.append(f"census unknown proof ID {identifier}: {name}")
        if classification == "duplicate-evidence":
            try:
                duplicate = relative_file(root, row.get("duplicate_of", ""))
                if digest(duplicate) != row["sha256"]:
                    errors.append(f"census duplicate bytes differ: {name}")
            except ValueError as exc:
                errors.append(f"census {name}: {exc}")
    if not structure_only:
        if pending:
            errors.append(
                f"{len(pending)} candidate records still require explicit question/classification review"
            )
        if pending_tests:
            errors.append(
                f"{pending_tests} test bindings still require exact proof-ID/document comments"
            )
    summary = {
        "questions": len(proofs),
        "corpus_files": len(rows),
        "candidate_records": sum(row.get("candidate", False) for row in rows),
        "pending_candidates": len(pending),
        "unlinked_tests": pending_tests,
        "structural_errors": len(errors),
    }
    return errors, summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument(
        "--structure-only",
        action="store_true",
        help="Audit structure while explicitly reporting pending completion obligations.",
    )
    parser.add_argument(
        "--write-inventory",
        action="store_true",
        help="Reviewable inventory refresh; preserve classifications only for exact unchanged bytes.",
    )
    args = parser.parse_args()
    root = args.root.resolve()
    if args.write_inventory:
        target = root / DIRECTORY / "corpus-inventory.json"
        old = json.loads(target.read_text()) if target.exists() else None
        target.write_text(json.dumps(inventory(root, old), indent=2) + "\n")
        print(
            f"Wrote {target}; classify each pending candidate before the complete gate."
        )
        return 0
    try:
        errors, summary = validate(root, args.structure_only)
    except (ValueError, OSError, KeyError, TypeError) as exc:
        print(f"Proof gate cannot inspect required evidence: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(summary, sort_keys=True))
    for error in errors:
        print(error, file=sys.stderr)
    if args.structure_only and not errors:
        print(
            "Structural audit completed; this is not a complete-coverage or execution result."
        )
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
