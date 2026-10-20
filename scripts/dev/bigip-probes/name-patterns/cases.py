#!/usr/bin/env python3
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
"""Prepare retained naming inputs without executing them on an appliance.

Adapted corpus cases split before the final complete top-level Tcl command and
rename owned namespaces/global wrapper procedures. They are never exact-input
answers. Exact stock cases retain the complete original bytes and require a
fresh child interpreter or process for each input. Wiki records retain the full
real-tcltest suite and selected-test metadata; no extracted substitute harness
is provided. Hosted originals require exact-name absence guards before whole
source evaluation, and cleanup is conditional on successful guard setup.
JSON-serialisable original-input metadata accompanies every case.
"""

import hashlib
import json
import re
import tkinter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
BUNDLE = ROOT / "experiments/name-handling/patterns/f5/questions.json"
WIKI_PATH = "experiments/name-handling/patterns/wiki-probes.tcl"

CAPABILITIES = {
    "alias-prefix-lifetime": [
        "namespace.eval",
        "proc",
        "interp.alias.create",
        "interp.alias.describe",
        "rename",
    ],
    "computed-binding-mutation": [
        "namespace.eval",
        "proc",
        "rename",
        "command.computed-head",
    ],
    "computed-qualified": ["namespace.eval", "proc", "command.computed-head"],
    "ensemble-prefix-map": ["namespace.eval", "proc", "namespace.ensemble.create"],
    "factory-body-near-miss": ["namespace.eval", "proc"],
    "finite-factory": ["namespace.eval", "proc"],
    "frame-wrapper-near-miss": ["namespace.eval", "proc", "eval", "upvar.level-one"],
    "frame-wrapper": ["namespace.eval", "proc", "uplevel.level-one", "upvar.level-one"],
    "keyed-variable-link": ["namespace.eval", "proc", "upvar.implicit-level"],
    "list-prefix": ["namespace.eval", "proc", "eval"],
    "namespace-captured-prefix": ["namespace.eval", "namespace.code", "proc", "eval"],
    "oo-prefix-deleted-near-miss": [
        "namespace.eval",
        "oo.class.create",
        "oo.new",
        "oo.self",
        "oo.destroy",
        "eval",
    ],
    "oo-self-prefix": [
        "namespace.eval",
        "oo.class.create",
        "oo.new",
        "oo.self",
        "eval",
    ],
    "string-prefix-near-miss": ["namespace.eval", "proc", "eval"],
    "tcltest-shared-lifecycle": [
        "namespace.eval",
        "namespace.import",
        "package.require.tcltest",
        "tcltest.test.lifecycle",
    ],
}


def _sha(data):
    return hashlib.sha256(data).hexdigest()


def _questions():
    return json.loads(BUNDLE.read_text(encoding="utf-8"))["questions"]


def _original(question):
    source = question["source"]
    data = (ROOT / source["path"]).read_bytes()
    actual = _sha(data)
    if actual != source["sha256"]:
        raise ValueError("original input hash differs from bundle: " + source["path"])
    return dict(source), data.decode("utf-8")


def _commands(source):
    """Use Tcl's completeness parser; do not execute source or reparse braces."""
    interpreter = tkinter.Tcl()
    commands = []
    pending = ""
    for line in source.splitlines(keepends=True):
        pending += line
        if interpreter.call("info", "complete", pending):
            if pending.strip():
                commands.append(pending)
            pending = ""
    if pending.strip():
        if not interpreter.call("info", "complete", pending):
            raise ValueError("incomplete retained Tcl source")
        commands.append(pending)
    if not commands:
        raise ValueError("empty retained Tcl source")
    return commands


def _tag(tag):
    if not isinstance(tag, str) or not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]{0,47}", tag):
        raise ValueError(
            "tag must start with a letter and contain at most 48 ASCII letters, digits or underscores"
        )
    return tag


def _cleanup(namespace, global_procs, object_case):
    lines = []
    if object_case:
        lines.append("if {[info exists object]} {catch {$object destroy}}")
    for name in global_procs:
        lines.append(
            "if {[llength [info commands " + name + "]]} {rename " + name + " {}}"
        )
    lines.append(
        "if {[namespace exists " + namespace + "]} {namespace delete " + namespace + "}"
    )
    return "\n".join(lines) + "\n"


def build_cases(tag):
    """Return the 15 adapted corpus cases, each with separate final command."""
    _tag(tag)
    result = []
    for question in _questions():
        case_id = question["case_id"]
        if case_id not in CAPABILITIES:
            continue
        original, source = _original(question)
        tail = "__tcl_lsp_probe_" + tag + "_" + case_id.replace("-", "_")
        namespace = "::" + tail
        adapted = re.sub(r"::nhclass(?![A-Za-z0-9_])", namespace + "::Class", source)
        adapted = re.sub(r"::nh(?![A-Za-z0-9_])", namespace, adapted)
        global_procs = []
        if case_id in {"frame-wrapper", "frame-wrapper-near-miss"}:
            for name in ["forward", "change", "outer"]:
                unique = "::" + tail + "_" + name
                adapted = re.sub(
                    r"(?<![A-Za-z0-9_:$])" + name + r"(?![A-Za-z0-9_:])",
                    unique,
                    adapted,
                )
                global_procs.append(unique)
        if case_id == "namespace-captured-prefix":
            unique_tail = tail + "_emit"
            adapted = adapted.replace(
                "proc emit {value args}", "proc " + unique_tail + " {value args}"
            )
            adapted = adapted.replace(
                "namespace code [list emit ",
                "namespace code [list " + unique_tail + " ",
            )
            adapted = adapted.replace(
                "proc emit {args}", "proc ::" + unique_tail + " {args}"
            )
            global_procs.append("::" + unique_tail)
        commands = _commands(adapted)
        object_case = case_id.startswith("oo-")
        result.append(
            {
                "id": case_id,
                "question_id": question["id"],
                "canonical_question_id": question["canonical_question_id"],
                "case_id": case_id,
                "variant_id": "run-namespaced-final-command-v1",
                "original_input": original,
                "setup": "namespace eval "
                + namespace
                + " {}\n"
                + ("unset -nocomplain object\n" if object_case else "")
                + "".join(commands[:-1]),
                "operation": commands[-1],
                "cleanup": _cleanup(namespace, global_procs, object_case),
                "owned_namespaces": [namespace],
                "owned_global_procs": global_procs,
                "required_capabilities": list(CAPABILITIES[case_id]),
                "scope": "Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.",
                "adaptation": {
                    "kind": "namespace-procedure-renaming-and-stage-split",
                    "original_namespace": "::nh",
                    "actual_namespace": namespace,
                    "global_procedure_names": global_procs,
                    "added_namespace_setup": True,
                    "reset_object_capture": object_case,
                    "warning": "Stage splitting changes catch boundaries; earlier alias description, mutation, test lifecycle or caught lookup can occur in setup. A failed setup leaves final command unreached.",
                },
                "exact_source": False,
            }
        )
    if len(result) != 15:
        raise ValueError("expected 15 retained corpus cases")
    return result


def stock_cases():
    """Complete exact sources; caller must use a fresh interpreter per case."""
    result = []
    for question in _questions():
        case_id = question["case_id"]
        if case_id not in CAPABILITIES:
            continue
        original, source = _original(question)
        result.append(
            {
                "id": case_id,
                "question_id": question["id"],
                "canonical_question_id": question["canonical_question_id"],
                "case_id": case_id,
                "variant_id": "whole-portable-source-v1",
                "original_input": original,
                "setup": "",
                "operation": source,
                "cleanup": "",
                "required_capabilities": list(CAPABILITIES[case_id]),
                "scope": "Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.",
                "adaptation": None,
                "exact_source": True,
            }
        )
    return result


def stock_split_cases():
    """Original names/bytes split into stages; use a fresh stock process."""
    result = []
    for case in stock_cases():
        source = case["operation"]
        commands = _commands(source)
        # _commands ignores empty complete lines. Locate the final retained
        # command in the original byte-preserving string so prefix/trailing
        # whitespace remain exactly the authored source, including blank lines.
        offset = source.rfind(commands[-1])
        if offset < 0:
            raise ValueError("final retained command not found in source")
        prefix = source[:offset]
        operation = source[offset:]
        if prefix + operation != source:
            raise ValueError("stock stage split changed retained source bytes")
        item = dict(case)
        item.update(
            {
                "variant_id": "original-names-stage-split-v1",
                "setup": prefix,
                "operation": operation,
                "cleanup": "",
                "scope": "Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.",
                "adaptation": {
                    "kind": "original-name-setup-final-stage-split",
                    "source_bytes_when_concatenated_unchanged": True,
                    "names_rewritten": False,
                    "fresh_stock_process_required": True,
                    "catch_boundary_changed": True,
                    "warning": "Separate catches for prefix and final command alter completion/error boundaries; frame and input channel must be retained. Earlier caught lookup or nested test lifecycle remains in setup.",
                },
                "exact_source": False,
            }
        )
        result.append(item)
    return result


def _hosted_guard(namespaces, procedures, variables, commands=()):
    """Own only exact names whose absence was checked in the retained frame."""
    namespace_words = " ".join(namespaces)
    procedure_words = " ".join(procedures)
    variable_words = " ".join(variables)
    command_words = " ".join(commands)
    setup = f"""set nh_guard_scope [namespace current]
set nh_guard_procs {{}}
set nh_guard_absent {{}}
foreach nh_guard_name {{{namespace_words}}} {{
    if {{[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]}} {{error [list ownership-collision namespace-or-command $nh_guard_name]}}
    lappend nh_guard_absent [list namespace-and-command $nh_guard_name]
}}
foreach nh_guard_tail {{{procedure_words}}} {{
    set nh_guard_name [string trimright $nh_guard_scope :]::$nh_guard_tail
    if {{[llength [info commands $nh_guard_name]] || [string length [namespace which -command $nh_guard_tail]]}} {{error [list ownership-collision procedure $nh_guard_name resolved [namespace which -command $nh_guard_tail]]}}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list procedure $nh_guard_name fallback $nh_guard_tail]
}}
foreach nh_guard_name {{{command_words}}} {{
    if {{[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]}} {{error [list ownership-collision command-or-namespace $nh_guard_name]}}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list command-and-namespace $nh_guard_name]
}}
foreach nh_guard_name {{{variable_words}}} {{
    if {{[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $nh_guard_name]]}} {{error [list ownership-collision frame-variable $nh_guard_name]}}
    lappend nh_guard_absent [list frame-variable $nh_guard_name]
}}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
"""
    cleanup = f"""if {{$nh_setup_code == 0}} {{
    foreach nh_guard_name $nh_guard_procs {{
        if {{[llength [info commands $nh_guard_name]]}} {{rename $nh_guard_name {{}}}}
    }}
    foreach nh_guard_name {{{namespace_words}}} {{
        if {{[namespace exists $nh_guard_name]}} {{namespace delete $nh_guard_name}}
    }}
    foreach nh_guard_name {{{variable_words}}} {{unset -nocomplain $nh_guard_name}}
    set nh_guard_remaining {{}}
    foreach nh_guard_name $nh_guard_procs {{
        if {{[llength [info commands $nh_guard_name]]}} {{lappend nh_guard_remaining [list command $nh_guard_name]}}
    }}
    foreach nh_guard_name {{{namespace_words}}} {{
        if {{[namespace exists $nh_guard_name]}} {{lappend nh_guard_remaining [list namespace $nh_guard_name]}}
    }}
    foreach nh_guard_name {{{variable_words}}} {{
        if {{[info exists $nh_guard_name]}} {{lappend nh_guard_remaining [list frame-variable $nh_guard_name]}}
    }}
    list ownership-cleanup execution-namespace $nh_guard_scope remaining $nh_guard_remaining
}} else {{
    list ownership-not-acquired original-source-unreached
}}
"""
    return setup, cleanup


def hosted_cases():
    """Complete original sources in the actual host, with collision guards.

    The capture wrapper owns nh_* bookkeeping. Original source is evaluated
    once, without a setup/final split, name rewriting or substitute interpreter.
    Intrinsic child interpreters in the Wiki source remain part of that source.
    """
    variables = {
        "alias-prefix-lifetime": ["description"],
        "computed-qualified": ["ns", "head"],
        "computed-binding-mutation": ["ns", "head"],
        "list-prefix": ["target", "prefix"],
        "string-prefix-near-miss": ["target"],
        "oo-self-prefix": ["object"],
        "oo-prefix-deleted-near-miss": ["object", "prefix", "result"],
    }
    result = []
    for case in stock_cases():
        case_id = case["id"]
        frame_case = case_id.startswith("frame-wrapper")
        object_case = case_id.startswith("oo-")
        namespaces = [] if frame_case or object_case else ["::nh"]
        procedures = (
            ["forward", "change", "outer"]
            if frame_case
            else ["emit"]
            if case_id == "namespace-captured-prefix"
            else []
        )
        commands = ["::nhclass"] if object_case else []
        owned_variables = variables.get(case_id, [])
        setup, cleanup = _hosted_guard(
            namespaces, procedures, owned_variables, commands
        )
        if case_id == "tcltest-shared-lifecycle":
            # A retained suite's -match/-skip state can suppress this test.
            # Reject that state instead of reporting a silently skipped body.
            setup = (
                setup.rsplit("list ownership-guard", 1)[0]
                + """if {[string length [package provide tcltest]] && [llength [info commands ::tcltest::configure]]} {
    if {[::tcltest::configure -match] != "*" || [llength [::tcltest::configure -skip]]} {error {ownership-collision pre-existing tcltest selection}}
}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
"""
            )
        case.update(
            {
                "setup": setup,
                "cleanup": cleanup,
                "owned_namespaces": namespaces,
                "owned_frame_procedures": procedures,
                "owned_absolute_commands": commands,
                "owned_frame_variables": owned_variables,
                "required_guard_capabilities": ["namespace.which"]
                + (["trace.variable.inspect"] if owned_variables else []),
                "scope": "Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.",
                "hosted_guard": {
                    "absence_proof_stage": "setup",
                    "collision": "setup error; entire original source unreached; no original-name cleanup",
                    "cleanup_gate": "nh_setup_code == 0",
                    "namespace_for_unqualified_procedures": "runtime namespace current, retained in setup result",
                    "class_cleanup": "delete newly owned class command, including its newly created instances"
                    if object_case
                    else None,
                },
            }
        )
        result.append(case)
    for case in wiki_cases():
        setup, cleanup = _hosted_guard([], ["isolated"], [])
        setup = setup.rsplit("list ownership-guard", 1)[0]
        setup += """package require tcltest
set nh_guard_tcltest_match [::tcltest::configure -match]
set nh_guard_tcltest_skip [::tcltest::configure -skip]
set nh_guard_tcltest_verbose [::tcltest::configure -verbose]
"""
        setup += "::tcltest::configure -match " + case["id"] + " -skip {}\n"
        setup += (
            "list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent selected "
            + case["id"]
            + " tcltest [package present tcltest]\n"
        )
        cleanup = cleanup.replace(
            "    set nh_guard_remaining {}",
            "    ::tcltest::configure -match $nh_guard_tcltest_match -skip $nh_guard_tcltest_skip -verbose $nh_guard_tcltest_verbose\n    set nh_guard_remaining {}",
        )
        case.update(
            {
                "setup": setup,
                "cleanup": cleanup,
                "owned_namespaces": [],
                "owned_frame_procedures": ["isolated"],
                "owned_absolute_commands": [],
                "owned_frame_variables": [],
                "required_guard_capabilities": ["namespace.which"],
                "scope": "Complete byte-identical real-tcltest Wiki suite evaluated in the retained host namespace/frame, with one selected test. Original isolated helper/child interpreters and constraints are unchanged. Package load failure or ownership collision leaves suite/body unreached. Pass/skip output is harness evidence; selected body bytes and inner options remain unavailable without separately labelled instrumentation.",
                "hosted_guard": {
                    "absence_proof_stage": "setup",
                    "collision": "setup error; original suite unreached; no original-name cleanup",
                    "cleanup_gate": "nh_setup_code == 0",
                    "tcltest_selection": case["id"],
                    "tcltest_configuration_restored": [
                        "-match",
                        "-skip",
                        "-verbose",
                    ],
                    "package_state": "Real package loading and suite counters/constraints are retained; no package namespace/commands are deleted.",
                    "namespace_for_unqualified_procedures": "runtime namespace current, retained in setup result",
                },
            }
        )
        result.append(case)
    return result


def wiki_cases():
    """Return full-suite source and real-tcltest selected-test metadata."""
    result = []
    for question in _questions():
        if question.get("source", {}).get("path") != WIKI_PATH:
            continue
        original, suite = _original(question)
        selected = question["case_id"]
        capabilities = [
            "package.require.tcltest",
            "tcltest.configure",
            "tcltest.test",
            "interp.create",
            "interp.eval",
            "interp.delete",
        ]
        if selected.startswith("wiki-oo-"):
            capabilities.append("oo.class.create")
        if selected == "wiki-prefix-2":
            capabilities.append("argument.expansion")
        constraint = (
            "tcloo"
            if selected.startswith("wiki-oo-")
            else {
                "wiki-prefix-2": "tcl85",
                "wiki-code-6": "aunt",
                "wiki-code-7": "noAunt",
            }.get(selected)
        )
        result.append(
            {
                "id": selected,
                "question_id": question["id"],
                "canonical_question_id": question["canonical_question_id"],
                "case_id": selected,
                "variant_id": "real-tcltest-full-suite-selected-v1",
                "original_input": original,
                "full_suite_source": suite,
                "setup": "package require tcltest\n::tcltest::configure -match "
                + selected
                + "\n",
                "operation": suite,
                "cleanup": "",
                "constraint": constraint,
                "required_capabilities": capabilities,
                "scope": "Fresh child/process; evaluate complete unchanged suite using real tcltest and actual isolated helper/constraints. A constraint skip is not unsupported proof.",
                "adaptation": None,
                "exact_source": True,
                "result_limitations": "Real tcltest pass/skip output alone does not expose body-result bytes or caught inner error options; retain those as unavailable unless separately instrumented.",
            }
        )
    if len(result) != 30:
        raise ValueError("expected 30 retained Wiki selections")
    return result


def originals():
    """JSON-serialisable original paths/hashes and canonical identity."""
    return [
        {
            "question_id": q["id"],
            "case_id": q["case_id"],
            "canonical_question_id": q["canonical_question_id"],
            "source": dict(q["source"]),
        }
        for q in _questions()
        if q["case_id"] in CAPABILITIES or q.get("source", {}).get("path") == WIKI_PATH
    ]
