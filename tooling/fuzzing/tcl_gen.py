"""Grammar-aware Tcl script generator for differential fuzzing.

Generates structurally valid Tcl scripts with matching brackets, braces,
and quotes.  The generator is parameterised by a random seed so that any
failing case is reproducible.

Design goals:
  - Every emitted script is syntactically valid Tcl (balanced delimiters).
  - Cover a wide surface: variables, exprs, control flow, procs, lists,
    dicts, strings, namespaces, error handling.
  - Avoid I/O, file, socket, and interp commands — keep scripts pure.
  - Depth/width limits prevent runaway generation.
"""

from __future__ import annotations

import random
from collections.abc import Callable
from dataclasses import dataclass


@dataclass
class GenConfig:
    """Tunables for the generator."""

    max_depth: int = 4
    max_stmts: int = 12
    max_list_len: int = 6
    max_expr_depth: int = 3
    max_proc_params: int = 4
    max_string_len: int = 20
    # Probability weights for statement types at each depth.
    # Higher depth → more leaves, fewer nested structures.
    leaf_bias: float = 0.3
    # Fraction of generated scripts that are intentionally malformed
    # to exercise error-recovery and error-handling paths.
    bad_input_pct: float = 0.05


# Identifiers

_VAR_NAMES = [
    "x",
    "y",
    "z",
    "a",
    "b",
    "result",
    "tmp",
    "val",
    "idx",
    "count",
    "item",
    "data",
    "key",
    "buf",
    "acc",
    "flag",
    "msg",
    "n",
    "i",
    "j",
]

_PROC_NAMES = [
    "myproc",
    "helper",
    "worker",
    "compute",
    "transform",
    "combine",
    "build",
    "check",
    "process",
    "handle",
]

_NS_NAMES = ["foo", "bar", "baz", "util", "inner"]


class TclGenerator:
    """Generate random structurally-valid Tcl scripts."""

    def __init__(self, seed: int | None = None, config: GenConfig | None = None) -> None:
        self.rng = random.Random(seed)
        self.cfg = config or GenConfig()
        self._vars_in_scope: list[str] = []
        self._procs_defined: list[str] = []
        self._depth = 0

    # Public API

    def generate(self) -> str:
        """Return a complete Tcl script.

        With probability ``cfg.bad_input_pct`` the script is intentionally
        corrupted so that error-handling / recovery paths get exercised.
        """
        self._vars_in_scope = []
        self._procs_defined = []
        self._depth = 0
        lines = self._block(top_level=True)
        script = "\n".join(lines) + "\n"
        if self.cfg.bad_input_pct > 0 and self.rng.random() < self.cfg.bad_input_pct:
            script = corrupt_script(script, self.rng)
        return script

    # Block / statement generators

    def _block(self, *, top_level: bool = False) -> list[str]:
        n = self.rng.randint(2, self.cfg.max_stmts)
        lines: list[str] = []
        for _ in range(n):
            lines.extend(self._statement(top_level=top_level))
        return lines

    def _statement(self, *, top_level: bool = False) -> list[str]:
        """Pick a random statement type appropriate for current depth."""
        # Build weighted choices — bias toward leaves at depth
        depth_ratio = self._depth / max(self.cfg.max_depth, 1)
        leaf_weight = self.cfg.leaf_bias + depth_ratio * (1 - self.cfg.leaf_bias)
        nest_weight = 1 - leaf_weight

        _Choice = tuple[float, Callable[[], list[str]]]

        leaves: list[_Choice] = [
            (leaf_weight * 3, self._set_stmt),
            (leaf_weight * 2, self._expr_stmt),
            (leaf_weight * 1.5, self._append_stmt),
            (leaf_weight * 1, self._incr_stmt),
            (leaf_weight * 1, self._string_stmt),
            (leaf_weight * 1, self._list_stmt),
            (leaf_weight * 0.5, self._lassign_stmt),
            (leaf_weight * 0.5, self._regexp_stmt),
        ]

        nesting: list[_Choice] = [
            (nest_weight * 2, self._if_stmt),
            (nest_weight * 1.5, self._for_stmt),
            (nest_weight * 1.5, self._foreach_stmt),
            (nest_weight * 1, self._while_stmt),
            (nest_weight * 0.8, self._switch_stmt),
            (nest_weight * 0.5, self._catch_stmt),
        ]

        if top_level and self._depth == 0:
            nesting.append((nest_weight * 1.5, self._proc_stmt))
            nesting.append((nest_weight * 0.5, self._namespace_stmt))

        if self._depth >= self.cfg.max_depth:
            choices = list(leaves)
        else:
            choices = leaves + nesting

        # Add proc call if procs are defined
        if self._procs_defined:
            choices.append((leaf_weight * 1, self._proc_call_stmt))

        funcs = [f for _, f in choices]
        weights = [w for w, _ in choices]
        fn = self.rng.choices(funcs, weights=weights, k=1)[0]
        return fn()

    # Leaf statements

    def _set_stmt(self) -> list[str]:
        var = self._pick_or_new_var()
        val = self._value()
        return [f"set {var} {val}"]

    def _expr_stmt(self) -> list[str]:
        return [f"expr {{{self._expr()}}}"]

    def _append_stmt(self) -> list[str]:
        var = self._pick_or_new_var()
        val = self._simple_value()
        return [f"append {var} {val}"]

    def _incr_stmt(self) -> list[str]:
        var = self._pick_or_new_var()
        # Ensure var has a numeric value
        init = [f"set {var} {self.rng.randint(0, 100)}"]
        amt = self.rng.randint(-5, 10)
        return init + [f"incr {var} {amt}"]

    def _string_stmt(self) -> list[str]:
        subcmds = [
            "length",
            "index",
            "range",
            "tolower",
            "toupper",
            "trim",
            "trimleft",
            "trimright",
            "reverse",
            "repeat",
            "map",
            "first",
            "last",
        ]
        sub = self.rng.choice(subcmds)
        s = self._quoted_string()
        if sub == "length":
            return [f"string length {s}"]
        elif sub == "index":
            idx = self.rng.randint(0, 5)
            return [f"string index {s} {idx}"]
        elif sub == "range":
            a, b = sorted(self.rng.sample(range(8), 2))
            return [f"string range {s} {a} {b}"]
        elif sub in ("tolower", "toupper", "trim", "trimleft", "trimright", "reverse"):
            return [f"string {sub} {s}"]
        elif sub == "repeat":
            n = self.rng.randint(0, 4)
            return [f"string repeat {s} {n}"]
        elif sub == "map":
            return [f"string map {{a b c d}} {s}"]
        elif sub == "first":
            return [f'string first "a" {s}']
        elif sub == "last":
            return [f'string last "a" {s}']
        return [f"string length {s}"]

    def _list_stmt(self) -> list[str]:
        subcmds = [
            "list",
            "llength",
            "lindex",
            "lrange",
            "lappend",
            "lsort",
            "lreverse",
            "concat",
            "join",
            "lsearch",
            "lrepeat",
        ]
        sub = self.rng.choice(subcmds)
        lst = self._list_literal()
        if sub == "list":
            return [f"list {' '.join(self._simple_values(self.rng.randint(1, 4)))}"]
        elif sub == "llength":
            return [f"llength {lst}"]
        elif sub == "lindex":
            return [f"lindex {lst} {self.rng.randint(0, 3)}"]
        elif sub == "lrange":
            a, b = sorted(self.rng.sample(range(5), 2))
            return [f"lrange {lst} {a} {b}"]
        elif sub == "lappend":
            var = self._pick_or_new_var()
            return [f"set {var} {lst}", f"lappend {var} {self._simple_value()}"]
        elif sub in ("lsort", "lreverse"):
            return [f"{sub} {lst}"]
        elif sub == "concat":
            lst2 = self._list_literal()
            return [f"concat {lst} {lst2}"]
        elif sub == "join":
            sep = self.rng.choice(['" "', '","', '"-"', '""'])
            return [f"join {lst} {sep}"]
        elif sub == "lsearch":
            return [f"lsearch {lst} {self._simple_value()}"]
        elif sub == "lrepeat":
            n = self.rng.randint(1, 3)
            return [f"lrepeat {n} {self._simple_value()}"]
        return [f"llength {lst}"]

    def _lassign_stmt(self) -> list[str]:
        lst = self._list_literal()
        nvars = self.rng.randint(1, 3)
        vars_ = [self._pick_or_new_var() for _ in range(nvars)]
        return [f"lassign {lst} {' '.join(vars_)}"]

    def _regexp_stmt(self) -> list[str]:
        # Simple patterns only — avoid pathological regexes
        patterns = [r'"[a-z]+"', r'"\\d+"', r'"."', r'"^[a-z]"', r'"[0-9]"']
        pat = self.rng.choice(patterns)
        s = self._quoted_string()
        return [f"regexp {pat} {s}"]

    def _proc_call_stmt(self) -> list[str]:
        name = self.rng.choice(self._procs_defined)
        # We don't track arity, so pass a few simple args
        nargs = self.rng.randint(0, 3)
        args = " ".join(self._simple_values(nargs))
        return [f"catch {{{name} {args}}}"]

    # Nesting statements

    def _if_stmt(self) -> list[str]:
        self._depth += 1
        cond = self._condition()
        body = self._nested_block()
        lines = [f"if {{{cond}}} {{", *["    " + ln for ln in body], "}"]
        if self.rng.random() < 0.4:
            else_body = self._nested_block()
            lines[-1] = "} else {"
            lines.extend(["    " + ln for ln in else_body])
            lines.append("}")
        if self.rng.random() < 0.2:
            cond2 = self._condition()
            elseif_body = self._nested_block()
            # Replace the final "}" with an elseif clause
            lines[-1] = f"}} elseif {{{cond2}}} {{"
            lines.extend(["    " + ln for ln in elseif_body])
            lines.append("}")
        self._depth -= 1
        return lines

    def _while_stmt(self) -> list[str]:
        self._depth += 1
        var = self._pick_or_new_var()
        limit = self.rng.randint(1, 5)
        body = self._nested_block()
        lines = [
            f"set {var} 0",
            f"while {{${var} < {limit}}} {{",
            *["    " + ln for ln in body],
            f"    incr {var}",
            "}",
        ]
        self._depth -= 1
        return lines

    def _for_stmt(self) -> list[str]:
        self._depth += 1
        var = self._pick_or_new_var()
        limit = self.rng.randint(1, 6)
        body = self._nested_block()
        lines = [
            f"for {{set {var} 0}} {{${var} < {limit}}} {{incr {var}}} {{",
            *["    " + ln for ln in body],
            "}",
        ]
        self._depth -= 1
        return lines

    def _foreach_stmt(self) -> list[str]:
        self._depth += 1
        var = self._pick_or_new_var()
        lst = self._list_literal()
        body = self._nested_block()
        lines = [
            f"foreach {var} {lst} {{",
            *["    " + ln for ln in body],
            "}",
        ]
        self._depth -= 1
        return lines

    def _switch_stmt(self) -> list[str]:
        self._depth += 1
        val = self._simple_value()
        ncases = self.rng.randint(1, 4)
        lines = [f"switch -- {val} {{"]
        for _ in range(ncases):
            pattern = self._simple_value()
            case_body = self._nested_block()
            lines.append(f"    {pattern} {{")
            lines.extend(["        " + ln for ln in case_body])
            lines.append("    }")
        # default case
        default_body = self._nested_block()
        lines.append("    default {")
        lines.extend(["        " + ln for ln in default_body])
        lines.append("    }")
        lines.append("}")
        self._depth -= 1
        return lines

    def _catch_stmt(self) -> list[str]:
        self._depth += 1
        body = self._nested_block()
        var = self._pick_or_new_var()
        lines = ["catch {", *["    " + ln for ln in body], f"}} {var}"]
        self._depth -= 1
        return lines

    def _proc_stmt(self) -> list[str]:
        self._depth += 1
        name = self.rng.choice(_PROC_NAMES)
        nparams = self.rng.randint(0, self.cfg.max_proc_params)
        params = self.rng.sample(_VAR_NAMES, min(nparams, len(_VAR_NAMES)))
        # Optionally add defaults
        param_list_parts: list[str] = []
        for p in params:
            if self.rng.random() < 0.3:
                param_list_parts.append(f"{{{p} {self._literal()}}}")
            else:
                param_list_parts.append(p)
        # Optionally add args
        if self.rng.random() < 0.2:
            param_list_parts.append("args")
        param_str = " ".join(param_list_parts)
        body = self._nested_block()
        # Add return statement
        ret_val = self._simple_value()
        body.append(f"return {ret_val}")
        lines = [
            f"proc {name} {{{param_str}}} {{",
            *["    " + ln for ln in body],
            "}",
        ]
        self._procs_defined.append(name)
        self._depth -= 1
        return lines

    def _namespace_stmt(self) -> list[str]:
        self._depth += 1
        ns = self.rng.choice(_NS_NAMES)
        body = self._nested_block()
        lines = [
            f"namespace eval {ns} {{",
            *["    " + ln for ln in body],
            "}",
        ]
        self._depth -= 1
        return lines

    # Expressions

    def _expr(self, depth: int = 0) -> str:
        if depth >= self.cfg.max_expr_depth:
            return self._expr_atom()
        choice = self.rng.random()
        if choice < 0.3:
            return self._expr_atom()
        elif choice < 0.65:
            op = self.rng.choice(
                [
                    "+",
                    "-",
                    "*",
                    "/",
                    "%",
                    "**",
                    "==",
                    "!=",
                    "<",
                    ">",
                    "<=",
                    ">=",
                    "&&",
                    "||",
                    "&",
                    "|",
                    "^",
                    "<<",
                    ">>",
                ]
            )
            left = self._expr(depth + 1)
            right = self._expr(depth + 1)
            # Avoid division by zero: wrap divisor
            if op in ("/", "%"):
                right = f"max(1, abs({right}))"
            return f"({left} {op} {right})"
        elif choice < 0.8:
            op = self.rng.choice(["!", "-", "~"])
            operand = self._expr(depth + 1)
            return f"({op}{operand})"
        elif choice < 0.9:
            cond = self._expr(depth + 1)
            then = self._expr(depth + 1)
            else_ = self._expr(depth + 1)
            return f"({cond} ? {then} : {else_})"
        else:
            func = self.rng.choice(["abs", "int", "double", "round", "wide", "max", "min"])
            if func in ("max", "min"):
                a = self._expr(depth + 1)
                b = self._expr(depth + 1)
                return f"{func}({a}, {b})"
            arg = self._expr(depth + 1)
            return f"{func}({arg})"

    def _expr_atom(self) -> str:
        choice = self.rng.random()
        if choice < 0.5:
            return str(self.rng.randint(-100, 100))
        elif choice < 0.7:
            return f"{self.rng.uniform(-100, 100):.2f}"
        elif choice < 0.9 and self._vars_in_scope:
            var = self.rng.choice(self._vars_in_scope)
            return f"${var}"
        else:
            return str(self.rng.randint(0, 50))

    def _condition(self) -> str:
        """Generate a boolean expression for if/while conditions."""
        if self._vars_in_scope and self.rng.random() < 0.5:
            var = self.rng.choice(self._vars_in_scope)
            op = self.rng.choice(["<", ">", "<=", ">=", "==", "!="])
            val = self.rng.randint(-10, 20)
            return f"${var} {op} {val}"
        return self._expr(depth=1)

    # Values

    def _value(self) -> str:
        choice = self.rng.random()
        if choice < 0.3:
            return self._literal()
        elif choice < 0.5:
            return self._quoted_string()
        elif choice < 0.7:
            return self._list_literal()
        elif choice < 0.85:
            return f"[expr {{{self._expr()}}}]"
        else:
            return self._literal()

    def _simple_value(self) -> str:
        choice = self.rng.random()
        if choice < 0.4:
            return self._literal()
        elif choice < 0.7:
            return self._quoted_string()
        else:
            return str(self.rng.randint(-100, 100))

    def _simple_values(self, n: int) -> list[str]:
        return [self._simple_value() for _ in range(n)]

    def _literal(self) -> str:
        choice = self.rng.random()
        if choice < 0.4:
            return str(self.rng.randint(-1000, 1000))
        elif choice < 0.6:
            return f"{self.rng.uniform(-100, 100):.2f}"
        elif choice < 0.8:
            length = self.rng.randint(1, 8)
            chars = [self.rng.choice("abcdefghijklmnopqrstuvwxyz") for _ in range(length)]
            return "".join(chars)
        else:
            return self.rng.choice(["true", "false", "yes", "no", "0", "1"])

    def _quoted_string(self) -> str:
        length = self.rng.randint(0, self.cfg.max_string_len)
        # Safe chars only — no unbalanced delimiters
        safe_chars = "abcdefghijklmnopqrstuvwxyz0123456789 .,;:!?-_+=@#"
        chars = [self.rng.choice(safe_chars) for _ in range(length)]
        return '"' + "".join(chars) + '"'

    def _list_literal(self) -> str:
        n = self.rng.randint(0, self.cfg.max_list_len)
        elems = [self._literal() for _ in range(n)]
        return "{" + " ".join(elems) + "}"

    # Variable management

    def _pick_or_new_var(self) -> str:
        if self._vars_in_scope and self.rng.random() < 0.6:
            return self.rng.choice(self._vars_in_scope)
        var = self.rng.choice(_VAR_NAMES)
        if var not in self._vars_in_scope:
            self._vars_in_scope.append(var)
        return var

    def _nested_block(self) -> list[str]:
        n = self.rng.randint(1, max(2, self.cfg.max_stmts // 2))
        lines: list[str] = []
        for _ in range(n):
            lines.extend(self._statement())
        return lines


# Corruption strategies
#
# These intentionally produce *invalid* Tcl to exercise error paths
# in the parser, lexer, and VM.  Each takes a valid script and returns
# a mangled version.


def corrupt_script(script: str, rng: random.Random) -> str:
    """Apply a random corruption to *script*, producing invalid Tcl."""
    strategies = [
        _corrupt_unbalanced_brace,
        _corrupt_unbalanced_quote,
        _corrupt_truncate,
        _corrupt_inject_garbage,
        _corrupt_null_byte,
        _corrupt_backslash_eol,
        _corrupt_duplicate_delimiter,
        _corrupt_empty_command,
    ]
    strategy = rng.choice(strategies)
    return strategy(script, rng)


def _corrupt_unbalanced_brace(script: str, rng: random.Random) -> str:
    """Remove a random closing brace or add an extra opening one."""
    if rng.random() < 0.5:
        # Remove a random '}'
        positions = [i for i, c in enumerate(script) if c == "}"]
        if positions:
            idx = rng.choice(positions)
            return script[:idx] + script[idx + 1 :]
    else:
        # Insert an extra '{'
        pos = rng.randint(0, len(script))
        return script[:pos] + "{" + script[pos:]
    return script


def _corrupt_unbalanced_quote(script: str, rng: random.Random) -> str:
    """Insert a lone double-quote at a random position."""
    pos = rng.randint(0, len(script))
    return script[:pos] + '"' + script[pos:]


def _corrupt_truncate(script: str, rng: random.Random) -> str:
    """Truncate the script at a random point (at least 10 chars kept)."""
    if len(script) <= 10:
        return script
    cut = rng.randint(10, len(script) - 1)
    return script[:cut]


def _corrupt_inject_garbage(script: str, rng: random.Random) -> str:
    """Inject random non-ASCII or control characters."""
    garbage_chars = "\x00\x01\x02\x07\x1b\xff\xfe"
    length = rng.randint(1, 5)
    garbage = "".join(rng.choice(garbage_chars) for _ in range(length))
    pos = rng.randint(0, len(script))
    return script[:pos] + garbage + script[pos:]


def _corrupt_null_byte(script: str, rng: random.Random) -> str:
    """Insert a null byte in the middle of a line."""
    lines = script.split("\n")
    if not lines:
        return script
    idx = rng.randint(0, len(lines) - 1)
    line = lines[idx]
    if line:
        pos = rng.randint(0, len(line))
        lines[idx] = line[:pos] + "\x00" + line[pos:]
    else:
        lines[idx] = "\x00"
    return "\n".join(lines)


def _corrupt_backslash_eol(script: str, rng: random.Random) -> str:
    """Insert a bare backslash at end-of-line to break continuation."""
    lines = script.split("\n")
    if len(lines) < 2:
        return script
    idx = rng.randint(0, len(lines) - 2)
    lines[idx] = lines[idx] + "\\"
    return "\n".join(lines)


def _corrupt_duplicate_delimiter(script: str, rng: random.Random) -> str:
    """Double a random delimiter (brace, bracket, quote)."""
    delims = [i for i, c in enumerate(script) if c in '{}[]"']
    if not delims:
        return script
    idx = rng.choice(delims)
    return script[:idx] + script[idx] + script[idx:]


def _corrupt_empty_command(script: str, rng: random.Random) -> str:
    """Replace a random line with a syntactically broken fragment."""
    fragments = [
        "[",
        "]]]",
        "{{{{",
        "set",
        "expr {1 +}",
        "if {",
        "proc",
        "$",
        "${",
        "\\",
    ]
    lines = script.split("\n")
    if not lines:
        return script
    idx = rng.randint(0, len(lines) - 1)
    lines[idx] = rng.choice(fragments)
    return "\n".join(lines)
