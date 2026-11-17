#!/usr/bin/env python3
"""Inventory of the Rust tests: what each one holds, what it cites, what it repeats.

Reads every `#[test]` under the given roots without compiling anything and writes

  inventory.tsv         one row per test
  clusters_exact.tsv    tests whose bodies are the same text
  clusters_overlap.tsv  tests that run the same Tcl sources through the same helpers against the same codes
  fixtures_shared.tsv   Tcl sources used by tests in more than one file
  summary.md            the counts a person reads first

The classes are heuristics for triage, not verdicts: a person or an area session
decides what a test is worth.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
TEST_ATTR = re.compile(r"#\[\s*(?:[A-Za-z_][A-Za-z0-9_]*\s*::\s*)*test\b[^\]]*\]")
FN = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")
MOD = re.compile(r"\bmod\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{")
CODE = re.compile(r"\b(?:IRULE\d{4}|[WEIOT]\d{3,4})\b")
ORACLE = re.compile(
    r"tclsh|jimsh|\bjim\b|\bC Tcl\b|\b8\.[456]\.\d+\b|\b9\.[01]\.\d+\b|\bTcl 8\.[456]\b|\bTcl 9\.[01]\b",
    re.IGNORECASE,
)
ISSUE = re.compile(r"(?:issue[_ #]*|#)(\d{3,5})\b", re.IGNORECASE)
CONTRACT = re.compile(r"docs/design|\bcontract\b|\bKCS\b|\bkcs-", re.IGNORECASE)
TCLISH = re.compile(
    r"\$[A-Za-z_:{]|\[[a-z:]|\b(?:set|proc|puts|expr|if|when|namespace|foreach|while|return|"
    r"catch|switch|package|oo::|dict|list|string|incr|variable|global|upvar|uplevel|eval|"
    r"lappend|lindex|regexp|after|trace|rename|interp|source)\b"
)
FORMAT_HOLE = re.compile(r"\{[A-Za-z_][A-Za-z_0-9]*\}")
DEBUG_HOLE = re.compile(r"\{[A-Za-z_0-9]*:[^}]*\}")
TCL_HEADS = frozenset(
    "set proc puts expr if when namespace foreach while return catch switch package dict list string "
    "incr variable global upvar uplevel eval lappend lindex regexp after trace rename interp source "
    "for try lassign unset array info format".split()
)
SMELLS = (
    "rust_behaviour",
    "residual",
    "coverage",
    "depth",
    "gap",
    "mutant",
    "kill",
    "snapshot",
    "golden",
    "regression",
    "pin",
)
NOT_HELPERS = {
    "assert", "assert_eq", "assert_ne", "format", "vec", "matches", "panic", "println", "eprintln",
    "contains", "iter", "any", "all", "to_string", "unwrap", "expect", "len", "is_empty", "map",
    "filter", "collect", "clone", "into", "as_str", "as_ref", "as_deref", "is_some", "is_none",
    "is_ok", "is_err", "find", "next", "get", "first", "last", "count", "join", "starts_with",
    "ends_with", "to_owned", "from", "new", "Some", "Ok", "Err", "None", "push", "extend", "into_iter",
    "unwrap_or", "unwrap_or_default", "cloned", "copied", "enumerate", "zip", "rev", "position",
    "trim", "lines", "split", "replace", "then", "and_then", "or_else", "ok", "err", "if", "for",
    "while", "match", "let", "fn", "String", "Vec", "Box", "Arc", "debug_assert", "write", "writeln",
    "filter_map", "flat_map", "unwrap_or_else", "is_some_and", "is_none_or", "iter_mut", "keys",
    "values", "sort", "sort_unstable", "dedup", "insert", "remove", "to_vec", "as_slice", "min",
    "max", "sum", "fold", "take", "skip", "chars", "bytes", "parse", "default", "eq", "ne",
}


@dataclass
class Scan:
    """A Rust source with strings and comments blanked, and what was blanked."""

    skeleton: str
    strings: list[tuple[int, int, str]] = field(default_factory=list)
    comments: list[tuple[int, int, str]] = field(default_factory=list)


def _unescape(raw: str) -> str:
    out: list[str] = []
    i = 0
    n = len(raw)
    simple = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\", '"': '"', "'": "'"}
    while i < n:
        ch = raw[i]
        if ch != "\\" or i + 1 >= n:
            out.append(ch)
            i += 1
            continue
        nxt = raw[i + 1]
        if nxt == "\n":
            i += 2
            while i < n and raw[i] in " \t\n\r":
                i += 1
        elif nxt in simple:
            out.append(simple[nxt])
            i += 2
        elif nxt == "x" and i + 3 < n:
            out.append(_code_point(raw[i + 2 : i + 4]))
            i += 4
        elif nxt == "u" and i + 2 < n and raw[i + 2] == "{":
            close = raw.find("}", i + 3)
            digits = raw[i + 3 : close] if close != -1 else ""
            out.append(_code_point(digits))
            i = close + 1 if close != -1 else n
        else:
            out.append(nxt)
            i += 2
    return "".join(out)


def _code_point(digits: str) -> str:
    try:
        return chr(int(digits.replace("_", ""), 16))
    except (ValueError, OverflowError):
        return "?"


def scan_rust(text: str) -> Scan:
    """Blank strings and comments so braces and keywords can be read safely."""
    out = list(text)
    strings: list[tuple[int, int, str]] = []
    comments: list[tuple[int, int, str]] = []
    n = len(text)
    i = 0

    def blank(start: int, end: int) -> None:
        for k in range(start, end):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        ch = text[i]
        two = text[i : i + 2]
        if two == "//":
            end = text.find("\n", i)
            end = n if end == -1 else end
            comments.append((i, end, text[i:end]))
            blank(i, end)
            i = end
        elif two == "/*":
            depth = 1
            j = i + 2
            while j < n and depth:
                if text[j : j + 2] == "/*":
                    depth += 1
                    j += 2
                elif text[j : j + 2] == "*/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            comments.append((i, j, text[i:j]))
            blank(i, j)
            i = j
        elif ch == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            j = min(j, n - 1)
            strings.append((i, j + 1, _unescape(text[i + 1 : j])))
            blank(i + 1, j)
            i = j + 1
        elif ch == "r" and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            j = i + 1
            while j < n and text[j] == "#":
                j += 1
            if j < n and text[j] == '"':
                closer = '"' + "#" * (j - i - 1)
                end = text.find(closer, j + 1)
                end = n - len(closer) if end == -1 else end
                strings.append((i, end + len(closer), text[j + 1 : end]))
                blank(j + 1, end)
                i = end + len(closer)
            else:
                i += 1
        elif ch == "'":
            m = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'").match(text, i)
            if m:
                blank(i + 1, m.end() - 1)
                i = m.end()
            else:
                i += 1
        else:
            i += 1
    return Scan("".join(out), strings, comments)


def _match_brace(skeleton: str, open_at: int) -> int:
    depth = 0
    for k in range(open_at, len(skeleton)):
        c = skeleton[k]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return k
    return len(skeleton) - 1


@dataclass
class Test:
    crate: str
    target: str
    module: str
    name: str
    file: str
    line: int
    lines: int
    fixtures: tuple[str, ...]
    codes: tuple[str, ...]
    helpers: tuple[str, ...]
    oracle: bool
    issues: tuple[str, ...]
    contract: bool
    smells: tuple[str, ...]
    body_hash: str
    failing: bool = False

    @property
    def path(self) -> str:
        return f"{self.module}::{self.name}" if self.module else self.name

    @property
    def fixture_key(self) -> str:
        return _digest("\x1f".join(sorted(self.fixtures))) if self.fixtures else ""

    @property
    def overlap_key(self) -> str:
        if not self.fixtures:
            return ""
        asserted = ",".join(self.codes) + "\x1e" + ",".join(sorted(self.helpers))
        return _digest("\x1f".join(sorted(self.fixtures)) + "\x1e" + asserted)

    @property
    def value_class(self) -> str:
        if not self.fixtures:
            return "D-internal"
        if self.oracle:
            return "A-oracle"
        if self.issues or self.contract:
            return "B-cited"
        return "C-uncited"


def _digest(text: str) -> str:
    return hashlib.sha1(text.encode("utf-8", "replace")).hexdigest()[:12]


def _normalise_ws(text: str) -> str:
    return " ".join(text.split())


def is_fixture(literal: str) -> bool:
    """A string literal that reads as Tcl source rather than as a message."""
    text = literal.strip()
    if len(text) < 6 or not TCLISH.search(text):
        return False
    if DEBUG_HOLE.search(text):
        return False
    if FORMAT_HOLE.search(text) and not any(c in text for c in "$[;\n"):
        return text.split(None, 1)[0] in TCL_HEADS
    return True


def crate_and_target(root: Path, file: Path) -> tuple[str, str, list[str]]:
    """The crate a file belongs to, its test target, and its file-derived module path."""
    parts = file.relative_to(root).parts
    crate_dir = file.parent
    while crate_dir != root.parent and not (crate_dir / "Cargo.toml").exists():
        crate_dir = crate_dir.parent
    crate = _crate_name(crate_dir)
    rel = file.relative_to(crate_dir).with_suffix("").parts if crate_dir in file.parents else parts
    if rel and rel[0] == "src":
        mods = [p for p in rel[1:] if p not in ("lib", "main", "mod")]
        if len(rel) > 2 and rel[1] == "bin":
            return crate, "bin/" + rel[2], [p for p in rel[3:] if p != "mod"]
        return crate, "lib", mods
    if rel and rel[0] in ("tests", "benches", "examples"):
        if len(rel) == 2:
            return crate, rel[1], []
        return crate, rel[1], [p for p in rel[2:] if p not in ("main", "mod")]
    return crate, "other", list(rel)


_CRATE_NAMES: dict[Path, str] = {}


def _crate_name(crate_dir: Path) -> str:
    if crate_dir not in _CRATE_NAMES:
        name = crate_dir.name
        manifest = crate_dir / "Cargo.toml"
        if manifest.exists():
            in_package = False
            for line in manifest.read_text(encoding="utf-8", errors="replace").splitlines():
                stripped = line.strip()
                if stripped.startswith("["):
                    in_package = stripped == "[package]"
                elif in_package:
                    m = re.match(r'name\s*=\s*"([^"]+)"', stripped)
                    if m:
                        name = m.group(1)
                        break
        _CRATE_NAMES[crate_dir] = name
    return _CRATE_NAMES[crate_dir]


def tests_in_file(root: Path, file: Path) -> list[Test]:
    text = file.read_text(encoding="utf-8", errors="replace")
    if "#[" not in text or "test" not in text:
        return []
    scan = scan_rust(text)
    sk = scan.skeleton
    crate, target, file_mods = crate_and_target(root, file)
    mod_spans = [(m.start(), _match_brace(sk, m.end() - 1), m.group(1)) for m in MOD.finditer(sk)]
    line_starts = [0]
    for idx, ch in enumerate(text):
        if ch == "\n":
            line_starts.append(idx + 1)

    def line_of(pos: int) -> int:
        lo, hi = 0, len(line_starts) - 1
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if line_starts[mid] <= pos:
                lo = mid
            else:
                hi = mid - 1
        return lo + 1

    found: list[Test] = []
    for attr in TEST_ATTR.finditer(sk):
        fn = FN.search(sk, attr.end())
        if not fn or sk[attr.end() : fn.start()].count(";") or sk[attr.end() : fn.start()].count("}"):
            continue
        open_at = sk.find("{", fn.end())
        if open_at == -1:
            continue
        close_at = _match_brace(sk, open_at)
        lead = line_starts[line_of(attr.start()) - 1]
        row = line_of(attr.start()) - 2
        while row >= 0:
            end = line_starts[row + 1] - 1
            if not text[line_starts[row] : end].strip().startswith(("//", "#[")):
                break
            lead = line_starts[row]
            row -= 1
        mods = file_mods + [name for start, end, name in mod_spans if start < attr.start() <= end]
        literals = [value for start, _end, value in scan.strings if open_at < start < close_at]
        remarks = " ".join(c for start, _end, c in scan.comments if lead <= start < close_at)
        body = text[open_at : close_at + 1]
        fixtures = tuple(sorted({_normalise_ws(v) for v in literals if is_fixture(v)}))
        cited = remarks + " " + " ".join(literals) + " " + fn.group(1)
        helpers = Counter(
            m.group(0)
            for m in IDENT.finditer(sk, open_at, close_at)
            if sk[m.end() : m.end() + 1] in ("(", "!") and m.group(0) not in NOT_HELPERS
        )
        name = fn.group(1)
        where = "/".join(mods + [name]).lower()
        found.append(
            Test(
                crate=crate,
                target=target,
                module="::".join(mods),
                name=name,
                file=str(file.relative_to(root)),
                line=line_of(fn.start()),
                lines=line_of(close_at) - line_of(fn.start()) + 1,
                fixtures=fixtures,
                codes=tuple(sorted(set(CODE.findall(body)))),
                helpers=tuple(h for h, _ in helpers.most_common(4)),
                oracle=bool(ORACLE.search(remarks)),
                issues=tuple(sorted(set(ISSUE.findall(cited)))),
                contract=bool(CONTRACT.search(remarks)),
                smells=tuple(s for s in SMELLS if s in where),
                body_hash=_digest(_normalise_ws(sk[open_at:close_at]) + "\x1e" + "\x1f".join(literals)),
            )
        )
    return found


def read_failing(paths: list[Path]) -> set[tuple[str, str]]:
    """Failing tests as (crate, module path) pairs from `<crate>[::target] <path>` lines."""
    failing: set[tuple[str, str]] = set()
    for path in paths:
        for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            fields = line.split()
            if len(fields) < 2:
                continue
            crate = fields[0].split("::")[0]
            crate = {"tcl-lsp-server-lib": "tcl-lsp-server"}.get(crate, crate)
            failing.add((crate, fields[-1]))
    return failing


def mark_failing(tests: list[Test], failing: set[tuple[str, str]]) -> int:
    """Mark tests named in the failing lists; the longest module suffix wins."""
    by_name: dict[tuple[str, str], list[Test]] = defaultdict(list)
    for test in tests:
        by_name[(test.crate, test.name)].append(test)
    matched = 0
    for crate, path in failing:
        name = path.rsplit("::", 1)[-1]
        candidates = by_name.get((crate, name), [])
        if not candidates:
            continue
        want = path.split("::")

        def score(test: Test) -> int:
            have = test.path.split("::")
            k = 0
            while k < min(len(have), len(want)) and have[-1 - k] == want[-1 - k]:
                k += 1
            return k

        best = max(score(t) for t in candidates)
        for test in candidates:
            if score(test) == best:
                test.failing = True
        matched += 1
    return matched


def collect(root: Path, subdirs: list[str]) -> list[Test]:
    tests: list[Test] = []
    for sub in subdirs:
        base = root / sub
        for file in sorted(base.rglob("*.rs")):
            if "/target/" in str(file) or "/vendor/" in str(file):
                continue
            tests.extend(tests_in_file(root, file))
    return tests


def _family(test: Test) -> str:
    head = test.module.split("::")[0] if test.module else "(root)"
    return f"{test.crate}/{test.target}/{head}"


def write_outputs(tests: list[Test], out: Path, unmatched: int) -> None:
    out.mkdir(parents=True, exist_ok=True)
    exact: dict[str, list[Test]] = defaultdict(list)
    overlap: dict[str, list[Test]] = defaultdict(list)
    by_fixture: dict[str, list[Test]] = defaultdict(list)
    for test in tests:
        exact[test.body_hash].append(test)
        if test.overlap_key:
            overlap[test.overlap_key].append(test)
        for fixture in test.fixtures:
            by_fixture[fixture].append(test)
    exact = {k: v for k, v in exact.items() if len(v) > 1}
    overlap = {k: v for k, v in overlap.items() if len(v) > 1}
    shared = {k: v for k, v in by_fixture.items() if len({t.file for t in v}) > 1}

    with (out / "inventory.tsv").open("w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh, delimiter="\t", lineterminator="\n")
        w.writerow(
            "crate target path file line lines failing class oracle issues contract codes "
            "fixtures helpers smells exact_cluster overlap_cluster".split()
        )
        for t in sorted(tests, key=lambda t: (t.crate, t.target, t.path)):
            w.writerow(
                [
                    t.crate, t.target, t.path, t.file, t.line, t.lines, int(t.failing), t.value_class,
                    int(t.oracle), ",".join(t.issues), int(t.contract), ",".join(t.codes),
                    len(t.fixtures), ",".join(t.helpers), ",".join(t.smells),
                    t.body_hash if t.body_hash in exact else "",
                    t.overlap_key if t.overlap_key in overlap else "",
                ]
            )

    def cluster_file(name: str, clusters: dict[str, list[Test]]) -> None:
        with (out / name).open("w", newline="", encoding="utf-8") as fh:
            w = csv.writer(fh, delimiter="\t", lineterminator="\n")
            w.writerow("cluster size failing files crate target path file line codes".split())
            order = sorted(clusters.items(), key=lambda kv: (-len(kv[1]), kv[0]))
            for key, members in order:
                bad = sum(t.failing for t in members)
                files = len({t.file for t in members})
                for t in sorted(members, key=lambda t: (t.file, t.line)):
                    w.writerow(
                        [key, len(members), bad, files, t.crate, t.target, t.path, t.file, t.line,
                         ",".join(t.codes)]
                    )

    cluster_file("clusters_exact.tsv", exact)
    cluster_file("clusters_overlap.tsv", overlap)

    with (out / "fixtures_shared.tsv").open("w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh, delimiter="\t", lineterminator="\n")
        w.writerow("tests files crates failing fixture".split())
        for fixture, members in sorted(shared.items(), key=lambda kv: (-len(kv[1]), kv[0])):
            w.writerow(
                [len(members), len({t.file for t in members}), len({t.crate for t in members}),
                 sum(t.failing for t in members), fixture[:300]]
            )

    (out / "summary.md").write_text(_summary(tests, exact, overlap, shared, unmatched), encoding="utf-8")


def _summary(tests, exact, overlap, shared, unmatched) -> str:
    total = len(tests)
    failing = [t for t in tests if t.failing]
    lines = [
        "# Test inventory",
        "",
        "Written by `tools/scrub_inventory.py`; do not edit by hand.",
        "",
        f"- Tests read: {total:,}; marked failing from the lists: {len(failing):,} "
        f"(list entries with no matching test: {unmatched:,}).",
        f"- Bodies that are the same text as another test's: {sum(len(v) for v in exact.values()):,} "
        f"tests in {len(exact):,} groups.",
        f"- Tests running the same Tcl sources through the same helpers against the same codes as another test: "
        f"{sum(len(v) for v in overlap.values()):,} in {len(overlap):,} groups "
        f"({sum(1 for v in overlap.values() if len({t.file for t in v}) > 1):,} groups span files).",
        f"- Tcl sources used from more than one file: {len(shared):,}.",
        "",
        "Classes: **A** runs Tcl source and cites tclsh, Jim or a release; **B** runs Tcl source and "
        "cites an issue, a contract or a KCS page; **C** runs Tcl source and cites nothing; **D** runs "
        "no Tcl source (it holds an internal structure or API).",
        "",
        "## By class",
        "",
        "| Class | Tests | Failing | Failing share of class |",
        "| --- | ---: | ---: | ---: |",
    ]
    classes = Counter(t.value_class for t in tests)
    bad = Counter(t.value_class for t in failing)
    for cls in sorted(classes):
        share = 100 * bad[cls] / classes[cls] if classes[cls] else 0
        lines.append(f"| {cls} | {classes[cls]:,} | {bad[cls]:,} | {share:.0f}% |")
    lines += ["", "## By crate", "", "| Crate | Tests | Failing | A | B | C | D | Failing A | Failing B | Failing C | Failing D |",
              "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"]
    crates = Counter(t.crate for t in tests)
    for crate, count in crates.most_common():
        mine = [t for t in tests if t.crate == crate]
        c = Counter(t.value_class[0] for t in mine)
        f = Counter(t.value_class[0] for t in mine if t.failing)
        lines.append(
            f"| `{crate}` | {count:,} | {sum(f.values()):,} | {c['A']:,} | {c['B']:,} | {c['C']:,} | "
            f"{c['D']:,} | {f['A']:,} | {f['B']:,} | {f['C']:,} | {f['D']:,} |"
        )
    lines += ["", "## Families with the most failing tests", "",
              "A family is a crate, a test target and the first module under it.", "",
              "| Family | Tests | Failing | Cites an oracle | Runs no Tcl source | In an overlap group |",
              "| --- | ---: | ---: | ---: | ---: | ---: |"]
    families: dict[str, list[Test]] = defaultdict(list)
    for t in tests:
        families[_family(t)].append(t)
    ranked = sorted(families.items(), key=lambda kv: -sum(t.failing for t in kv[1]))[:40]
    for fam, mine in ranked:
        n = len(mine)
        lines.append(
            f"| `{fam}` | {n:,} | {sum(t.failing for t in mine):,} | "
            f"{100 * sum(t.oracle for t in mine) / n:.0f}% | "
            f"{100 * sum(not t.fixtures for t in mine) / n:.0f}% | "
            f"{100 * sum(t.overlap_key in overlap for t in mine) / n:.0f}% |"
        )
    lines += ["", "## Failing tests and overlap", ""]
    fo = [t for t in failing if t.overlap_key in overlap]
    mixed = sum(
        1 for v in overlap.values() if any(t.failing for t in v) and any(not t.failing for t in v)
    )
    all_bad = sum(1 for v in overlap.values() if all(t.failing for t in v))
    lines += [
        f"- Failing tests that share sources, helpers and codes with another test: {len(fo):,}.",
        f"- Overlap groups where every member fails (one cause, several tests): {all_bad:,}.",
        f"- Overlap groups where some members pass and some fail (same input, different verdicts: "
        f"the first place to look for a wrong expectation): {mixed:,}.",
        "",
        "## Name smells among failing tests",
        "",
        "| Word in the module path or name | Tests | Failing |",
        "| --- | ---: | ---: |",
    ]
    for smell in SMELLS:
        mine = [t for t in tests if smell in t.smells]
        if mine:
            lines.append(f"| `{smell}` | {len(mine):,} | {sum(t.failing for t in mine):,} |")
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="repository root")
    parser.add_argument("--sub", action="append", default=None, help="subdirectory to read (repeatable)")
    parser.add_argument("--failing", type=Path, nargs="*", default=[], help="failing-test lists")
    parser.add_argument("--out", type=Path, required=True, help="output directory")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    tests = collect(root, args.sub or ["rust", "runtime/rust"])
    failing = read_failing(args.failing)
    matched = mark_failing(tests, failing)
    write_outputs(tests, args.out, len(failing) - matched)
    print(f"{len(tests)} tests, {sum(t.failing for t in tests)} failing, "
          f"{len(failing) - matched} list entries unmatched", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
