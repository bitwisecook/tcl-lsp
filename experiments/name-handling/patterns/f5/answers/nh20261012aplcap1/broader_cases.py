#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Materialise bounded F5 name questions as separate setup/operation scripts.

These are run-tagged adaptations, not exact portable-corpus answers. A runner
must record setup completion before evaluating operation, check each capability
in the intended context, retain source bytes and record actual event/frame/TMM.
The source uses Tcl 8.4 syntax; feature availability is deliberately measured.
Store scripts as data and evaluate them dynamically: literal unsupported heads
in a TMM wrapper can prevent the wrapper from loading, even inside catch.

Each case owns its namespace. A scope mentioning global slots, a child, or a
global export also requires that runner's explicit teardown. Never run these in
an interpreter with colliding names. Nothing here deploys configuration, logs,
captures results, substitutes a tcltest implementation, or claims completeness.
"""

import re


def _case(case_id, question_id, setup, operation, capabilities, scope):
    return {
        "id": case_id,
        "question_id": question_id,
        "setup": setup.strip() + "\n",
        "operation": operation.strip() + "\n",
        "required_capabilities": list(capabilities),
        "scope": scope,
    }


CASES = [
    _case(
        "current-before-global",
        "F5-N02",
        """proc ::@TAIL@_p {} {return GLOBAL}
namespace eval @NS@ {proc @TAIL@_p {} {return CURRENT}}""",
        "namespace eval @NS@ {@TAIL@_p}",
        ["namespace.eval", "proc"],
        "NH01 adapted names; clean global command ::@TAIL@_p as well as @NS@.",
    ),
    _case(
        "relative-qualifier-missing-target",
        "F5-N02",
        """namespace eval @NS@_target {proc p {} {return GLOBAL_TARGET}}
namespace eval @NS@ {namespace eval @TAIL@_target {}}""",
        "namespace eval @NS@ {@TAIL@_target::p}",
        ["namespace.eval", "proc"],
        "NH02 adapted names; clean @NS@_target too; target qualifier exists locally.",
    ),
    _case(
        "namespace-path-relative",
        "F5-N02",
        """namespace eval @NS@_path::inner {proc p {} {return PATH_TARGET}}
namespace eval @NS@ {namespace path @NS@_path}""",
        "namespace eval @NS@ {inner::p}",
        ["namespace.eval", "namespace.path", "proc"],
        "NH03 adapted names; clean @NS@_path too; path support is a separate control.",
    ),
    _case(
        "no-ancestor-walk",
        "F5-N02",
        "namespace eval @NS@ {proc @TAIL@_parent {} {return ANCESTOR}; namespace eval child {}}",
        "namespace eval @NS@::child {@TAIL@_parent}",
        ["namespace.eval", "proc"],
        "NH04 adapted names; unique tail avoids accidental global fallback.",
    ),
    _case(
        "absolute-bypasses-local",
        "F5-N02",
        """namespace eval @NS@_target {proc p {} {return ABSOLUTE}}
namespace eval @NS@::@TAIL@_target {proc p {} {return LOCAL}}""",
        "namespace eval @NS@ {@NS@_target::p}",
        ["namespace.eval", "proc"],
        "Absolute command spelling; clean @NS@_target too.",
    ),
    _case(
        "colon-run-and-empty-tail",
        "F5-N03",
        "namespace eval @NS@ {proc p {} {return NORMAL}; proc {} {} {return EMPTY_TAIL}}",
        "list [@NS@:::p] [@NS@:::]",
        ["namespace.eval", "proc"],
        "NH09 adapted names; preserve separate substitution reachability.",
    ),
    _case(
        "duplicate-display-slots",
        "F5-N03",
        """namespace eval @NS@ {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval a {proc :p {} {return SECOND_SLOT}}
}""",
        """namespace eval @NS@ {
    list [namespace eval a: {p}] [namespace eval a {:p}] \
        [namespace eval a: {namespace which p}] \
        [namespace eval a {namespace which :p}]
}""",
        ["namespace.eval", "namespace.which", "proc"],
        "NH10 adapted names; invocation identity and displayed name are distinct observations.",
    ),
    _case(
        "lone-colon-command",
        "F5-N03",
        "namespace eval @NS@ {proc : {} {return LONE_COLON}}",
        "namespace eval @NS@ {:}",
        ["namespace.eval", "proc"],
        "Lone-colon command in owned namespace.",
    ),
    _case(
        "empty-command-head",
        "F5-N03",
        "namespace eval @NS@ {proc {} {} {return EMPTY_HEAD}; set head {}}",
        "namespace eval @NS@ {$head}",
        ["namespace.eval", "proc", "command.computed-head"],
        "Computed empty head; distinguish it from namespace-qualified empty tail.",
    ),
    _case(
        "relative-variable-write-read",
        "F5-N04",
        "namespace eval @NS@::n {variable x 0}\nnamespace eval @NS@ {proc run {} {set n::x 7; return $n::x}}",
        "namespace eval @NS@ {list [run] [set n::x]}",
        ["namespace.eval", "variable", "proc"],
        "NH05 adapted names.",
    ),
    _case(
        "namespace-missing-read-global-fallback",
        "F5-N04",
        "set ::@TAIL@_x GLOBAL_VALUE\nnamespace eval @NS@ {}",
        "namespace eval @NS@ {set @TAIL@_x}",
        ["namespace.eval"],
        "NH06 adapted names; clean global variable ::@TAIL@_x.",
    ),
    _case(
        "namespace-missing-write-local",
        "F5-N04",
        "set ::@TAIL@_x GLOBAL_VALUE\nnamespace eval @NS@ {}",
        "namespace eval @NS@ {set @TAIL@_x LOCAL_VALUE}; list [set ::@TAIL@_x] [set @NS@::@TAIL@_x]",
        ["namespace.eval"],
        "Namespace write/create contrast; clean global variable ::@TAIL@_x.",
    ),
    _case(
        "namespace-exists-global-fallback",
        "F5-N04",
        "set ::@TAIL@_x GLOBAL_VALUE\nnamespace eval @NS@ {}",
        "namespace eval @NS@ {info exists @TAIL@_x}",
        ["namespace.eval", "info.exists"],
        "Existence contrast to missing read; clean ::@TAIL@_x.",
    ),
    _case(
        "relative-variable-missing-parent",
        "F5-N04",
        "namespace eval @NS@ {}",
        "namespace eval @NS@ {set absent::x VALUE}",
        ["namespace.eval"],
        "Creation with nonexistent relative qualifier; expected error is a measurement.",
    ),
    _case(
        "literal-variable-name-data",
        "F5-N05",
        "namespace eval @NS@ {set {} EMPTY; set {$n} LITERAL_DOLLAR; set {{a}} LITERAL_BRACES}",
        "namespace eval @NS@ {list [set {}] [set {$n}] [set {{a}}] [info exists {{a}}]}",
        ["namespace.eval", "info.exists"],
        "NH08 adapted namespace; braced spellings remain literal.",
    ),
    _case(
        "unicode-variable-name",
        "F5-N05",
        "namespace eval @NS@ {set name [format %c 955]; set $name UNICODE}",
        "namespace eval @NS@ {list [set $name] [string length $name] [info exists $name]}",
        ["namespace.eval", "format.percent-c", "info.exists"],
        "Constructed U+03BB name; capture encoding explicitly.",
    ),
    _case(
        "nul-variable-name",
        "F5-N05",
        "namespace eval @NS@ {set name [binary format H* 610062]; set $name NUL_NAME}",
        "namespace eval @NS@ {list [set $name] [string length $name] [info exists $name]}",
        ["namespace.eval", "binary.format.hex", "info.exists"],
        "Name contains literal NUL; retain binary and Unicode representation separately.",
    ),
    _case(
        "array-index-colons-parentheses",
        "F5-N05",
        "namespace eval @NS@ {set {arr(a::b(c))} ARRAY_DATA}",
        "namespace eval @NS@ {list [set {arr(a::b(c))}] [array names arr]}",
        ["namespace.eval", "array.names"],
        "Array index contains namespace separator and parentheses.",
    ),
    _case(
        "upvar-zero-current-frame",
        "F5-N06",
        "namespace eval @NS@ {proc run {} {set x 1; upvar 0 x y; set y 2; return $x}}",
        "@NS@::run",
        ["namespace.eval", "proc", "upvar.level-zero"],
        "NH14 adapted names.",
    ),
    _case(
        "upvar-one-namespace-caller",
        "F5-N06",
        "namespace eval @NS@ {variable x NAMESPACE_VALUE; proc run {} {upvar 1 x y; return $y}}",
        "namespace eval @NS@ {run}",
        ["namespace.eval", "variable", "proc", "upvar.level-one"],
        "NH15 adapted names; caller is namespace eval.",
    ),
    _case(
        "upvar-global-frame",
        "F5-N06",
        "set ::@TAIL@_x GLOBAL\nnamespace eval @NS@ {proc run {} {upvar #0 @TAIL@_x y; set y CHANGED; return $y}}",
        "list [@NS@::run] [set ::@TAIL@_x]",
        ["namespace.eval", "proc", "upvar.absolute-level"],
        "Explicit #0; clean global variable ::@TAIL@_x.",
    ),
    _case(
        "global-declaration-link",
        "F5-N06",
        "set ::@TAIL@_x GLOBAL\nnamespace eval @NS@ {proc run {} {global @TAIL@_x; set @TAIL@_x CHANGED}}",
        "@NS@::run; set ::@TAIL@_x",
        ["namespace.eval", "proc", "global"],
        "Unique global declaration/link; clean global variable ::@TAIL@_x.",
    ),
    _case(
        "variable-declaration-link",
        "F5-N06",
        "namespace eval @NS@ {variable x NAMESPACE; proc run {} {variable x; set x CHANGED}}",
        "@NS@::run; set @NS@::x",
        ["namespace.eval", "proc", "variable"],
        "Namespace variable declaration in proc.",
    ),
    _case(
        "variable-alternating-arguments",
        "F5-N06",
        "namespace eval @NS@ {}",
        "namespace eval @NS@ {variable a b; list $a [info exists b]}",
        ["namespace.eval", "variable", "info.exists"],
        "Single variable declaration versus name/value alternation.",
    ),
    _case(
        "namespace-upvar-link",
        "F5-N06",
        "namespace eval @NS@::source {variable x ORIGINAL}\nnamespace eval @NS@::consumer {}",
        "namespace eval @NS@::consumer {namespace upvar @NS@::source x y; set y CHANGED}; set @NS@::source::x",
        ["namespace.eval", "namespace.upvar", "variable"],
        "Namespace-to-namespace link with explicit owner.",
    ),
    _case(
        "import-source-renamed",
        "F5-N07",
        """namespace eval @NS@::source {proc p {} {return ORIGINAL}; namespace export p}
namespace eval @NS@::consumer {namespace import @NS@::source::p}
rename @NS@::source::p @NS@::source::pp""",
        "namespace eval @NS@::consumer {list [p] [namespace origin p]}",
        [
            "namespace.eval",
            "namespace.export",
            "namespace.import",
            "namespace.origin",
            "proc",
            "rename",
        ],
        "NH11 adapted names; import invocation and origin text retained separately.",
    ),
    _case(
        "import-source-redefined",
        "F5-N07",
        """namespace eval @NS@::source {proc p {} {return OLD}; namespace export p}
namespace eval @NS@::consumer {namespace import @NS@::source::p}
proc @NS@::source::p {} {return NEW}""",
        "namespace eval @NS@::consumer {p}",
        ["namespace.eval", "namespace.export", "namespace.import", "proc"],
        "NH12 adapted names.",
    ),
    _case(
        "import-source-deleted",
        "F5-N07",
        """namespace eval @NS@::source {proc p {} {return OLD}; namespace export p}
namespace eval @NS@::consumer {namespace import @NS@::source::p}
rename @NS@::source::p {}""",
        "namespace eval @NS@::consumer {p}",
        [
            "namespace.eval",
            "namespace.export",
            "namespace.import",
            "proc",
            "rename.delete",
        ],
        "Delete owned source command, then invoke imported tail.",
    ),
    _case(
        "import-force-conflict",
        "F5-N07",
        """namespace eval @NS@::source {proc p {} {return SOURCE}; namespace export p}
namespace eval @NS@::consumer {proc p {} {return LOCAL}}""",
        "namespace eval @NS@::consumer {namespace import -force @NS@::source::p; p}",
        ["namespace.eval", "namespace.export", "namespace.import.force", "proc"],
        "Force replaces only owned consumer command.",
    ),
    _case(
        "import-chain-origin",
        "F5-N07",
        """namespace eval @NS@::source {proc p {} {return SOURCE}; namespace export p}
namespace eval @NS@::middle {namespace import @NS@::source::p; namespace export p}
namespace eval @NS@::consumer {namespace import @NS@::middle::p}""",
        "namespace eval @NS@::consumer {list [p] [namespace origin p]}",
        [
            "namespace.eval",
            "namespace.export",
            "namespace.import",
            "namespace.origin",
            "proc",
        ],
        "Two import edges with owned namespaces.",
    ),
    _case(
        "import-forget-global-origin",
        "F5-N07",
        """proc ::@TAIL@_source {} {return GLOBAL_IMPORT}
namespace eval :: {namespace export @TAIL@_source}
namespace eval @NS@ {namespace import ::@TAIL@_source; set before [@TAIL@_source]}""",
        "namespace eval @NS@ {namespace forget ::@TAIL@_source; list $before [namespace which @TAIL@_source] [info commands @NS@::@TAIL@_source]}",
        [
            "namespace.eval",
            "namespace.export",
            "namespace.import",
            "namespace.forget",
            "namespace.which",
            "proc",
        ],
        "NH16 adapted names; fresh interpreter only, global export state changes; clean ::@TAIL@_source.",
    ),
    _case(
        "alias-rebound-target",
        "F5-N08",
        """namespace eval @NS@ {proc target {} {return OLD}}
interp alias {} @NS@::alias {} @NS@::target
set @NS@::before [@NS@::alias]
proc @NS@::target {} {return NEW}""",
        "list [set @NS@::before] [@NS@::alias]",
        ["namespace.eval", "interp.alias.create", "proc"],
        "NH13 adapted owned namespace; before call is a setup observation.",
    ),
    _case(
        "alias-target-global-anchor",
        "F5-N08",
        """proc ::@TAIL@_target {} {return GLOBAL}
namespace eval @NS@ {proc @TAIL@_target {} {return LOCAL}; interp alias {} @NS@::alias {} @TAIL@_target}""",
        "@NS@::alias",
        ["namespace.eval", "interp.alias.create", "proc"],
        "Alias target uses bare unique tail; clean global command ::@TAIL@_target.",
    ),
    _case(
        "alias-describe-prefix",
        "F5-N08",
        "namespace eval @NS@ {proc target args {return $args}}\ninterp alias {} @NS@::alias {} @NS@::target {a b} {c d}",
        "interp alias {} @NS@::alias",
        ["namespace.eval", "proc", "interp.alias.create", "interp.alias.describe"],
        "Description retains duplicated/space-bearing prefix words without flattening.",
    ),
    _case(
        "alias-destination-missing-parent",
        "F5-N08",
        "namespace eval @NS@ {proc target {} {return TARGET}}",
        "interp alias {} @NS@::absent::alias {} @NS@::target",
        ["namespace.eval", "proc", "interp.alias.create"],
        "Observe whether qualified alias destination creates missing parent.",
    ),
    _case(
        "alias-child-target-owner",
        "F5-N08",
        """namespace eval @NS@ {proc target {} {return MAIN}}
interp create @TAIL@_child
interp eval @TAIL@_child {proc target {} {return CHILD}}
interp alias {} @NS@::alias @TAIL@_child target""",
        "@NS@::alias",
        [
            "namespace.eval",
            "proc",
            "interp.create",
            "interp.eval",
            "interp.alias.create",
        ],
        "Child-owned target; delete child interpreter @TAIL@_child before namespace teardown.",
    ),
    _case(
        "alias-deleted-lookup",
        "F5-N08",
        "namespace eval @NS@ {proc target {} {return TARGET}}\ninterp alias {} @NS@::alias {} @NS@::target\ninterp alias {} @NS@::alias {}",
        "@NS@::alias",
        ["namespace.eval", "proc", "interp.alias.create", "interp.alias.delete"],
        "Alias deletion followed by fresh lookup.",
    ),
    _case(
        "rename-procedure-namespace",
        "F5-N09",
        """namespace eval @NS@::one {variable x ONE; proc target {} {variable x; return $x}}
namespace eval @NS@::two {variable x TWO}
rename @NS@::one::target @NS@::two::target""",
        "@NS@::two::target",
        ["namespace.eval", "proc", "variable", "rename"],
        "Procedure namespace after moving owned command.",
    ),
    _case(
        "command-trace-rename",
        "F5-N09",
        """namespace eval @NS@ {
    set events {}
    proc target {} {return TARGET}
    proc changed {old new op} {variable events; lappend events [list $old $new $op]}
}
trace add command @NS@::target rename @NS@::changed""",
        "rename @NS@::target @NS@::renamed; set @NS@::events",
        ["namespace.eval", "proc", "variable", "trace.add.command", "rename"],
        "Modern trace syntax is an independently checked capability, despite 8.4-compatible source syntax.",
    ),
    _case(
        "unknown-isolated-child",
        "F5-N09",
        """namespace eval @NS@ {}
interp create @TAIL@_child
interp eval @TAIL@_child {proc unknown {head args} {return [linsert $args 0 $head]}}""",
        "interp eval @TAIL@_child {@TAIL@_absent fixed}",
        ["namespace.eval", "interp.create", "interp.eval", "proc"],
        "Only child unknown is redefined; delete @TAIL@_child; parent stock commands remain intact.",
    ),
    _case(
        "namespace-unknown-handler",
        "F5-N09",
        "namespace eval @NS@ {proc handler {head args} {return [linsert $args 0 $head]}; namespace unknown @NS@::handler}",
        "namespace eval @NS@ {@TAIL@_absent fixed}",
        ["namespace.eval", "namespace.unknown", "proc"],
        "Namespace-local handler; no global unknown redefinition.",
    ),
    _case(
        "namespace-code-deleted",
        "F5-N10",
        "namespace eval @NS@::owner {set x OLD; set @NS@::prefix [namespace code {set x}]}\nnamespace delete @NS@::owner",
        "eval [set @NS@::prefix]",
        ["namespace.eval", "namespace.code", "namespace.delete", "eval"],
        "Captured namespace spelling after deletion.",
    ),
    _case(
        "namespace-code-recreated",
        "F5-N10",
        """namespace eval @NS@::owner {set x OLD; set @NS@::prefix [namespace code {set x}]}
namespace delete @NS@::owner
namespace eval @NS@::owner {set x RECREATED}""",
        "eval [set @NS@::prefix]",
        ["namespace.eval", "namespace.code", "namespace.delete", "eval"],
        "Recreated namespace with same spelling.",
    ),
    _case(
        "active-namespace-delete-recreate",
        "F5-N10",
        "namespace eval @NS@::owner {variable x OLD; proc target {} {variable x; namespace delete @NS@::owner; namespace eval @NS@::owner {variable x NEW}; list $x [set @NS@::owner::x]}}",
        "@NS@::owner::target",
        ["namespace.eval", "namespace.delete", "proc", "variable"],
        "Delete/recreate while proc frame is active; old and new cells observed in one call.",
    ),
    _case(
        "protocol-relative-user-shadow",
        "F5-N12",
        "namespace eval @NS@::HTTP {proc method {} {return USER_SHADOW}}",
        "namespace eval @NS@ {HTTP::method}",
        ["namespace.eval", "proc"],
        "User command shares protocol tail; native HTTP command is not used by setup.",
    ),
    _case(
        "protocol-absolute-native",
        "F5-N12",
        "namespace eval @NS@::HTTP {proc method {} {return USER_SHADOW}}",
        "namespace eval @NS@ {::HTTP::method}",
        ["namespace.eval", "proc", "HTTP::method"],
        "Dynamic absolute protocol call; record HTTP profile and actual invoking event.",
    ),
    _case(
        "protocol-computed-native",
        "F5-N12",
        "namespace eval @NS@ {set head ::HTTP::method}",
        "namespace eval @NS@ {$head}",
        ["namespace.eval", "command.computed-head", "HTTP::method"],
        "Computed protocol head; requires actual HTTP flow for runtime claim.",
    ),
    _case(
        "expr-local-mathfunc",
        "F5-N14",
        "namespace eval @NS@::tcl::mathfunc {proc pick {} {return 17}}",
        "namespace eval @NS@ {expr {pick()}}",
        ["namespace.eval", "proc", "expr.mathfunc.namespace"],
        "Custom mathfunc lookup is gated; stock Tcl8.4 need not support it.",
    ),
    _case(
        "expr-mathfunc-replaced",
        "F5-N14",
        "namespace eval @NS@::tcl::mathfunc {proc pick {} {return 17}; proc pick {} {return 29}}",
        "namespace eval @NS@ {expr {pick()}}",
        ["namespace.eval", "proc", "expr.mathfunc.namespace"],
        "Fresh expression after custom mathfunc replacement, not a cache claim.",
    ),
    _case(
        "oo-forward-owned-target",
        "F5-N14",
        """namespace eval @NS@ {proc target x {list OLD $x}}
oo::class create @NS@::Class {forward send @NS@::target fixed}
set @NS@::object [@NS@::Class new]
proc @NS@::target x {list NEW $x}""",
        "[set @NS@::object] send",
        ["namespace.eval", "proc", "oo.class.create", "oo.forward", "oo.new"],
        "Optional TclOO capability; destroy owned object/class before namespace teardown.",
    ),
    _case(
        "error-missing-command",
        "F5-N15",
        "namespace eval @NS@ {}",
        "namespace eval @NS@ {@TAIL@_absent}",
        ["namespace.eval"],
        "Direct missing-command error, no inner catch.",
    ),
    _case(
        "error-missing-variable",
        "F5-N15",
        "namespace eval @NS@ {}",
        "set @NS@::absent",
        ["namespace.eval"],
        "Direct missing-variable error, no inner catch.",
    ),
    _case(
        "error-missing-namespace",
        "F5-N15",
        "namespace eval @NS@ {}",
        "namespace inscope @NS@::absent {set x}",
        ["namespace.eval", "namespace.inscope"],
        "Direct missing-namespace error after capability control.",
    ),
    _case(
        "error-link-array-element",
        "F5-N15",
        "namespace eval @NS@ {set arr(k) ELEMENT; proc target {} {upvar #0 @NS@::arr(k) link; set link(k) VALUE}}",
        "@NS@::target",
        ["namespace.eval", "proc", "upvar.absolute-level"],
        "Link targets an array element then attempts array access through scalar link.",
    ),
    _case(
        "error-rename-conflict",
        "F5-N15",
        "namespace eval @NS@ {proc source {} {return SOURCE}; proc destination {} {return DESTINATION}}",
        "rename @NS@::source @NS@::destination",
        ["namespace.eval", "proc", "rename"],
        "Conflict between owned commands only.",
    ),
    _case(
        "error-import-conflict",
        "F5-N15",
        "namespace eval @NS@::source {proc p {} {return SOURCE}; namespace export p}\nnamespace eval @NS@::consumer {proc p {} {return LOCAL}}",
        "namespace eval @NS@::consumer {namespace import @NS@::source::p}",
        ["namespace.eval", "namespace.export", "namespace.import", "proc"],
        "Non-force import conflict between owned commands.",
    ),
    _case(
        "name-braced-head",
        "F5-N16",
        "namespace eval @NS@ {proc {two words} x {return $x}}",
        "namespace eval @NS@ {{two words} {a b}}",
        ["namespace.eval", "proc"],
        "Braced spaced command spelling.",
    ),
    _case(
        "name-quoted-head",
        "F5-N16",
        "namespace eval @NS@ {proc {two words} x {return $x}}",
        'namespace eval @NS@ {"two words" {a b}}',
        ["namespace.eval", "proc"],
        "Quoted spaced command spelling.",
    ),
    _case(
        "name-substituted-head",
        "F5-N16",
        "namespace eval @NS@ {proc {two words} x {return $x}; set head {two words}}",
        "namespace eval @NS@ {$head {a b}}",
        ["namespace.eval", "proc", "command.computed-head"],
        "Substitution forms a single command word.",
    ),
    _case(
        "name-list-prefix",
        "F5-N16",
        "namespace eval @NS@ {proc {two words} x {return $x}; set prefix [list {two words} {a b}]}",
        "namespace eval @NS@ {eval $prefix}",
        ["namespace.eval", "proc", "eval"],
        "List prefix retains spaced head and data.",
    ),
    _case(
        "name-script-prefix-near-miss",
        "F5-N16",
        "namespace eval @NS@ {proc {two words} x {return $x}; set head {two words}}",
        'namespace eval @NS@ {eval "$head {a b}"}',
        ["namespace.eval", "proc", "eval"],
        "String script reparses spaced head.",
    ),
    _case(
        "brace-newline-command",
        "F5-N16",
        "namespace eval @NS@ {}",
        "namespace eval @NS@ {list a b\n{c}}",
        ["namespace.eval"],
        "Actual newline in runtime script; keep literal-load variant separate.",
    ),
    _case(
        "display-name-roundtrip",
        "F5-N16",
        "namespace eval @NS@ {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :p {} {return SECOND_SLOT}}}",
        "set @NS@::displayed [namespace eval @NS@::a: {namespace which p}]; eval [list [set @NS@::displayed]]",
        ["namespace.eval", "namespace.which", "proc", "eval"],
        "Display-FQN roundtrip after duplicate-slot setup; returned spelling need not identify the original command slot.",
    ),
]


def build_cases(tag):
    """Return independent source dictionaries with safe run-specific names."""
    if not isinstance(tag, str) or not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]{0,47}", tag):
        raise ValueError(
            "tag must start with a letter and contain at most 48 ASCII letters, digits or underscores"
        )
    result = []
    for case in CASES:
        tail = "__tcl_lsp_probe_" + tag + "_" + case["id"].replace("-", "_")
        namespace = "::" + tail
        materialised = {}
        for key, value in case.items():
            if isinstance(value, str):
                materialised[key] = value.replace("@NS@", namespace).replace(
                    "@TAIL@", tail
                )
            else:
                materialised[key] = list(value)
        materialised["setup"] = (
            "namespace eval " + namespace + " {}\n" + materialised["setup"]
        )
        result.append(materialised)
    return result
