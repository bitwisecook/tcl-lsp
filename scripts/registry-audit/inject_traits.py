#!/usr/bin/env python3
"""Ensure every Python behavioural bool is reflected as a Rust `Traits`
bit on the matching command. Adds the missing `Traits::FLAG` to the
command's `traits:` expression (or creates the field). Idempotent.

Usage: python3 scripts/registry-audit/inject_traits.py <group>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import files_by_name, load_specs, rust_dir, set_spec_field  # noqa: E402

# Python bool field -> Rust Traits flag.
BOOL_TRAITS = {
    "creates_dynamic_barrier": "CREATES_DYNAMIC_BARRIER",
    "has_loop_body": "HAS_LOOP_BODY",
    "never_inline_body": "NEVER_INLINE_BODY",
    "warn_without_terminator": "WARN_WITHOUT_TERMINATOR",
    "evaluates_code": "EVALUATES_CODE",
    "performs_substitution": "PERFORMS_SUBSTITUTION",
    "opens_channel": "OPENS_CHANNEL",
    "sources_file": "SOURCES_FILE",
    "has_switch_body": "HAS_SWITCH_BODY",
    "has_string_list_confusion_risk": "STRING_LIST_CONFUSION",
    "configures_channel": "CONFIGURES_CHANNEL",
    "has_interp_eval": "HAS_INTERP_EVAL",
    "has_destructive_ops": "HAS_DESTRUCTIVE_OPS",
    "is_irules_event_handler": "IS_EVENT_HANDLER",
    "is_unnormalized_http_getter": "UNNORMALISED_HTTP_GETTER",
    "pure": "PURE",
    "cse_candidate": "CSE_CANDIDATE",
    "diagram_action": "DIAGRAM_ACTION",
    "is_control_flow": "CONTROL_FLOW",
    "needs_start_cmd": "NEEDS_START_CMD",
    "terminates_block": "TERMINATES_BLOCK",
    "is_oo_metaclass": "IS_OO_METACLASS",
    "irules_top_level_only": "IRULES_TOP_LEVEL_ONLY",
    "is_unescape_command": "IS_UNESCAPE",
    "reads_variable_before_write": "READS_BEFORE_WRITE",
    "has_boolean_condition": "HAS_BOOLEAN_COND",
    "loop_list_header": "LOOP_LIST_HEADER",
    "creates_scope_alias": "CREATES_SCOPE_ALIAS",
    "returns_path": "RETURNS_PATH",
    "pure_evaluation": "PURE_EVALUATION",
    "destroys_variable": "DESTROYS_VARIABLE",
    "is_language_keyword": "LANGUAGE_KEYWORD",
    "produces_canonical_list": "PRODUCES_CANONICAL_LIST",
    "is_side_switch": "IS_SIDE_SWITCH",
    "defines_procedure": "DEFINES_PROCEDURE",
    "unsafe": "UNSAFE",
    "password_option_command": "PASSWORD_OPTION",
    "taint_sink": "TAINT_SINK",
}


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "tcl"
    by_name = files_by_name(rust_dir(_REPO_ROOT, group))
    changed = 0
    for spec in load_specs(group):
        want = [flag for f, flag in BOOL_TRAITS.items() if getattr(spec, f, False)]
        if not want:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        present = set(re.findall(r"Traits::([A-Z_]+)", text))
        missing = [f for f in want if f not in present]
        if not missing:
            continue
        add = " | ".join(f"Traits::{f}" for f in missing)
        m = re.search(r"^(\s*)traits:\s*(.*?),\n", text, re.S | re.M)
        if m:
            text = (
                text[: m.start()]
                + f"{m.group(1)}traits: {m.group(2).strip()} | {add},\n"
                + text[m.end() :]
            )
        else:
            text = set_spec_field(text, f"traits: {add},")
        path.write_text(text)
        changed += 1
    print(f"{group}: traits topped up on {changed} files")


if __name__ == "__main__":
    main()
