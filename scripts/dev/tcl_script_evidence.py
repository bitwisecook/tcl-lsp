#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Record what each Tcl-defined library procedure can touch.

Reads the procedures of a Tcl source tree (tcllib's `modules/`) and decides,
per procedure, whether its body is closed: every command it runs is a core
command whose C implementation is closed on every Tcl release the library
supports (`tcl_command_evidence.py`), a command that writes only the
procedure's own local variables, or another closed procedure of the tree;
every variable it reads or writes is local; every `expr` is braced; and it
completes only normally or with an error.

Anything the analysis cannot read — a substituted command word, `eval`,
`uplevel`, `upvar`, `global`, `variable`, a qualified variable, an unbraced
expression, `return -code` other than `ok` / `error` — leaves the procedure
open. One file is written:
`rust/tcl-registry/tests/data/command_evidence/tcllib.txt`, one line per
closed procedure: `NAME<TAB>closed<TAB>-`; a procedure not listed is open.

Usage: scripts/dev/tcl_script_evidence.py [tcllib-modules] [core-evidence-dir] [out]
"""

from __future__ import annotations

import pathlib
import re
import sys
from dataclasses import dataclass, field

# Commands whose only writes are to variables the procedure names in its own
# frame (an unqualified name), and whose reads are their operands.
LOCAL_WRITERS: dict[str, tuple[int, ...] | str] = {
    "set": (0,),
    "incr": (0,),
    "append": (0,),
    "lappend": (0,),
    "unset": "names",
    "lassign": "after1",
    "scan": "after2",
    "regexp": "regexp",
    "regsub": "regsub",
    "foreach": "foreach",
    "lmap": "foreach",
}

# Control commands whose braced script and expression words the analysis
# enters itself.
CONTROL = frozenset(
    "if while for foreach lmap catch try switch expr return error break continue".split()
)

COMPLETIONS = frozenset({"ok", "error", "0", "1"})


@dataclass
class Word:
    """One word: `braced` text, or parts of `('text'|'var'|'cmd', str)`."""

    braced: str | None = None
    parts: list[tuple[str, str]] = field(default_factory=list)
    expanded: bool = False

    @property
    def literal(self) -> str | None:
        if self.braced is not None:
            return self.braced
        if all(kind == "text" for kind, _ in self.parts):
            return "".join(text for _, text in self.parts)
        return None


class ParseError(Exception):
    pass


def _skip_braces(text: str, i: int) -> int:
    """Index after the brace that closes the one at `i`."""
    depth = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "\\":
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    raise ParseError("unbalanced brace")


def _skip_brackets(text: str, i: int) -> int:
    """Index after the `]` closing the command substitution opened at `i`."""
    j = i + 1
    _, j = parse_script(text, j, in_brackets=True)
    if j >= len(text) or text[j] != "]":
        raise ParseError("unbalanced bracket")
    return j + 1


VARNAME = re.compile(r"(?:::)?[A-Za-z0-9_]+(?:::+[A-Za-z0-9_]+)*")


def _variable(text: str, i: int, parts: list[tuple[str, str]]) -> int:
    """Read the substitution at `text[i] == '$'`; append it and return the end."""
    if i + 1 < len(text) and text[i + 1] == "{":
        end = text.index("}", i + 2)
        parts.append(("var", text[i + 2 : end]))
        return end + 1
    m = VARNAME.match(text, i + 1)
    if not m:
        parts.append(("text", "$"))
        return i + 1
    name = m.group(0)
    j = m.end()
    if j < len(text) and text[j] == "(":
        depth = 0
        k = j
        while k < len(text):
            if text[k] == "\\":
                k += 2
                continue
            if text[k] == "[":
                k = _skip_brackets(text, k)
                continue
            if text[k] == "(":
                depth += 1
            elif text[k] == ")":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        index = text[j + 1 : k]
        parts.append(("var", name))
        for kind, value in parse_word_parts(index):
            if kind != "text":
                parts.append((kind, value))
        return k + 1
    parts.append(("var", name))
    return j


def parse_word_parts(text: str) -> list[tuple[str, str]]:
    """The substitutions of a whole string, as a quoted word's would be."""
    parts: list[tuple[str, str]] = []
    i = 0
    while i < len(text):
        c = text[i]
        if c == "\\":
            parts.append(("text", text[i : i + 2]))
            i += 2
        elif c == "$":
            i = _variable(text, i, parts)
        elif c == "[":
            end = _skip_brackets(text, i)
            parts.append(("cmd", text[i + 1 : end - 1]))
            i = end
        else:
            j = i
            while j < len(text) and text[j] not in "\\$[":
                j += 1
            parts.append(("text", text[i:j]))
            i = j
    return parts


def parse_script(
    text: str, i: int = 0, in_brackets: bool = False
) -> tuple[list[list[Word]], int]:
    """Commands of `text` from `i`; stops at an unmatched `]` when nested."""
    commands: list[list[Word]] = []
    n = len(text)
    words: list[Word] = []
    while i < n:
        c = text[i]
        if c == "\\" and i + 1 < n and text[i + 1] == "\n":
            i += 2
            continue
        if c in " \t\r":
            i += 1
            continue
        if c in "\n;":
            if words:
                commands.append(words)
                words = []
            i += 1
            continue
        if c == "]" and in_brackets:
            break
        if c == "#" and not words:
            while i < n and text[i] != "\n":
                i += 2 if text[i] == "\\" else 1
            continue
        word = Word()
        if text.startswith("{*}", i) and i + 3 < n and text[i + 3] not in " \t\n;":
            word.expanded = True
            i += 3
            c = text[i]
        if c == "{":
            end = _skip_braces(text, i)
            word.braced = text[i + 1 : end - 1]
            i = end
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                if text[j] == "\\":
                    j += 2
                elif text[j] == "[":
                    j = _skip_brackets(text, j)
                else:
                    j += 1
            word.parts = parse_word_parts(text[i + 1 : j])
            i = j + 1
        else:
            j = i
            while (
                j < n
                and text[j] not in " \t\r\n;"
                and not (in_brackets and text[j] == "]")
            ):
                if text[j] == "\\":
                    j += 2
                elif text[j] == "[":
                    j = _skip_brackets(text, j)
                elif text[j] == "$" and j + 1 < n and text[j + 1] == "{":
                    j = text.index("}", j) + 1
                else:
                    j += 1
            word.parts = parse_word_parts(text[i:j])
            i = j
        words.append(word)
    if words:
        commands.append(words)
    return commands, i


@dataclass
class Proc:
    name: str
    args: list[str]
    body: str
    path: str


def namespace_procs(
    text: str, namespace: str, path: str, procs: dict[str, Proc]
) -> None:
    """Collect `proc` definitions, entering `namespace eval NS {…}` bodies."""
    try:
        commands, _ = parse_script(text)
    except (ParseError, ValueError):
        return
    for words in commands:
        head = words[0].literal
        if head == "proc" and len(words) == 4:
            name, args, body = (w.literal for w in words[1:])
            if name is None or args is None or body is None:
                continue
            qualified = (
                name.lstrip(":")
                if name.startswith("::")
                else (f"{namespace}::{name}" if namespace else name)
            )
            try:
                arglist, _ = parse_script(args)
            except (ParseError, ValueError):
                continue
            names = []
            for item in arglist[0] if arglist else []:
                literal = item.literal or ""
                names.append(literal.split()[0] if literal.split() else literal)
            procs.setdefault(qualified, Proc(qualified, names, body, path))
        elif head == "namespace" and len(words) >= 4 and words[1].literal == "eval":
            ns = words[2].literal
            body = words[3].literal
            if ns is None or body is None or len(words) != 4:
                continue
            ns = (
                ns.lstrip(":")
                if ns.startswith("::")
                else (f"{namespace}::{ns}" if namespace else ns)
            )
            namespace_procs(body, ns, path, procs)


@dataclass
class Verdict:
    closed: bool
    reason: str = ""


class Analyser:
    def __init__(self, procs: dict[str, Proc], closed_core: set[str]):
        self.procs = procs
        self.closed_core = closed_core
        self.verdicts: dict[str, Verdict] = {}
        self.active: set[str] = set()

    def proc(self, name: str) -> Verdict:
        if name in self.verdicts:
            return self.verdicts[name]
        if name in self.active:
            # A recursive call adds nothing the other paths do not.
            return Verdict(True)
        proc = self.procs[name]
        self.active.add(name)
        try:
            verdict = self.script(proc.body, set(proc.args), proc)
        except (ParseError, ValueError) as error:
            verdict = Verdict(False, f"unreadable: {error}")
        self.active.discard(name)
        self.verdicts[name] = verdict
        return verdict

    def resolve(self, word: str, proc: Proc) -> str | None:
        """The tree procedure a literal command word names from `proc`."""
        if word.startswith("::"):
            target = word[2:]
            return target if target in self.procs else None
        namespace = proc.name.rpartition("::")[0]
        while True:
            candidate = f"{namespace}::{word}" if namespace else word
            if candidate in self.procs:
                return candidate
            if not namespace:
                return None
            namespace = namespace.rpartition("::")[0] if "::" in namespace else ""

    def script(self, text: str, locals_: set[str], proc: Proc) -> Verdict:
        commands, _ = parse_script(text)
        for words in commands:
            verdict = self.command(words, locals_, proc)
            if not verdict.closed:
                return verdict
        return Verdict(True)

    def word(self, word: Word, locals_: set[str], proc: Proc) -> Verdict:
        if word.expanded:
            return Verdict(False, "expanded word")
        for kind, value in word.parts:
            if kind == "var" and not self.local(value):
                return Verdict(False, f"reads ${value}")
            if kind == "cmd":
                verdict = self.script(value, locals_, proc)
                if not verdict.closed:
                    return verdict
        return Verdict(True)

    @staticmethod
    def local(name: str) -> bool:
        return "::" not in name

    def expression(self, text: str, locals_: set[str], proc: Proc) -> Verdict:
        for kind, value in parse_word_parts(text):
            if kind == "var" and not self.local(value):
                return Verdict(False, f"reads ${value}")
            if kind == "cmd":
                verdict = self.script(value, locals_, proc)
                if not verdict.closed:
                    return verdict
        for function in re.findall(
            r"\b([A-Za-z_]\w*)\s*\(", re.sub(r'"[^"]*"', "", text)
        ):
            if f"tcl::mathfunc::{function}" not in self.closed_core:
                return Verdict(False, f"math function {function}()")
        return Verdict(True)

    def command(self, words: list[Word], locals_: set[str], proc: Proc) -> Verdict:
        head = words[0].literal
        if head is None or words[0].expanded:
            return Verdict(False, "substituted command word")
        args = words[1:]
        for arg in args:
            verdict = self.word(arg, locals_, proc)
            if not verdict.closed:
                return verdict
        literal = [arg.literal for arg in args]
        if head in LOCAL_WRITERS or head in CONTROL:
            return self.builtin(head, args, literal, locals_, proc)
        target = self.resolve(head, proc)
        if target is not None:
            verdict = self.proc(target)
            return (
                verdict
                if verdict.closed
                else Verdict(False, f"calls {target}: {verdict.reason}")
            )
        name = head.lstrip(":")
        if name in self.closed_core:
            return Verdict(True)
        if (
            literal
            and literal[0] is not None
            and f"{name} {literal[0]}" in self.closed_core
        ):
            return Verdict(True)
        return Verdict(
            False, f"runs {name}{' ' + literal[0] if literal and literal[0] else ''}"
        )

    def builtin(self, head, args, literal, locals_, proc) -> Verdict:
        def var_names(names):
            for name in names:
                if name is None or not self.local(name):
                    return Verdict(False, f"{head} writes a non-local variable")
                locals_.add(name)
            return Verdict(True)

        def body(index):
            if (
                index >= len(literal)
                or literal[index] is None
                or args[index].braced is None
            ):
                return Verdict(False, f"{head} body is not a braced literal")
            return self.script(literal[index], locals_, proc)

        def expr(index):
            if index >= len(literal) or args[index].braced is None:
                return Verdict(False, f"{head} expression is not braced")
            return self.expression(literal[index], locals_, proc)

        shape = LOCAL_WRITERS.get(head)
        if isinstance(shape, tuple):
            return var_names([literal[i] if i < len(literal) else None for i in shape])
        if shape == "names":
            names = [lit for lit in literal if lit not in ("-nocomplain", "--")]
            return var_names(names)
        if shape == "after1":
            return var_names(literal[1:])
        if shape == "after2":
            return var_names(literal[2:])
        if shape in ("regexp", "regsub"):
            i = 0
            while i < len(literal) and literal[i] and literal[i].startswith("-"):
                if literal[i] == "--":
                    i += 1
                    break
                if literal[i] in ("-start",):
                    i += 1
                i += 1
            rest = literal[i:]
            if shape == "regexp":
                return var_names(rest[2:])
            return var_names(rest[3:4])
        if shape == "foreach":
            for i in range(0, len(literal) - 1, 2):
                if literal[i] is None:
                    return Verdict(False, f"{head} names its variables dynamically")
                verdict = var_names(literal[i].split())
                if not verdict.closed:
                    return verdict
            return body(len(literal) - 1)
        if head == "if":
            i = 0
            while i < len(literal):
                verdict = expr(i)
                if not verdict.closed:
                    return verdict
                i += 1
                if i < len(literal) and literal[i] == "then":
                    i += 1
                verdict = body(i)
                if not verdict.closed:
                    return verdict
                i += 1
                if i < len(literal) and literal[i] == "elseif":
                    i += 1
                    continue
                if i < len(literal) and literal[i] == "else":
                    i += 1
                    return body(i)
                if i < len(literal):
                    return body(i)
                break
            return Verdict(True)
        if head == "while":
            return expr(0) if not expr(0).closed else body(1)
        if head == "for":
            for check in (
                lambda: body(0),
                lambda: expr(1),
                lambda: body(2),
                lambda: body(3),
            ):
                verdict = check()
                if not verdict.closed:
                    return verdict
            return Verdict(True)
        if head == "expr":
            return (
                expr(0)
                if len(literal) == 1
                else Verdict(False, "expr of several words")
            )
        if head == "catch":
            verdict = body(0)
            if verdict.closed and len(literal) > 1:
                verdict = var_names(literal[1:3])
            return verdict
        if head == "return":
            i = 0
            while i + 1 < len(literal) and literal[i] and literal[i].startswith("-"):
                if literal[i] == "-code" and literal[i + 1] not in COMPLETIONS:
                    return Verdict(False, f"return -code {literal[i + 1]}")
                if literal[i] == "-level":
                    return Verdict(False, "return -level")
                i += 2
            return Verdict(True)
        if head in ("error",):
            return Verdict(True)
        if head in ("break", "continue"):
            return Verdict(False, f"{head} completes abnormally")
        if head == "switch":
            i = 0
            while i < len(literal) and literal[i] and literal[i].startswith("-"):
                if literal[i] == "--":
                    i += 1
                    break
                i += 1
            rest = literal[i + 1 :]
            if len(rest) == 1 and args[i + 1].braced is not None:
                clauses, _ = parse_script(rest[0])
                bodies = [w.literal for words in clauses for w in words][1::2]
            else:
                bodies = rest[1::2]
            for text in bodies:
                if text is None:
                    return Verdict(False, "switch body is not a literal")
                if text == "-":
                    continue
                verdict = self.script(text, locals_, proc)
                if not verdict.closed:
                    return verdict
            return Verdict(True)
        return Verdict(False, f"{head} is not read")


def main(argv: list[str]) -> int:
    root = pathlib.Path(__file__).resolve().parents[2]
    modules = (
        pathlib.Path(argv[1]) if len(argv) > 1 else root / "tmp/tcllib-2.0/modules"
    )
    core = (
        pathlib.Path(argv[2])
        if len(argv) > 2
        else root / "rust/tcl-registry/tests/data/command_evidence"
    )
    out = pathlib.Path(argv[3]) if len(argv) > 3 else core / "tcllib.txt"
    # tcllib 2.0 supports Tcl 8.5 to 9.x: a core command counts only when it
    # is closed on every one of those releases.
    closed_core: set[str] | None = None
    for release in ("8.5", "8.6", "9.0", "9.1"):
        names = {
            line.split("\t")[0]
            for line in (core / f"tcl{release}.txt").read_text().splitlines()
            if not line.startswith("#") and line.split("\t")[1] == "closed"
        }
        closed_core = names if closed_core is None else closed_core & names
    procs: dict[str, Proc] = {}
    for path in sorted(modules.rglob("*.tcl")):
        if "/tests/" in str(path) or path.name.endswith(".test"):
            continue
        namespace_procs(
            path.read_text(encoding="utf-8", errors="replace"),
            "",
            str(path.relative_to(modules)),
            procs,
        )
    analyser = Analyser(procs, closed_core or set())
    lines = []
    opened = 0
    for name in sorted(procs):
        verdict = analyser.proc(name)
        if verdict.closed:
            lines.append(f"{name}\tclosed\t-")
        else:
            opened += 1
    header = (
        "# tcllib 2.0 modules/*.tcl, by scripts/dev/tcl_script_evidence.py\n"
        f"# the {len(lines)} closed procedures of {len(lines) + opened}; any other is open\n"
        "# NAME\tVERDICT\tREASON\n"
    )
    out.write_text(header + "\n".join(lines) + "\n")
    print(f"tcllib: {len(lines) + opened} procedures, {len(lines)} closed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
