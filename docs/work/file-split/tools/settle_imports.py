#!/usr/bin/env python3
"""Settle the imports of a package after a split, from the compiler's own diagnostics.

    settle_imports.py PACKAGE CRATE_DIR [--log FILE]

Run once the package compiles for every target with the new modules still
starting `use super::*;`. Repeats until nothing changes:

1. Each `use super::*;` clippy reports becomes the list of names the module
   uses. A name only the test build uses gets its own `#[cfg(test)]` line.
   (`cargo clippy --fix` cannot do this for a whole crate: one suggestion that
   names a function-local alias makes it discard every fix.)
2. `direct_imports.py` points each name at the module that defines it.
3. An import no build uses is removed; one only the test build uses is put
   under `#[cfg(test)]`.

Cargo says how each unit was built, so what the plain build reports and what
the test builds report are kept apart.
"""

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

from apply_split import is_test_file, prologue_end
from direct_imports import leaves, rewrite

TOOLS = Path(__file__).resolve().parent
MAX_ROUNDS = 10
LOG = None  # an open file when --log names one: every edit, before and after


def cargo(package: str, manifest: str, prefix: str) -> tuple[list[dict], list[dict], bool]:
    """Clippy's diagnostics for every target, split into the plain build and
    the test builds, and whether everything compiled.

    One job at a time, so the messages of a unit sit directly before the
    record that says how it was built.
    """
    # The wildcard lint is asked for by name: not every crate inherits the workspace's lints.
    command = ["cargo", "clippy", "--message-format=json", "--manifest-path", manifest, "-p", package, "--all-targets", "-j", "1"]
    command += ["--", "-W", "clippy::wildcard_imports"]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    plain: list[dict] = []
    tests: list[dict] = []
    pending: list[dict] = []
    plain_built = False
    succeeded = False
    for line in result.stdout.split("\n"):
        if not line.startswith("{"):
            continue
        record = json.loads(line)
        reason = record.get("reason")
        if reason == "compiler-message":
            rebase(record["message"], prefix)
        if reason == "build-finished":
            succeeded = bool(record.get("success"))
        elif f"/{package}#" not in record.get("package_id", "") and f"#{package}@" not in record.get("package_id", ""):
            continue
        elif reason == "compiler-message":
            pending.append(record["message"])
        elif reason == "compiler-artifact" and "custom-build" not in record["target"]["kind"]:
            if record["profile"]["test"]:
                tests += pending
            else:
                plain += pending
                plain_built = True
            pending = []
    # What is left belongs to a unit that did not build.
    (tests if plain_built else plain).extend(pending)
    return plain, tests, succeeded


def rebase(message: dict, prefix: str) -> None:
    """Make a diagnostic's file names relative to the repository, not to the cargo workspace."""
    if not prefix:
        return
    for span in message.get("spans", []):
        if not span["file_name"].startswith("/"):
            span["file_name"] = f"{prefix}/{span['file_name']}"
    for child in message.get("children", []):
        rebase(child, prefix)


def workspace_prefix(root: Path, manifest: str) -> str:
    """The cargo workspace root of `manifest`, relative to the repository root."""
    located = subprocess.run(
        ["cargo", "locate-project", "--workspace", "--message-format", "plain", "--manifest-path", manifest],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    relative = Path(located).parent.resolve().relative_to(root.resolve()).as_posix()
    return "" if relative == "." else relative


def code_of(message: dict) -> str:
    return (message.get("code") or {}).get("code", "")


def primary_spans(message: dict) -> list[dict]:
    return [span for span in message.get("spans", []) if span.get("is_primary")]


USE_START = re.compile(r"[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?use\b")


def line_start(text: str, offset: int) -> int:
    return text.rfind("\n", 0, offset) + 1


def statement_bounds(text: str, offset: int) -> tuple[int, int]:
    """The `use ...;` statement around `offset`, with its attribute lines."""
    start = line_start(text, offset)
    while start > 0 and not USE_START.match(text, start):
        start = line_start(text, start - 1)
    while start > 0:
        previous = line_start(text, start - 1)
        if not text[previous:start].lstrip().startswith("#["):
            break
        start = previous
    end = text.index(";", offset) + 1
    if text[end : end + 1] == "\n":
        end += 1
    return start, end


def split_statement(statement: str) -> tuple[str, str, str]:
    """`(attributes, visibility, tree)` of one use statement."""
    match = re.match(r"((?:\s*#\[[^\n]*\]\n)*)\s*((?:pub(?:\([^)]*\))?\s+)?)use\s+(.*);\s*$", statement, re.S)
    if match is None:
        raise ValueError(f"not a use statement: {statement!r}")
    return match.group(1), match.group(2), " ".join(match.group(3).split())


def render(attributes: str, visibility: str, imports: list[tuple[tuple[str, ...], str]]) -> str:
    groups: dict[tuple[str, ...], list[str]] = defaultdict(list)
    for path, bound in imports:
        if path[-1] == "*":
            groups[path[:-1]].append("*")
        elif bound == path[-1]:
            groups[path[:-1]].append(bound)
        else:
            groups[path[:-1]].append(f"{path[-1]} as {bound}")
    lines = []
    for prefix, names in sorted(groups.items()):
        names = sorted(set(names))
        joined = names[0] if len(names) == 1 else "{" + ", ".join(names) + "}"
        lines.append(f"{attributes}{visibility}use {'::'.join((*prefix, joined))};\n")
    return "".join(lines)


def wildcard_names(message: dict) -> tuple[str, int, int, list[tuple[tuple[str, ...], str]]] | None:
    for child in message.get("children", []):
        for span in child.get("spans", []):
            replacement = span.get("suggested_replacement")
            if replacement:
                return span["file_name"], span["byte_start"], span["byte_end"], leaves(replacement)
    return None


def narrow(plain: list[dict], tests: list[dict], compiled_plain: set[str], root: Path, scope: set[str]) -> int:
    """Rewrite each reported `use super::*;` in the files the split created (`scope`)."""
    found: dict[tuple[str, int, int], dict[str, list]] = defaultdict(dict)
    for build, messages in (("plain", plain), ("test", tests)):
        for message in messages:
            if code_of(message) != "clippy::wildcard_imports":
                continue
            names = wildcard_names(message)
            if names and names[0] in scope:
                found[names[:3]][build] = names[3]
    edits: dict[str, list[tuple[int, int, str]]] = defaultdict(list)
    for (file_name, start, end), builds in found.items():
        data = (root / file_name).read_bytes()
        text = data.decode("utf-8")
        begin, finish = statement_bounds(text, len(data[:start].decode("utf-8")))
        attributes, visibility, _ = split_statement(text[begin:finish])
        base = builds.get("plain")
        if base is None and file_name in compiled_plain:
            base = []  # the plain build takes nothing from the parent
        if base is None:
            replacement = render(attributes, visibility, builds["test"])
        else:
            extra = [leaf for leaf in builds.get("test", []) if leaf not in base]
            replacement = render(attributes, visibility, base)
            if extra and "cfg(test)" not in attributes:
                replacement += render(attributes + "#[cfg(test)]\n", visibility, extra)
        edits[file_name].append((begin, finish, replacement))
    return apply(edits, root)


def apply(edits: dict[str, list[tuple[int, int, str]]], root: Path) -> int:
    count = 0
    for file_name, changes in edits.items():
        path = root / file_name
        text = path.read_text(encoding="utf-8")
        last = len(text) + 1
        for begin, finish, replacement in sorted(set(changes), reverse=True):
            if finish > last:
                continue  # overlaps an edit already made; the next round sees it
            if LOG is not None:
                LOG.write(f"{file_name}:{text.count(chr(10), 0, begin) + 1}\n- {text[begin:finish].strip()}\n+ {replacement.strip()}\n")
            text = text[:begin] + replacement + text[finish:]
            last = begin
            count += 1
        path.write_text(text, encoding="utf-8")
    return count


def flagged(messages: list[dict], code: str) -> dict[str, set[tuple[int, int]]]:
    """Per file, the primary spans (byte ranges) of every `code` diagnostic."""
    spans: dict[str, set[tuple[int, int]]] = defaultdict(set)
    for message in messages:
        if code_of(message) == code:
            for span in primary_spans(message):
                spans[span["file_name"]].add((span["byte_start"], span["byte_end"]))
    return spans


def drop_or_gate(plain: list[dict], tests: list[dict], compiled_plain: set[str], root: Path, unresolved: bool) -> int:
    """Remove imports nothing uses; gate the ones only the test build uses.

    With `unresolved`, the spans are names a narrowed list should not have
    carried (clippy lists function-local aliases); they are simply removed.
    """
    code = "E0432" if unresolved else "unused_imports"
    in_plain, in_test = flagged(plain, code), flagged(tests, code)
    edits: dict[str, list[tuple[int, int, str]]] = defaultdict(list)
    for file_name in set(in_plain) | set(in_test):
        path = root / file_name
        data = path.read_bytes()
        text = data.decode("utf-8")
        statements: dict[tuple[int, int], dict[str, set[str]]] = defaultdict(lambda: {"remove": set(), "gate": set()})
        for span in in_plain[file_name] | in_test[file_name]:
            try:
                begin, finish = statement_bounds(text, len(data[: span[0]].decode("utf-8")))
                names = {bound for _, bound in leaves(data[span[0] : span[1]].decode("utf-8"))}
            except (ValueError, IndexError):
                continue
            plain_flags, test_flags = span in in_plain[file_name], span in in_test[file_name]
            if unresolved or (plain_flags and test_flags) or (test_flags and file_name not in compiled_plain):
                statements[(begin, finish)]["remove"] |= names
            elif plain_flags:
                statements[(begin, finish)]["gate"] |= names
        for (begin, finish), verdict in statements.items():
            try:
                attributes, visibility, tree = split_statement(text[begin:finish])
            except ValueError:
                continue
            imports = leaves(tree)
            if "cfg(test)" in attributes:
                verdict["remove"] |= verdict["gate"]
                verdict["gate"] = set()
            kept = [leaf for leaf in imports if leaf[1] not in verdict["remove"] | verdict["gate"]]
            gated = [leaf for leaf in imports if leaf[1] in verdict["gate"] and leaf[1] not in verdict["remove"]]
            if len(kept) == len(imports):
                continue
            replacement = render(attributes, visibility, kept) + render(attributes + "#[cfg(test)]\n", visibility, gated)
            edits[file_name].append((begin, finish, replacement))
    return apply(edits, root)


NAME_ERRORS = ("E0405", "E0412", "E0422", "E0423", "E0425", "E0433", "E0531", "E0532")


def restore_names(plain: list[dict], tests: list[dict], candidates: set[str], tried: set[tuple[str, str]], root: Path) -> int:
    """Import from the parent a name a narrowed module still needs.

    Clippy's list of what a wildcard import supplied misses a module used
    only as a path prefix (`tests::FIXTURE`). The compiler then says the name
    is not in scope; it is in the parent's.
    """
    wanted: dict[str, dict[str, str]] = defaultdict(dict)
    for build, messages in (("test", tests), ("plain", plain)):
        for message in messages:
            if message.get("level") != "error" or code_of(message) not in NAME_ERRORS:
                continue
            for span in primary_spans(message):
                file_name = span["file_name"]
                if file_name not in candidates:
                    continue
                written = (root / file_name).read_bytes()[span["byte_start"] : span["byte_end"]].decode("utf-8")
                name = re.match(r"(?:r#)?\w+", written)
                if name and (file_name, name.group(0)) not in tried:
                    wanted[file_name][name.group(0)] = "" if build == "plain" else "#[cfg(test)]\n"
    count = 0
    for file_name, names in wanted.items():
        path = root / file_name
        text = path.read_text(encoding="utf-8")
        first_use = re.search(r"^(?:#\[[^\n]*\]\n)*(?:pub(?:\([^)]*\))? )?use ", text, re.M)
        at = first_use.start() if first_use else prologue_end(text)
        block = "".join(f"{gate}use super::{name};\n" for name, gate in sorted(names.items()))
        if not first_use:
            block = "\n\n" + block
        if LOG is not None:
            LOG.write(f"{file_name}: restored\n+ {block.strip()}\n")
        path.write_text(text[:at] + block + text[at:], encoding="utf-8")
        tried.update((file_name, name) for name in names)
        count += len(names)
    return count


def files_of(messages: list[dict]) -> set[str]:
    return {span["file_name"] for message in messages for span in message.get("spans", [])}


def created(root: Path, crate_dir: str) -> list[str]:
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "--others", "--exclude-standard", "--", crate_dir],
        check=True, capture_output=True, text=True,
    ).stdout.split("\n")
    return sorted((name for name in listed if name.endswith(".rs")), key=lambda name: (name.count("/"), name))


def source_files(root: Path, crate_dir: str, part: str) -> set[str]:
    return {path.relative_to(root).as_posix() for path in (root / crate_dir / part).rglob("*.rs")}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("package")
    parser.add_argument("crate_dir")
    parser.add_argument("--log", type=Path, help="append every edit made, before and after, to this file")
    arguments = parser.parse_args()
    if arguments.log:
        global LOG
        LOG = arguments.log.open("a", encoding="utf-8")
    root = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True, text=True).stdout.strip())
    manifest = f"{arguments.crate_dir}/Cargo.toml"
    prefix = workspace_prefix(root, manifest)
    tried: set[tuple[str, str]] = set()
    for round_number in range(1, MAX_ROUNDS + 1):
        plain, tests, compiled = cargo(arguments.package, manifest, prefix)
        compiled_plain = {name for name in source_files(root, arguments.crate_dir, "src") if not is_test_file(root / name)}
        # Names a narrowed list should not carry go first: nothing else can be trusted until they do.
        changed = drop_or_gate(plain, tests, compiled_plain, root, unresolved=True)
        if not changed and not compiled:
            changed = restore_names(plain, tests, set(created(root, arguments.crate_dir)), tried, root)
            if not changed:
                print("settle_imports: the package does not compile; fix that first (cargo check --all-targets)")
                for message in [*plain, *tests]:
                    if message.get("level") == "error" and primary_spans(message):
                        span = primary_spans(message)[0]
                        print(f"    {span['file_name']}:{span['line_start']}: {code_of(message)} {message['message'][:160]}")
                return 1
        elif not changed:
            changed = narrow(plain, tests, compiled_plain, root, set(created(root, arguments.crate_dir)))
            for name in created(root, arguments.crate_dir):
                changed += rewrite(root / name)
            if not changed:
                changed = drop_or_gate(plain, tests, compiled_plain, root, unresolved=False)
        print(f"    round {round_number}: {changed} import edits", flush=True)
        if LOG is not None:
            LOG.write(f"== end of round {round_number}\n")
            LOG.flush()
        if not changed:
            return 0
    print("settle_imports: still changing after the last round")
    return 1


if __name__ == "__main__":
    sys.exit(main())
