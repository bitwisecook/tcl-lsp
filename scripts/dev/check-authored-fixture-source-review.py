#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Authenticate selected old Rust fixtures and compare exact authored payloads.

Source stays in memory. This is a structural authored-source review, not a
native capture, Rust test, formatter-equivalence claim, or acceptance gate.
"""

import argparse
import ast
import base64
import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.parse
from pathlib import Path

PROOF_DIRECTORY = "docs/design/analysis/name-resolution-proofs"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def tokens(raw):
    text = raw.decode()
    result = []
    position = 0
    while position < len(text):
        remaining = text[position:]
        if remaining[0].isspace():
            position += 1
            continue
        if remaining.startswith("//"):
            end = text.find("\n", position)
            position = len(text) if end < 0 else end + 1
            continue
        if remaining.startswith("/*"):
            depth = 1
            position += 2
            while depth:
                if text.startswith("/*", position):
                    depth += 1
                    position += 2
                elif text.startswith("*/", position):
                    depth -= 1
                    position += 2
                else:
                    position += 1
                if position > len(text):
                    raise ValueError("unterminated Rust block comment")
            continue
        raw_string = re.match(r'(?:b|c)?r(#+)?"', remaining)
        if raw_string:
            closing = '"' + (raw_string[1] or "")
            end = text.find(closing, position + raw_string.end())
            if end < 0:
                raise ValueError("unterminated Rust raw literal")
            end += len(closing)
            result.append(text[position:end])
            position = end
            continue
        string = re.match(r'(?:b|c)?"(?:\\.|[^"\\])*"', remaining, re.DOTALL)
        character = re.match(
            r"(?:b)?'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'",
            remaining,
        )
        identifier = re.match(r"[A-Za-z_][A-Za-z0-9_]*|[0-9][A-Za-z0-9_]*", remaining)
        match = string or character or identifier
        if match:
            result.append(match[0])
            position += match.end()
        else:
            result.append(remaining[0])
            position += 1
    return result


def close(items, start):
    pairs = {"{": "}", "[": "]", "(": ")"}
    stack = []
    for index in range(start, len(items)):
        token = items[index]
        if token in pairs:
            stack.append(pairs[token])
        elif token in pairs.values():
            if not stack or token != stack.pop():
                raise ValueError("unbalanced authored Rust literal")
            if not stack:
                return index
    raise ValueError("unterminated authored Rust literal")


def fields(items):
    result = {}
    index = 0
    while index < len(items):
        visibility = []
        if items[index] == "pub":
            visibility = ["pub"]
            index += 1
        if index + 1 >= len(items) or items[index + 1] != ":":
            raise ValueError("unsupported authored field declaration")
        name = items[index]
        if name in result:
            raise ValueError("duplicate authored field declaration")
        start = index + 2
        index = start
        while index < len(items) and items[index] != ",":
            if items[index] in {"{", "[", "("}:
                index = close(items, index)
            index += 1
        result[name] = visibility + items[start:index]
        index += 1
    return result


def analyse(raw):
    items = tokens(raw)
    structures = {}
    erased = []
    for index, token in enumerate(items):
        if token == "struct":
            if items[index + 2] != "{":
                raise ValueError("unsupported authored struct shape")
            end = close(items, index + 2)
            structures[items[index + 1]] = fields(items[index + 3 : end])
            erased.append((index, end + 1))
    wrappers = []
    for index, token in enumerate(items):
        if token != "impl":
            continue
        begin = index
        while items[begin] != "{":
            begin += 1
        end = close(items, begin)
        body = items[index : end + 1]
        # This exact pattern permits only a shared borrow of a named wrapper.
        joined = "".join(body)
        match = re.fullmatch(
            r"implcore::ops::Dereffor(\w+)\{typeTarget=(\w+);fnderef\(&self\)->&Self::Target\{&self\.(\w+)\}\}",
            joined,
        )
        if not match:
            raise ValueError(
                "authored impl is not the reviewed pure shared-borrow Deref"
            )
        owner, target, field = match.groups()
        if structures[owner][field] != [target]:
            raise ValueError("Deref target does not match its stored wrapper field")
        wrappers.append({"owner": owner, "target": target, "field": field})
        erased.append((index, end + 1))

    def flatten_type(name, ancestry=()):
        if name in ancestry:
            raise ValueError("recursive authored wrapper type")
        result = {}
        for field, value in structures[name].items():
            if len(value) == 1 and value[0] in structures:
                child = flatten_type(value[0], (*ancestry, name))
            else:
                child = {field: value}
            if result.keys() & child.keys():
                raise ValueError("wrapper flattening would alias field identities")
            result.update(child)
        return result

    def flatten_value(value):
        result = {}
        for field, literal in fields(value).items():
            if (
                len(literal) >= 3
                and literal[0] in structures
                and literal[1] == "{"
                and close(literal, 1) == len(literal) - 1
            ):
                child = flatten_value(literal[2:-1])
            else:
                child = {field: literal}
            if result.keys() & child.keys():
                raise ValueError(
                    "wrapper flattening would alias literal field identities"
                )
            result.update(child)
        return result

    rows = []
    covered = {index for start, end in erased for index in range(start, end)}
    index = 0
    while index < len(items) - 1:
        if (
            index in covered
            or items[index + 1] != "{"
            or not re.fullmatch(r"[A-Z]\w*", items[index])
        ):
            index += 1
            continue
        end = close(items, index + 1)
        rows.append(
            {"type": items[index], "fields": flatten_value(items[index + 2 : end])}
        )
        erased.append((index, end + 1))
        index = end + 1
    covered = {index for start, end in erased for index in range(start, end)}
    rest = [token for index, token in enumerate(items) if index not in covered]
    row_types = {row["type"] for row in rows}
    types = {name: flatten_type(name) for name in row_types if name in structures}
    reachable = {name for name in row_types if name in structures}
    pending = list(reachable)
    while pending:
        name = pending.pop()
        for value in structures[name].values():
            if len(value) == 1 and value[0] in structures and value[0] not in reachable:
                reachable.add(value[0])
                pending.append(value[0])
    if reachable != structures.keys():
        raise ValueError("unreviewed authored struct outside fixed row storage")
    for wrapper in wrappers:
        if any(value != ["bool"] for value in flatten_type(wrapper["target"]).values()):
            raise ValueError("regrouped authored storage contains a non-boolean leaf")
    normalised = []
    index = 0
    decoder_changes = []
    decoder_ranges = []
    for offset, token in enumerate(rest[:-1]):
        if token == "fn" and rest[offset + 1] == "unhex":
            begin = offset + 2
            while rest[begin] != "{":
                begin += 1
            decoder_ranges.append((begin, close(rest, begin)))
    while index < len(rest):
        if rest[index : index + 5] == [".", "chunks", "(", "2", ")"] and rest[
            index + 5 : index + 9
        ] == [".", "filter", "(", "|"]:
            name = rest[index + 9]
            suffix = [name, "|", name, ".", "len", "(", ")", "=", "=", "2", ")"]
            if rest[index + 9 : index + 20] != suffix:
                raise ValueError("unsupported authored partial-byte filter")
            if not any(start < index < end for start, end in decoder_ranges):
                raise ValueError(
                    "full-pair iterator normalisation escapes unhex helper"
                )
            normalised.extend([".", "chunks_exact", "(", "2", ")"])
            decoder_changes.append(
                "same full pairs and ignored remainder via exact chunks(2).filter(len==2)"
            )
            index += 20
        elif rest[index : index + 15] == [
            ".",
            "as_chunks",
            ":",
            ":",
            "<",
            "2",
            ">",
            "(",
            ")",
            ".",
            "0",
            ".",
            "iter",
            "(",
            ")",
        ]:
            if not any(start < index < end for start, end in decoder_ranges):
                raise ValueError(
                    "full-pair iterator normalisation escapes unhex helper"
                )
            normalised.extend([".", "chunks_exact", "(", "2", ")"])
            decoder_changes.append(
                "same full pairs and ignored remainder via exact as_chunks::<2>().0.iter()"
            )
            index += 15
        else:
            normalised.append(rest[index])
            index += 1
    hex_strings = []
    odd_hex_fields = set()
    for row in rows:
        for field, value in row["fields"].items():
            for token in value:
                if token.startswith('"'):
                    literal = ast.literal_eval(token)
                    if re.fullmatch("[0-9a-fA-F]*", literal):
                        hex_strings.append(literal)
                        if len(literal) % 2:
                            odd_hex_fields.add(field)
    return {
        "rows": rows,
        "flattened_types": types,
        "remaining_tokens": rest,
        "normalised_decoder_tokens": normalised,
        "decoder_changes": decoder_changes,
        "literal_hex_values": len(hex_strings),
        "odd_literal_hex_fields": sorted(odd_hex_fields),
        "wrappers": wrappers,
        "structures": structures,
    }


def original(root, row, fetch):
    environment = dict(os.environ, GIT_NO_LAZY_FETCH="1")

    def git(*arguments):
        return subprocess.run(
            ["git", "-c", "remote.origin.promisor=false", *arguments],
            cwd=root,
            env=environment,
            capture_output=True,
            check=False,
            timeout=20,
        )

    tree = git("ls-tree", "-r", row["original_commit"], "--", row["path"])
    entries = tree.stdout.decode().strip().split("\t")
    if (
        tree.returncode == 0
        and len(entries) == 2
        and entries[1] == row["path"]
        and entries[0].split()[-1] == row["original_blob"]
    ):
        blob = git("cat-file", "blob", row["original_blob"])
        if blob.returncode == 0:
            return blob.stdout, "local authenticated commit/tree/blob objects"
    if not fetch:
        raise ValueError(
            "required original is unavailable offline; origin reads were not requested"
        )
    remote = git("config", "--get", "remote.origin.url")
    if (
        remote.returncode
        or remote.stdout.decode().strip()
        != "https://github.com/bitwisecook/tcl-lsp.git"
    ):
        raise ValueError("configured origin differs from reviewed source origin")
    response = subprocess.run(
        [
            "gh",
            "api",
            "--method",
            "GET",
            "repos/bitwisecook/tcl-lsp/contents/"
            + urllib.parse.quote(row["path"], safe="/"),
            "-f",
            "ref=" + row["original_commit"],
        ],
        cwd=root,
        capture_output=True,
        check=False,
        timeout=45,
    )
    if response.returncode:
        raise ValueError("authenticated origin fixture read failed")
    value = json.loads(response.stdout)
    if value["sha"] != row["original_blob"] or value["encoding"] != "base64":
        raise ValueError("origin fixture commit/path/blob identity differs")
    return base64.b64decode(
        value["content"]
    ), "explicit configured-origin immutable commit/path read"


def validate(root, contract_path, fetch=False):
    root = root.resolve()
    contract_raw = contract_path.read_bytes()
    contract = json.loads(contract_raw)
    if (
        contract["schema_version"] != 1
        or contract["kind"] != "maintained-rust-fixed-control-source-regrouping"
    ):
        raise ValueError("unsupported authored fixture review contract")
    if (
        len(contract["files"]) != 7
        or len({row["path"] for row in contract["files"]}) != 7
    ):
        raise ValueError("reviewed seven-source roster changed")
    manifest_path = root / PROOF_DIRECTORY / "manifest.json"
    census_path = root / PROOF_DIRECTORY / "corpus-inventory.json"
    manifest_raw, census_raw = manifest_path.read_bytes(), census_path.read_bytes()
    manifest, census = json.loads(manifest_raw), json.loads(census_raw)
    census_rows = {row["path"]: row for row in census["files"]}
    results = []
    for item in contract["files"]:
        path = item["path"]
        source = (root / path).resolve()
        if (
            Path(path).is_absolute()
            or not source.is_relative_to(root)
            or not path.startswith("runtime/rust/tests/data/native_")
            or not path.endswith("/software_controls.rs")
        ):
            raise ValueError(
                "authored fixture path escapes the reviewed source boundary"
            )
        if not re.fullmatch(
            "[0-9a-f]{40}", item["original_commit"]
        ) or not re.fullmatch("[0-9a-f]{40}", item["original_blob"]):
            raise ValueError("invalid immutable original source identity")
        if any(
            evidence["path"] == path
            for record in manifest["proofs"]
            for evidence in record["evidence"]
        ):
            raise ValueError(
                "reviewed source gained a direct proof evidence binding; independent claim review required"
            )
        if census_rows[path]["sha256"] not in {
            item["original_sha256"],
            item["current_sha256"],
        }:
            raise ValueError(
                "authored source census pin differs from both reviewed snapshots"
            )
        old, availability = original(root, item, fetch)
        new = source.read_bytes()
        if sha(new) != item["current_sha256"] or len(new) != item["current_bytes"]:
            raise ValueError("current authored fixture snapshot changed")
        if (
            sha(old) != item["original_sha256"]
            or hashlib.sha1(b"blob " + str(len(old)).encode() + b"\0" + old).hexdigest()
            != item["original_blob"]
        ):
            raise ValueError("original fixture SHA256/Git blob authentication failed")
        prior, current = analyse(old), analyse(new)
        if (
            prior["rows"] != current["rows"]
            or prior["flattened_types"] != current["flattened_types"]
            or prior["normalised_decoder_tokens"]
            != current["normalised_decoder_tokens"]
        ):
            raise ValueError(
                "authored payload/type/code delta exceeds reviewed storage and full-pair iterator changes"
            )
        if (
            len(current["rows"]) != item["row_count"]
            or sha(json.dumps(current["rows"], sort_keys=True).encode())
            != item["flattened_rows_sha256"]
        ):
            raise ValueError("authored fixed row roster or payload fingerprint changed")
        if current["odd_literal_hex_fields"]:
            raise ValueError(
                "reviewed literal byte domain gained an odd hexadecimal spelling"
            )
        if source.read_bytes() != new:
            raise ValueError("authored fixture changed during review")
        row = {
            "path": path,
            "original_commit": item["original_commit"],
            "original_blob": item["original_blob"],
            "original_sha256": sha(old),
            "current_sha256": sha(new),
            "current_bytes": len(new),
            "original_availability": availability,
            "row_count": len(current["rows"]),
            "exact_flattened_row_payloads_equal": True,
            "exact_flattened_row_field_types_equal": True,
            "remaining_code_equal_after_exact_full_pair_iterator_normalisation": True,
            "full_pair_iterator_delta": current["decoder_changes"],
            "literal_hex_values": current["literal_hex_values"],
            "odd_literal_hex_fields": current["odd_literal_hex_fields"],
            "current_pure_storage_wrappers": current["wrappers"],
            "flattened_rows_sha256": sha(
                json.dumps(current["rows"], sort_keys=True).encode()
            ),
            "native_execution": False,
        }
        results.append(row)
    if (
        manifest_path.read_bytes() != manifest_raw
        or census_path.read_bytes() != census_raw
    ):
        raise ValueError("proof metadata changed during authored fixture review")
    return {
        "schema_version": 1,
        "scope": "Authenticated original/current maintained Rust fixed-control source review. Exact leaf field identities/types/literals and remaining code compared after pure storage Deref regrouping and equivalent full-pair iterator spelling. Comments/whitespace excluded. No original source content retained; no native/Rust execution, ABI/layout equivalence or original capture renewal.",
        "verifier_sha256": sha(Path(__file__).read_bytes()),
        "contract_sha256": sha(contract_raw),
        "input_manifest_sha256": sha(manifest_raw),
        "input_census_sha256": sha(census_raw),
        "files": results,
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
        subprocess.TimeoutExpired,
    ):
        print(
            "Authored fixture source review failed; required originals, pins or exact reviewed transformation are unavailable.",
            file=sys.stderr,
        )
        return 1
    print(
        json.dumps(
            {
                "source_files": len(result["files"]),
                "fixed_rows": sum(row["row_count"] for row in result["files"]),
                "native_or_rust_execution": "not run",
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
