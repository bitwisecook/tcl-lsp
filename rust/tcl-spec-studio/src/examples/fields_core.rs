// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Field examples: identity, availability, arity, types, subcommands,
//! documentation, and the option table.
//!
//! One entry per schema field key. There is no group-level fallback — a
//! shared snippet is how a setting ships without a sample that shows *it*.
//!
//! Every snippet uses a shipped command that really carries the field, so
//! the arrows point at a consequence the analyser draws today. The browser
//! locates each needle by its *first* occurrence on the line, so needles are
//! chosen to be unambiguous there.

use super::{Example, focus};

/// One entry per field key this half owns, in catalogue order.
pub(super) const ENTRIES: &[(&str, Example)] = &[
    // ---- Identity -------------------------------------------------------
    (
        "name",
        Example {
            code: "mypkg::frobnicate $value\n::mypkg::frobnicate $value",
            focuses: &[
                focus(
                    0,
                    "mypkg::frobnicate",
                    "is the fully qualified word, written without a leading ::",
                ),
                focus(
                    1,
                    "::mypkg::frobnicate",
                    "resolves to the same spec once the leading :: is normalised",
                ),
            ],
        },
    ),
    // ---- Availability ---------------------------------------------------
    (
        "surface",
        Example {
            code: "lappend names Ada\nHTTP::header exists Host",
            focuses: &[
                focus(0, "lappend", "left unset: the word exists in every dialect"),
                focus(
                    1,
                    "HTTP::header",
                    "ticks only F5 iRules, so a Tcl 9.0 document reports it as unknown",
                ),
            ],
        },
    ),
    // ---- Arity and arguments -------------------------------------------
    (
        "arity",
        Example {
            code: "incr count\nincr count 5\nincr count 5 extra",
            focuses: &[
                focus(
                    0,
                    "count",
                    "one argument after the name meets the minimum of 1",
                ),
                focus(1, "5", "a second reaches the maximum of 2"),
                focus(2, "extra", "a third draws the wrong # args diagnostic"),
            ],
        },
    ),
    (
        "arity_windows",
        Example {
            code: "ttk::treeview .tree -columns price\n.tree see $item\n.tree see $item price",
            focuses: &[
                focus(
                    1,
                    "see $item",
                    "fits the exact-1 window that is retired at Tk 9.1",
                ),
                focus(
                    2,
                    "price",
                    "is a second argument, legal only once the 9.1 window applies",
                ),
            ],
        },
    ),
    (
        "arg_roles",
        Example {
            code: "proc greet {name} {\n    puts \"hello $name\"\n}",
            focuses: &[
                focus(
                    0,
                    "greet",
                    "the Name role makes this word a defined command",
                ),
                focus(0, "{name}", "the ParamList role declares the parameters"),
                focus(
                    1,
                    "puts \"hello $name\"",
                    "the Body role analyses this as code, so $name resolves to the parameter",
                ),
            ],
        },
    ),
    (
        "arg_presentation",
        Example {
            code: "for {set i 0} {$i < 3} {incr i} {\n    puts $i\n}",
            focuses: &[
                focus(
                    0,
                    "{set i 0}",
                    "InlineScript keeps the start script on the header line",
                ),
                focus(
                    0,
                    "{incr i}",
                    "InlineScript keeps the next script inline as well",
                ),
                focus(
                    1,
                    "puts $i",
                    "the body keeps the default block layout on its own indented line",
                ),
            ],
        },
    ),
    (
        "command_prefixes",
        Example {
            code: "namespace unknown fallback\nproc fallback {} { ... }",
            focuses: &[
                focus(
                    0,
                    "fallback",
                    "is a command prefix, invoked later with the unresolved name appended",
                ),
                focus(
                    1,
                    "{}",
                    "takes no parameters, so the callback arity check reports it",
                ),
            ],
        },
    ),
    // ---- Types ----------------------------------------------------------
    (
        "return_type",
        Example {
            code: "set count [llength $items]\nputs [expr {$count + 1}]",
            focuses: &[
                focus(
                    0,
                    "[llength $items]",
                    "Int: types this result as an integer",
                ),
                focus(1, "$count", "is known to be an integer downstream"),
            ],
        },
    ),
    (
        "var_write_typing",
        Example {
            code: "set count [gets $chan line]\nputs [string toupper $line]",
            focuses: &[
                focus(
                    0,
                    "[gets $chan line]",
                    "returns a character count, which types $count as Int",
                ),
                focus(
                    1,
                    "$line",
                    "holds the line as a Fixed String, not that count",
                ),
            ],
        },
    ),
    (
        "return_type_hook",
        Example {
            code: "set count [regexp {\\d+} $text match]\nset digits [regexp -inline {\\d+} $text]",
            focuses: &[
                focus(
                    0,
                    "[regexp {\\d+} $text match]",
                    "the hook types this call as an Int match count",
                ),
                focus(1, "-inline", "switches the result kind"),
                focus(
                    1,
                    "digits",
                    "so the hook types this variable as a List instead",
                ),
            ],
        },
    ),
    (
        "return_elements",
        Example {
            code: "set endpoint [list $host $port]\nset target [lindex $endpoint 0]",
            focuses: &[
                focus(
                    0,
                    "[list $host $port]",
                    "ListOfArgs: packs these arguments as the result's elements",
                ),
                focus(
                    1,
                    "[lindex $endpoint 0]",
                    "ElementOf: yields element 0 of its list argument",
                ),
                focus(1, "target", "is therefore tracked as $host's value"),
            ],
        },
    ),
    (
        "var_elements_effect",
        Example {
            code: "lappend names Bob Cy\nset last [lindex $names end]",
            focuses: &[
                focus(
                    0,
                    "names",
                    "the container in this variable is edited in place",
                ),
                focus(0, "Bob Cy", "become its new trailing elements"),
                focus(1, "[lindex $names end]", "is therefore known to be Cy"),
            ],
        },
    ),
    (
        "representation_effect",
        Example {
            code: "set backup $items\nlappend items extra",
            focuses: &[
                focus(0, "$items", "shares one list object between two variables"),
                focus(
                    1,
                    "lappend items extra",
                    "copies the shared list before mutating it, which the performance lint reports",
                ),
            ],
        },
    ),
    (
        "arg_types",
        Example {
            code: "set line [gets $chan]\nlindex $line 0",
            focuses: &[
                focus(0, "[gets $chan]", "produces a string"),
                focus(
                    1,
                    "$line",
                    "sits in a List position, so the string shimmers to a list here",
                ),
            ],
        },
    ),
    // ---- Subcommands ----------------------------------------------------
    (
        "subcommands",
        Example {
            code: "string length $text\nstring lenght $text",
            focuses: &[
                focus(
                    0,
                    "length",
                    "is declared, so it completes and its arity is checked",
                ),
                focus(
                    1,
                    "lenght",
                    "is undeclared, so it is flagged as an unknown subcommand",
                ),
            ],
        },
    ),
    (
        "allow_unknown_subcommands",
        Example {
            code: "oo::class create Account { method deposit {amount} { ... } }\nAccount create savings\nsavings deposit 50\nsavings destroy",
            focuses: &[
                focus(
                    2,
                    "deposit",
                    "is a user-defined word, so no unknown-subcommand warning fires",
                ),
                focus(
                    3,
                    "destroy",
                    "is declared, so it keeps its full arity check",
                ),
            ],
        },
    ),
    (
        "prefix_matching",
        Example {
            code: "string le $text\nstring l $text",
            focuses: &[
                focus(
                    0,
                    "le",
                    "resolves to length under Enabled; Strict would reject the abbreviation",
                ),
                focus(
                    1,
                    "l $text",
                    "is ambiguous between length, last and lower, so it fails either way",
                ),
            ],
        },
    ),
    (
        "default_form_first_word",
        Example {
            code: "after 200 {puts tick}\nafter cancel $id",
            focuses: &[
                focus(
                    0,
                    "200",
                    "an Integer first word selects the delay form, not an unknown subcommand",
                ),
                focus(
                    1,
                    "cancel",
                    "a keyword first word still dispatches through the subcommand table",
                ),
            ],
        },
    ),
    // ---- Documentation --------------------------------------------------
    (
        "hover",
        Example {
            code: "lappend names Bob",
            focuses: &[focus(
                0,
                "lappend",
                "resting the pointer here shows the summary, synopsis and return value",
            )],
        },
    ),
    (
        "forms",
        Example {
            code: "set path [HTTP::path]\nHTTP::path /login",
            focuses: &[
                focus(
                    0,
                    "[HTTP::path]",
                    "the getter form, documented with its own synopsis",
                ),
                focus(
                    1,
                    "HTTP::path /login",
                    "the setter form shown beside it; neither row changes what is checked",
                ),
            ],
        },
    ),
    // ---- Arity and arguments (continued) -------------------------------
    (
        "assigns_variable_at",
        Example {
            code: "set greeting hello\nputs $greeting",
            focuses: &[
                focus(
                    0,
                    "greeting",
                    "position 0 names the variable this call writes",
                ),
                focus(
                    1,
                    "$greeting",
                    "counts as defined here, so no used-before-set warning",
                ),
            ],
        },
    ),
    // ---- Availability (continued) --------------------------------------
    (
        "safe_on_uninit",
        Example {
            code: "unset -nocomplain visits\nincr visits",
            focuses: &[
                focus(
                    0,
                    "unset -nocomplain visits",
                    "leaves the variable undefined",
                ),
                focus(
                    1,
                    "visits",
                    "is created by incr in 8.5+, so W210 fires only for an 8.4 target",
                ),
            ],
        },
    ),
    // ---- Types (continued) ---------------------------------------------
    (
        "inferred_storage_type",
        Example {
            code: "dict set config port 8080\nputs $config(port)",
            focuses: &[
                focus(0, "config", "is now known to hold a Dict"),
                focus(
                    1,
                    "$config(port)",
                    "reads it as an array, so the mixed-kinds diagnostic fires",
                ),
            ],
        },
    ),
    // ---- Availability (continued) --------------------------------------
    (
        "required_package",
        Example {
            code: "package require msgcat\nputs [msgcat::mc Hello]",
            focuses: &[
                focus(
                    0,
                    "package require msgcat",
                    "makes the command exist from here on",
                ),
                focus(
                    1,
                    "msgcat::mc",
                    "lights up now; above the require it draws the missing-import warning",
                ),
            ],
        },
    ),
    (
        "excluded_events",
        Example {
            code: "when HTTP_REQUEST {\n    HTTP::status\n}",
            focuses: &[
                focus(0, "HTTP_REQUEST", "is a listed event context"),
                focus(
                    1,
                    "HTTP::status",
                    "used inside it is reported by the validity check",
                ),
            ],
        },
    ),
    // ---- Options and values --------------------------------------------
    (
        "closed_value_args",
        Example {
            code: "string is integer $n\nstring is number $n",
            focuses: &[
                focus(
                    0,
                    "integer",
                    "is one of the declared values, so it is legal",
                ),
                focus(
                    1,
                    "number",
                    "is outside the closed set, so it is a diagnostic, not a missing completion",
                ),
            ],
        },
    ),
    (
        "options",
        Example {
            code: "regexp -nocase -start 3 -- $pattern $text",
            focuses: &[
                focus(0, "-nocase", "takes no value"),
                focus(
                    0,
                    "-start 3",
                    "takes one value, highlighted as flag then value",
                ),
                focus(
                    0,
                    "--",
                    "is the declared end-of-options marker, keeping the dynamic pattern safe",
                ),
            ],
        },
    ),
    (
        "option_relations",
        Example {
            code: "glob -directory $dir -path $prefix *.tcl",
            focuses: &[
                focus(0, "-directory $dir", "supplies one of a conflicting pair"),
                focus(
                    0,
                    "-path $prefix",
                    "together with it breaks option_conflict, so the checker reports the call",
                ),
            ],
        },
    ),
    (
        "option_placement",
        Example {
            code: "http::geturl $url -timeout 5000",
            focuses: &[
                focus(0, "$url", "is a positional that would end a Leading scan"),
                focus(
                    0,
                    "-timeout 5000",
                    "is still read as an option because placement is Anywhere",
                ),
            ],
        },
    ),
    (
        "reserved_trailing_words",
        Example {
            code: "lsearch -exact $flags -v",
            focuses: &[
                focus(0, "-exact", "is scanned as an option"),
                focus(
                    0,
                    "-v",
                    "sits in the final 2 reserved words, so it is the pattern, not a flag",
                ),
            ],
        },
    ),
    (
        "arg_values",
        Example {
            code: "lseq 1 to 10\nlseq 1 5",
            focuses: &[
                focus(0, "to", "is offered by completion at this position"),
                focus(
                    1,
                    "5",
                    "a plain number is still fine, since the position is not closed",
                ),
            ],
        },
    ),
    // ---- Availability (continued) --------------------------------------
    (
        "versioned_arg_values",
        Example {
            code: "persist add mcp $key\npersist add uie $key",
            focuses: &[
                focus(
                    0,
                    "mcp",
                    "is gated to BIG-IP 21.1.0+, so an older target reports it",
                ),
                focus(
                    1,
                    "uie",
                    "carries no gate, so it is present in every version",
                ),
            ],
        },
    ),
    // ---- Arity and arguments (continued) -------------------------------
    (
        "body_arg_implicit_args",
        Example {
            code: "fileutil::updateInPlace notes.txt strip_trailing\nproc strip_trailing {contents} { string trimright $contents }",
            focuses: &[
                focus(
                    0,
                    "strip_trailing",
                    "is invoked with the file contents appended at run time",
                ),
                focus(
                    1,
                    "{contents}",
                    "so one parameter passes the arity check with no argument written here",
                ),
            ],
        },
    ),
    // ---- Options and values (continued) --------------------------------
    (
        "pattern_type",
        Example {
            code: "string match *.tcl $name\nregexp {\\.tcl$} $name",
            focuses: &[
                focus(
                    0,
                    "*.tcl",
                    "Glob: checked and highlighted as a glob pattern",
                ),
                focus(
                    1,
                    "{\\.tcl$}",
                    "Regex: validated as a regular expression instead",
                ),
            ],
        },
    ),
    (
        "pattern_arg_resolver",
        Example {
            code: "lsearch $names a*\nlsearch -regexp $names ^a",
            focuses: &[
                focus(0, "a*", "the resolver reports glob for the plain call"),
                focus(1, "-regexp", "switches the pattern grammar"),
                focus(1, "^a", "so the resolver reports regex for this word"),
            ],
        },
    ),
    (
        "format_string_type",
        Example {
            code: "format %d $count\nclock format $now -format %d",
            focuses: &[
                focus(0, "%d", "Printf: a decimal integer conversion"),
                focus(1, "%d", "Clock: the day of the month instead"),
            ],
        },
    ),
    // ---- Availability (continued) --------------------------------------
    (
        "tcllib_package",
        Example {
            code: "package require json\nset data [json::json2dict $text]",
            focuses: &[
                focus(
                    0,
                    "package require json",
                    "activates the module's commands for this document",
                ),
                focus(
                    1,
                    "json::json2dict",
                    "is labelled as coming from tcllib json in completion",
                ),
            ],
        },
    ),
    (
        "introduced_version",
        Example {
            code: "ttk::entry .name -width 20",
            focuses: &[focus(
                0,
                "ttk::entry",
                "is introduced in Tk 8.5, so a Tk 8.4 target reports this use",
            )],
        },
    ),
    (
        "deprecated_version",
        Example {
            code: "interp slaves",
            focuses: &[focus(
                0,
                "slaves",
                "still works, but from 8.6 on draws the deprecation warning",
            )],
        },
    ),
    (
        "retired_version",
        Example {
            code: "trace variable counter w on_change\ntrace add variable counter write on_change",
            focuses: &[
                focus(
                    0,
                    "trace variable",
                    "is gone in 9.0, so a 9.0 target reports it as an error, not a warning",
                ),
                focus(1, "trace add variable", "is the surviving spelling"),
            ],
        },
    ),
    (
        "deprecation_fix",
        Example {
            code: "interp slaves\ninterp children",
            focuses: &[
                focus(0, "slaves", "is the deprecated word the quick fix targets"),
                focus(
                    1,
                    "children",
                    "is the replacement it offers, marked semantics-equivalent",
                ),
            ],
        },
    ),
    (
        "warn_missing_import",
        Example {
            code: "button .ok -text OK",
            focuses: &[focus(
                0,
                "button",
                "needs Tk, which wish auto-loads, so no missing-import warning fires",
            )],
        },
    ),
    (
        "is_namespace_exported",
        Example {
            code: "namespace import ::tcltest::*\ntest add-1 {} -body { expr 1+1 } -result 2",
            focuses: &[
                focus(0, "::tcltest::*", "imports the names the namespace exports"),
                focus(
                    1,
                    "test",
                    "resolves to ::tcltest::test because the bare name is exported",
                ),
            ],
        },
    ),
    // ---- Subcommands (continued) ---------------------------------------
    (
        "creates_instance_at",
        Example {
            code: "entry .name -width 20\n.name insert end Ada",
            focuses: &[
                focus(0, ".name", "position 0 names the new object command"),
                focus(
                    1,
                    ".name insert",
                    "dispatches the widget class's methods, so insert is checked",
                ),
            ],
        },
    ),
    (
        "defines_command_at",
        Example {
            code: "coroutine gen apply {{} { yield 1; yield 2 }}\ngen",
            focuses: &[
                focus(
                    0,
                    "gen",
                    "the literal at position 0 becomes a callable command once this runs",
                ),
                focus(1, "gen", "is no longer an unknown command"),
            ],
        },
    ),
    (
        "implementation_namespace",
        Example {
            code: "dict get $config port\n::tcl::dict::get $config port",
            focuses: &[
                focus(0, "dict get", "the ensemble spelling"),
                focus(
                    1,
                    "::tcl::dict::get",
                    "resolves to the same subcommand spec",
                ),
            ],
        },
    ),
    // ---- Documentation (continued) -------------------------------------
    (
        "detail",
        Example {
            code: "string length $text",
            focuses: &[focus(
                0,
                "length",
                "shows its few-word detail beside this name in the completion list",
            )],
        },
    ),
    (
        "synopsis",
        Example {
            code: "dict get $config port",
            focuses: &[focus(
                0,
                "dict get",
                "hover and completion show the usage line dict get dictionary ?key ...?",
            )],
        },
    ),
    // ---- Options and values (continued) --------------------------------
    (
        "min_abbrev",
        Example {
            code: "mycommand configure -width 10\nmycommand conf -width 10\nmycommand co -width 10",
            focuses: &[
                focus(0, "configure", "the full subcommand name"),
                focus(
                    1,
                    "conf",
                    "meets a documented minimum of 4 characters, so it resolves",
                ),
                focus(
                    2,
                    "co",
                    "is unique yet shorter than the documented minimum, so it is rejected",
                ),
            ],
        },
    ),
    (
        "arg_values_accept_prefix",
        Example {
            code: "trace add var counter write on_change\ntrace add vars counter write on_change",
            focuses: &[
                focus(0, "var", "is accepted as a unique prefix of variable"),
                focus(
                    1,
                    "vars",
                    "is not a prefix of any declared value, so it is still rejected",
                ),
            ],
        },
    ),
    // ---- Subcommands (continued) ---------------------------------------
    (
        "sub_subcommands",
        Example {
            code: "namespace ensemble create -command fs\nnamespace ensemble exists fs",
            focuses: &[
                focus(
                    0,
                    "create",
                    "is the second-level word selected after the subcommand",
                ),
                focus(0, "-command fs", "comes from create's own option table"),
                focus(
                    1,
                    "exists",
                    "declares an empty table, so no flags are offered there",
                ),
            ],
        },
    ),
    // ---- Options and values (continued) --------------------------------
    (
        "max_leading_option_words",
        Example {
            code: "namespace export -clear -clear helper",
            focuses: &[
                focus(0, "-clear", "is the one leading flag the scan consumes"),
                focus(
                    0,
                    "-clear helper",
                    "is past the cap of 1, so it is an ordinary export pattern",
                ),
            ],
        },
    ),
    (
        "taints_var_write",
        Example {
            code: "label .shown -textvariable status\nttk::entry .typed -textvariable status\neval $status",
            focuses: &[
                focus(
                    0,
                    "-textvariable status",
                    "off for a display-only link: nothing external writes here",
                ),
                focus(
                    1,
                    "-textvariable status",
                    "on: later typing writes the variable, tainting its definition",
                ),
                focus(2, "$status", "carries that taint to a code-evaluation sink"),
            ],
        },
    ),
    (
        "variable_scope",
        Example {
            code: "proc build {} {\n    checkbutton .dark -variable dark_mode\n}\nputs $::dark_mode",
            focuses: &[
                focus(
                    1,
                    "-variable dark_mode",
                    "Global: links ::dark_mode even from inside a procedure",
                ),
                focus(
                    3,
                    "$::dark_mode",
                    "reads that same variable, not a local of build",
                ),
            ],
        },
    ),
    (
        "script_timing",
        Example {
            code: "lsort -command by_age $people\nbutton .go -command {launch}",
            focuses: &[
                focus(
                    0,
                    "by_age",
                    "SameInvocation: runs before lsort returns, so its error aborts this call",
                ),
                focus(
                    1,
                    "{launch}",
                    "Deferred: stored for a later click, and cannot abort the button call",
                ),
            ],
        },
    ),
];
