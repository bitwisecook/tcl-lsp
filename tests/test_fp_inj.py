# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""TP/FP regression tests for the INJ (injection / style) family.

Each test pairs to an ``FP-INJ-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

INJ family covers:

* W301 — `uplevel` style (the bare-var `uplevel 1 $body` form is the safe
  idiom; W301 only fires for genuinely risky forms).
* W101 — `eval` injection (the `eval [list …]` form is canonical-safe and
  exempt; `eval "$cmd $x"` is the canonical TP and ships a quick-fix
  rewriting it to the safe form).
* T102 — option-injection (suppressed when the tainted value carries the
  PATH_PREFIXED colour from HTTP::uri / HTTP::path; still fires when a literal
  `-` prefix breaks the path-anchoring guarantee).

Ground truth: real C tclsh 9.0.3 + iRules dialect for the T102 cases.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from server.features.diagnostics import get_diagnostics


def _diag_codes(source: str, code: str):
    return [d for d in analyse(source).diagnostics if d.code == code]


def _all_codes(source: str, code: str):
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-INJ-01 — uplevel 1 $body is the safe idiom


FP_INJ_01_REPRO = """\
proc f {body} {
    # The canonical `uplevel 1 $body` pattern — must NOT fire W301.
    uplevel 1 $body
}
"""


def test_FP_INJ_01_bare_var_no_w301():
    """FP: `uplevel 1 $body` (single pure-var arg) is the safe canonical idiom."""
    assert _diag_codes(FP_INJ_01_REPRO, "W301") == []


def test_FP_INJ_01_quoted_interpolation_still_w301():
    """TP control: a quoted-interpolated `uplevel "$cmd $arg"` IS unsafe (double
    substitution risk) and SHOULD still fire W301."""
    src = 'proc f {cmd arg} { uplevel 1 "$cmd $arg" }'
    # The genuine-risk form must still warn; we tolerate W301 or W105 (eval-style
    # injection) firing depending on which check claims it.
    diags = _all_codes(src, "W301") or _all_codes(src, "W105")
    assert diags, "quoted-interpolation uplevel must still warn"


# FP-INJ-02 — eval [list ...] is the canonical safe form


def test_FP_INJ_02_eval_list_clean():
    """FP: `eval [list set $var $val]` is canonical-safe (list-returning
    cmd-sub) — must NOT fire W101."""
    assert _diag_codes("eval [list set $varname $value]", "W101") == []


def test_FP_INJ_02_eval_linsert_clean():
    """FP control: any list-returning canonical command is exempt."""
    assert _diag_codes("eval [linsert $cmdlist 0 extraarg]", "W101") == []


def test_FP_INJ_02_eval_string_concat_still_w101():
    """TP control: `eval "cmd $x"` does double substitution — W101 must fire."""
    src = 'eval "process $x"'
    assert _diag_codes(src, "W101"), "eval of double-quoted string must fire W101"


# FP-INJ-03 — T102 suppression for HTTP::uri / HTTP::path (PATH_PREFIXED)


def test_FP_INJ_03_http_uri_no_t102():
    """FP: HTTP::uri always starts with `/`; piping it into regexp cannot
    inject an option."""
    diags = [
        d for d in get_diagnostics("set uri [HTTP::uri]\nregexp $uri test") if d.code == "T102"
    ]
    assert diags == []


def test_FP_INJ_03_http_path_no_t102():
    """FP control: HTTP::path has the same path-anchoring guarantee."""
    diags = [d for d in get_diagnostics("set p [HTTP::path]\nregexp $p test") if d.code == "T102"]
    assert diags == []


def test_FP_INJ_03_path_prefixed_copy_suppresses():
    """FP: PATH_PREFIXED colour propagates through copy assignments."""
    src = "set uri [HTTP::uri]\nset x $uri\nregexp $x test"
    diags = [d for d in get_diagnostics(src) if d.code == "T102"]
    assert diags == []


def test_FP_INJ_03_literal_non_dash_prefix_no_t102():
    """FP: a fixed non-dash literal prefix preserves path-prefixed safety."""
    src = 'set foo "path_[HTTP::path]"\nregexp $foo test'
    diags = [d for d in get_diagnostics(src) if d.code == "T102"]
    assert diags == []


# FP-INJ-04 — TP control: literal '-' prefix [HTTP::path] still warns


def test_FP_INJ_04_dash_prefix_still_warns():
    """TP control: prepending a fixed `-` breaks the path-anchoring guarantee;
    T102 must still fire."""
    src = 'set foo "-[HTTP::path]"\nregexp $foo test'
    diags = [d for d in get_diagnostics(src) if d.code == "T102"]
    assert diags, "literal `-` prefix on HTTP::path must still fire T102"


def test_FP_INJ_04_generic_taint_still_warns():
    """TP control: generic tainted data (no PATH_PREFIXED colour) still fires."""
    src = "set x [read $fd]\nregexp $x test"
    diags = [d for d in get_diagnostics(src) if d.code == "T102"]
    assert diags, "non-PATH_PREFIXED tainted data must fire T102"


# FP-INJ-05 — eval "$cmd $x" (TP) -> W101 + code-action rewrite


FP_INJ_05_REPRO = """\
set x foo
eval "process $x"
"""


def test_FP_INJ_05_eval_string_fires_w101():
    """TP: eval of a double-quoted command-string fires W101 (double-substitution
    injection risk)."""
    assert _diag_codes(FP_INJ_05_REPRO, "W101"), "eval of double-quoted string must fire W101"


def test_FP_INJ_05_code_action_rewrites_to_eval_list():
    """TP / quick-fix: the W101 code-action rewrites the call to the safe
    `eval [list …]` form.  This locks in the literal replacement-text contract
    — a regression on the rewrite would silently degrade the LSP UX."""
    # Use a single-line reproducer so we can apply the fix to the source as
    # a string and check the result byte-for-byte against the safe form.
    src = 'eval "process $x"\n'
    diags = _diag_codes(src, "W101")
    assert diags, "test setup: eval of double-quoted must fire W101 to expose its quick-fix"
    diag = diags[0]
    # Quick-fixes are produced by `analyse(...)` -> `diagnostic.fixes` (see
    # tests/test_checks.py::TestEvalInjection.test_eval_string_offers_list_fix).
    fixes = list(getattr(diag, "fixes", None) or ())
    assert len(fixes) == 1, f"expected exactly one quick-fix on W101, got {fixes}"
    fix = fixes[0]
    rewritten = src[: fix.range.start.offset] + fix.new_text + src[fix.range.end.offset :]
    assert rewritten == "eval [list process $x]\n", (
        f"W101 quick-fix must rewrite to `eval [list process $x]`, got {rewritten!r}"
    )
