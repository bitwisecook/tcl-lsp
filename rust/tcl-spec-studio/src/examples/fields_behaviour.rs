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

//! Field examples: behaviour, side effects, compiler hooks, taint, and the
//! advanced descriptors.
//!
//! One entry per schema field key. There is no group-level fallback — a
//! shared snippet is how a setting ships without a sample that shows *it*.

use super::{Example, focus};

/// Shared with the `traits` picker catalogue (see `catalogue_template`), so
/// the field and the catalogue introduce traits with the same snippet.
pub(super) const FIELD_TRAITS: Example = Example {
    code: "if {$ready} { start }\neval $script\nset n [string length $name]",
    focuses: &[
        focus(0, "if", "CONTROL_FLOW: reachability follows the branch"),
        focus(
            1,
            "eval $script",
            "EVALUATES_CODE: the argument is analysed as a script",
        ),
        focus(
            2,
            "[string length $name]",
            "PURE: the optimiser may fold the call",
        ),
    ],
};
/// Shared with the `variableScope` catalogue; the field itself lives in the
/// core half.
pub(super) const FIELD_VARIABLE_SCOPE: Example = Example {
    code: "proc build {} {\n    ttk::entry .country -textvariable country\n}\nbuild\nputs $::country",
    focuses: &[
        focus(
            1,
            "-textvariable country",
            "Global resolves the unqualified link as ::country",
        ),
        focus(
            4,
            "$::country",
            "reads the same linked variable outside the procedure",
        ),
    ],
};
/// Shared with the `scriptTiming` catalogue; the field itself lives in the
/// core half.
pub(super) const FIELD_SCRIPT_TIMING: Example = Example {
    code: "button .save -command {save_document}\nputs ready",
    focuses: &[
        focus(
            0,
            "-command {save_document}",
            "stores this script for a later button event",
        ),
        focus(
            1,
            "puts ready",
            "runs when construction returns, before any future click",
        ),
    ],
};

/// One entry per field key this half owns, in the order of the field list.
pub(super) const ENTRIES: &[(&str, Example)] = &[
    ("traits", FIELD_TRAITS),
    (
        "arg_role_resolver",
        Example {
            code: "if {$a} { one } elseif {$b} { two } else { three }",
            focuses: &[
                focus(
                    0,
                    "{$a}",
                    "an expression role assigned from the actual call",
                ),
                focus(
                    0,
                    "{ two }",
                    "a body role found by walking the elseif chain",
                ),
            ],
        },
    ),
    (
        "repeated_args",
        Example {
            code: "global cfg log db\nvariable host localhost port 8080",
            focuses: &[
                focus(0, "db", "stride 1 from index 0: rename reaches every name"),
                focus(
                    1,
                    "port",
                    "stride 2: every other word is a name, the rest are values",
                ),
            ],
        },
    ),
    (
        "frame_effect",
        Example {
            code: "upvar #0 $name local\nuplevel 1 { set touched 1 }",
            focuses: &[
                focus(0, "#0", "the level word selects the global frame"),
                focus(0, "$name local", "an alias pair: $name lives in that frame"),
                focus(
                    1,
                    "{ set touched 1 }",
                    "a script that runs in the selected frame",
                ),
            ],
        },
    ),
    (
        "clause_shape_check",
        Example {
            code: "if {$a} { one } elseif {$b}",
            focuses: &[
                focus(
                    0,
                    "if {$a} { one }",
                    "any chain length is fine while cond body pairs alternate",
                ),
                focus(
                    0,
                    "elseif {$b}",
                    "the rhythm breaks: the missing body is reported here",
                ),
            ],
        },
    ),
    (
        "command_prefix_resolver",
        Example {
            code: "interp alias {} greet {} puts stdout",
            focuses: &[
                focus(0, "{} greet", "the path and alias name come first"),
                focus(
                    0,
                    "puts stdout",
                    "the resolver places the command prefix after them",
                ),
            ],
        },
    ),
    (
        "script_timing_resolver",
        Example {
            code: "send other {work now}\nsend -async other {work later}",
            focuses: &[
                focus(
                    0,
                    "{work now}",
                    "the resolver reports SameInvocation without -async",
                ),
                focus(
                    1,
                    "{work later}",
                    "the resolver reports Deferred when -async is present",
                ),
            ],
        },
    ),
    (
        "callback_taint_inputs",
        Example {
            code: "entry .password -validatecommand {set proposed %P; eval $proposed}\nbind .password <Key> {set typed %A; eval $typed}",
            focuses: &[
                focus(
                    0,
                    "%P",
                    "the proposed editable value is external input when validation runs",
                ),
                focus(
                    0,
                    "$proposed",
                    "carries that value to the code-evaluation sink",
                ),
                focus(
                    1,
                    "%A",
                    "the typed event character is external input for this binding",
                ),
            ],
        },
    ),
    (
        "command_forms",
        Example {
            code: "incr count\nincr count 5",
            focuses: &[
                focus(
                    0,
                    "incr count",
                    "matches the implicit form by arity: count is the only role",
                ),
                focus(
                    1,
                    "5",
                    "the explicit form's own roles say this is the increment, not a variable",
                ),
            ],
        },
    ),
    (
        "semantic_operation",
        Example {
            code: "set port [dict get $config port]\nset n [llength $items]",
            focuses: &[
                focus(0, "dict get", "resolves to the DictGet intrinsic"),
                focus(
                    1,
                    "llength",
                    "a different spelling resolves to its own operation, ListLength",
                ),
            ],
        },
    ),
    (
        "completion",
        Example {
            code: "error \"missing input\"\nputs unreachable",
            focuses: &[
                focus(
                    0,
                    "error \"missing input\"",
                    "always finishes with the error code",
                ),
                focus(
                    1,
                    "puts unreachable",
                    "dead code after a call that always raises",
                ),
            ],
        },
    ),
    (
        "const_fold",
        Example {
            code: "set order [lreverse {c b a}]",
            focuses: &[
                focus(0, "{c b a}", "every argument is a literal"),
                focus(
                    0,
                    "[lreverse {c b a}]",
                    "so the folder computes a b c at compile time",
                ),
            ],
        },
    ),
    (
        "const_fold_versioned",
        Example {
            code: "set bits [format %b 5]",
            focuses: &[
                focus(0, "%b", "binary conversion exists only from Tcl 8.6"),
                focus(
                    0,
                    "[format %b 5]",
                    "folds to 101 when the target is 8.6 or later, stays a call before",
                ),
            ],
        },
    ),
    (
        "lowering_hook",
        Example {
            code: "if {$x > 0} { positive } else { other }",
            focuses: &[
                focus(0, "if", "lowered by its own hook into branch blocks"),
                focus(
                    0,
                    "{ positive }",
                    "becomes a block, not an opaque script argument",
                ),
            ],
        },
    ),
    (
        "codegen_hook",
        Example {
            code: "lappend items $value",
            focuses: &[
                focus(
                    0,
                    "lappend",
                    "emits the dedicated bytecode C Tcl would, not a generic invoke",
                ),
                focus(0, "$value", "pushed, then appended in place"),
            ],
        },
    ),
    (
        "inline_codegen_hook",
        Example {
            code: "if {[info exists cfg(port)]} { use $cfg(port) }\nset n [llength $items]",
            focuses: &[
                focus(
                    0,
                    "[info exists cfg(port)]",
                    "in value position the inline emitter produces the test directly",
                ),
                focus(
                    1,
                    "[llength $items]",
                    "another value-position call with its own inline emitter",
                ),
            ],
        },
    ),
    (
        "bpf_op",
        Example {
            code: "load16 proto pkt 12 be\ndeny",
            focuses: &[
                focus(
                    0,
                    "load16",
                    "lowers to a 16-bit packet load with a PKT_READ effect",
                ),
                focus(1, "deny", "lowers to a verdict for the program type"),
            ],
        },
    ),
    (
        "native_lowering",
        Example {
            code: "lappend items $x\nglobal cfg\nbreak",
            focuses: &[
                focus(0, "lappend", "a cell read-modify-write shape"),
                focus(1, "global", "a scope link"),
                focus(2, "break", "a fixed completion"),
            ],
        },
    ),
    (
        "analyser_hook",
        Example {
            code: "package require Tk\nproc show {w} { pack $w }",
            focuses: &[
                focus(
                    0,
                    "package require",
                    "routed to the package analyser family",
                ),
                focus(
                    1,
                    "proc",
                    "routed to the proc family for the definition record",
                ),
            ],
        },
    ),
    (
        "command_table_effect",
        Example {
            code: "proc greet {} { puts hi }\nrename greet hello\nhello",
            focuses: &[
                focus(0, "proc greet", "defines greet: no longer unknown"),
                focus(
                    1,
                    "rename greet hello",
                    "moves it: hello exists now and greet does not",
                ),
                focus(2, "hello", "resolves instead of being reported"),
            ],
        },
    ),
    (
        "side_effects",
        Example {
            code: "puts $chan $line\nfile delete $tmp",
            focuses: &[
                focus(
                    0,
                    "puts",
                    "declares a channel write, so the call is never removable",
                ),
                focus(
                    1,
                    "file delete",
                    "declares a filesystem write that ordering keeps after the puts",
                ),
            ],
        },
    ),
    (
        "world_effects",
        Example {
            code: "set before [info commands ::app::*]\nnamespace delete ::app\nset after [info commands ::app::*]",
            focuses: &[
                focus(
                    1,
                    "namespace delete ::app",
                    "declares the namespace and command domains written",
                ),
                focus(
                    2,
                    "[info commands ::app::*]",
                    "the optimiser cannot reuse the earlier result across it",
                ),
            ],
        },
    ),
    (
        "state_transitions",
        Example {
            code: "proc greet {} { puts hi }\ninterp alias {} hi {} greet",
            focuses: &[
                focus(
                    0,
                    "greet",
                    "a command binding comes into being under this name",
                ),
                focus(
                    1,
                    "hi",
                    "an alias binding appears, provably targeting greet",
                ),
            ],
        },
    ),
    (
        "dispatch_dependencies",
        Example {
            code: "set n [string length $s]\nrename ::string ::str\nset m [string length $s]",
            focuses: &[
                focus(
                    0,
                    "[string length $s]",
                    "resolution relies only on the binding staying put",
                ),
                focus(1, "rename ::string ::str", "changes that domain"),
                focus(
                    2,
                    "[string length $s]",
                    "the registry's knowledge no longer applies here",
                ),
            ],
        },
    ),
    (
        "result_stability",
        Example {
            code: "set a [clock seconds]\nset b [clock seconds]\nset n [string length $s]",
            focuses: &[
                focus(
                    0,
                    "[clock seconds]",
                    "volatile: never repeats, never reused",
                ),
                focus(
                    2,
                    "[string length $s]",
                    "referentially transparent: a second call may reuse this",
                ),
            ],
        },
    ),
    (
        "constraints",
        Example {
            code: "mycommand -from 3 -to 1",
            focuses: &[
                focus(
                    0,
                    "-from 3",
                    "consulted only after every declarative row stays silent",
                ),
                focus(
                    0,
                    "-to 1",
                    "a relation between values that only code can judge",
                ),
            ],
        },
    ),
    (
        "literal_argument_validator",
        Example {
            code: "trace add variable cfg {read write} on_change\ntrace add variable cfg {enter leave} on_change",
            focuses: &[
                focus(0, "{read write}", "operations legal for a variable trace"),
                focus(
                    1,
                    "{enter leave}",
                    "execution-trace operations on a variable trace: reported",
                ),
            ],
        },
    ),
    (
        "tk_geometry",
        Example {
            code: "frame .panel\nlabel .name -text Name\npack .name -in .panel\npack configure .name -padx 8\npack forget .name",
            focuses: &[
                focus(2, "pack .name", "the direct form places the widget"),
                focus(2, "-in .panel", "selects the effective container"),
                focus(
                    3,
                    "configure .name",
                    "the placement subcommand reconfigures it",
                ),
                focus(4, "forget .name", "a release subcommand stops managing it"),
            ],
        },
    ),
    (
        "unsafe_command",
        Example {
            code: "when HTTP_REQUEST {\n    uplevel 1 { set escaped 1 }\n}",
            focuses: &[
                focus(
                    1,
                    "uplevel",
                    "escapes the iRules sandbox: IRULE2003 unsafe command",
                ),
                focus(
                    1,
                    "{ set escaped 1 }",
                    "runs in a frame the rule should never reach",
                ),
            ],
        },
    ),
    (
        "event_requires",
        Example {
            code: "when CLIENT_ACCEPTED {\n    set uri [HTTP::uri]\n}",
            focuses: &[
                focus(
                    0,
                    "CLIENT_ACCEPTED",
                    "the surrounding event has no HTTP profile yet",
                ),
                focus(
                    1,
                    "HTTP::uri",
                    "requires one: IRULE1001 command not valid in this event",
                ),
            ],
        },
    ),
    (
        "event_requirement_forms",
        Example {
            code: "when RULE_INIT {\n    FIX::tag map set sender /Common/senders\n}\nwhen FIX_MESSAGE {\n    set type [FIX::tag get 35]\n}",
            focuses: &[
                focus(
                    1,
                    "map set",
                    "this spelling carries no requirement, so RULE_INIT is fine",
                ),
                focus(
                    4,
                    "get 35",
                    "this spelling is only valid inside FIX_MESSAGE",
                ),
            ],
        },
    ),
    (
        "data_collection",
        Example {
            code: "when HTTP_REQUEST {\n    HTTP::collect 1024\n}\nwhen HTTP_REQUEST_DATA {\n    set body [HTTP::payload]\n    HTTP::release\n}",
            focuses: &[
                focus(
                    1,
                    "HTTP::collect 1024",
                    "starts the collection the payload read depends on",
                ),
                focus(
                    4,
                    "[HTTP::payload]",
                    "reads bytes that exist only after a collect",
                ),
                focus(
                    5,
                    "HTTP::release",
                    "pairs with the collect; a missing one is diagnosed",
                ),
            ],
        },
    ),
    (
        "side_switch_target",
        Example {
            code: "when SERVER_CONNECTED {\n    clientside { TCP::collect }\n}",
            focuses: &[
                focus(1, "clientside", "switches the body to the client side"),
                focus(
                    1,
                    "TCP::collect",
                    "checked as a client-side call, not a server-side one",
                ),
            ],
        },
    ),
    (
        "event_handler_priority",
        Example {
            code: "when HTTP_REQUEST priority 100 { log local0. first }\nwhen HTTP_REQUEST { log local0. second }",
            focuses: &[
                focus(0, "priority 100", "an explicit priority; lower runs first"),
                focus(
                    1,
                    "when HTTP_REQUEST {",
                    "omitted: the runtime default, 500, applies unreported",
                ),
            ],
        },
    ),
    (
        "irules_top_level_effect",
        Example {
            code: "priority 100\nwhen HTTP_REQUEST { log local0. early }",
            focuses: &[
                focus(
                    0,
                    "priority 100",
                    "persists for the declarations that follow",
                ),
                focus(
                    1,
                    "when HTTP_REQUEST",
                    "inherits 100 instead of the default 500",
                ),
            ],
        },
    ),
    (
        "body_kind",
        Example {
            code: "if {$ok} { set status ready }\nproc run {} { set status ready }\nputs $status",
            focuses: &[
                focus(
                    0,
                    "{ set status ready }",
                    "a Plain body: the write reaches the enclosing scope",
                ),
                focus(
                    1,
                    "{ set status ready }",
                    "a Structural body: its own frame, no data flow out",
                ),
                focus(2, "$status", "sees only the first write"),
            ],
        },
    ),
    (
        "body_interpreter",
        Example {
            code: "interp eval $child {set remote 1}\nputs $remote",
            focuses: &[
                focus(
                    0,
                    "$child",
                    "Argument(1) selects the interpreter that owns the body",
                ),
                focus(
                    0,
                    "{set remote 1}",
                    "this body runs in the selected child interpreter",
                ),
                focus(
                    1,
                    "$remote",
                    "the child interpreter's variable is not visible in the caller",
                ),
            ],
        },
    ),
    (
        "taint_output_sink",
        Example {
            code: "set name [HTTP::header X-User]\nHTTP::respond 200 content \"<b>$name</b>\"",
            focuses: &[
                focus(
                    0,
                    "[HTTP::header X-User]",
                    "client-controlled text enters here",
                ),
                focus(
                    1,
                    "$name",
                    "echoed into the response unescaped: the declared code, IRULE3001, fires",
                ),
            ],
        },
    ),
    (
        "taint_output_sink_subcommands",
        Example {
            code: "set ua [HTTP::header User-Agent]\nHTTP::header insert X-Seen $ua\nHTTP::header exists $ua",
            focuses: &[
                focus(0, "[HTTP::header User-Agent]", "an untrusted value"),
                focus(
                    1,
                    "insert",
                    "a listed subcommand makes this call the sink: IRULE3002",
                ),
                focus(
                    2,
                    "exists",
                    "an unlisted subcommand is no sink, even with the same tainted word",
                ),
            ],
        },
    ),
    (
        "taint_log_sink",
        Example {
            code: "set user [HTTP::header X-User]\nlog local0. \"login for $user\"",
            focuses: &[
                focus(
                    0,
                    "[HTTP::header X-User]",
                    "attacker text; an embedded newline forges a second entry",
                ),
                focus(
                    1,
                    "$user",
                    "reaching the log line raises the declared code, IRULE3003",
                ),
            ],
        },
    ),
    (
        "taint_network_sink_args",
        Example {
            code: "set host [gets $chan]\nset sock [socket $host 443]",
            focuses: &[
                focus(0, "[gets $chan]", "an untrusted destination"),
                focus(
                    1,
                    "$host",
                    "in a listed position: T104 server-side request forgery",
                ),
                focus(1, "443", "an unlisted position is never a network sink"),
            ],
        },
    ),
    (
        "taint_code_sink_args",
        Example {
            code: "set lambda [gets $chan]\napply $lambda $arg",
            focuses: &[
                focus(0, "[gets $chan]", "untrusted script text"),
                focus(
                    1,
                    "$lambda",
                    "the listed slot is evaluated as code: T100 fires",
                ),
                focus(
                    1,
                    "$arg",
                    "an unlisted slot is bound as data, never re-evaluated",
                ),
            ],
        },
    ),
    (
        "taint_interp_eval_subcommands",
        Example {
            code: "set script [gets $chan]\ninterp eval $child $script\ninterp exists $child",
            focuses: &[
                focus(0, "[gets $chan]", "untrusted text"),
                focus(
                    1,
                    "eval",
                    "a listed subcommand runs it in another interpreter: T105",
                ),
                focus(
                    2,
                    "exists",
                    "an unlisted subcommand is not a cross-interpreter sink",
                ),
            ],
        },
    ),
    (
        "taint_source",
        Example {
            code: "set q [HTTP::query]\nset msg \"search for $q\"\nHTTP::respond 200 content $msg",
            focuses: &[
                focus(
                    0,
                    "[HTTP::query]",
                    "the result enters analysis attacker-controlled",
                ),
                focus(1, "$q", "text derived from it stays tainted"),
                focus(2, "$msg", "raw taint reaching a sink is reported"),
            ],
        },
    ),
    (
        "taint_transform",
        Example {
            code: "set name [HTTP::header X-User]\nset safe [HTML::encode $name]\nHTTP::respond 200 content \"<p>$safe</p>\"",
            focuses: &[
                focus(
                    1,
                    "[HTML::encode $name]",
                    "adds HTML_ESCAPED to the tainted value passing through",
                ),
                focus(
                    2,
                    "$safe",
                    "the response sink accepts the proven value without a finding",
                ),
            ],
        },
    ),
    (
        "taint_double_encode_colour",
        Example {
            code: "set once [URI::encode $q]\nset twice [URI::encode $once]",
            focuses: &[
                focus(
                    0,
                    "[URI::encode $q]",
                    "the first pass stamps URL_ENCODED on its result",
                ),
                focus(
                    1,
                    "$once",
                    "input already carrying that colour: T106 reports double encoding",
                ),
            ],
        },
    ),
    (
        "taint_sink_safe_colour",
        Example {
            code: "set host [gets $chan]\nset atom [validate_host $host]\nexec ping -c 1 $atom",
            focuses: &[
                focus(0, "[gets $chan]", "untrusted text arrives"),
                focus(
                    1,
                    "[validate_host $host]",
                    "a validator stamps the sink's accepted colour, SHELL_ATOM",
                ),
                focus(
                    2,
                    "$atom",
                    "carries that proof, so the exec sink stays quiet",
                ),
            ],
        },
    ),
    (
        "taint_sink_gate",
        Example {
            code: "subst -nocommands $template\nsubst $template",
            focuses: &[
                focus(
                    0,
                    "-nocommands",
                    "the gate sees this flag and switches the sink off for this call",
                ),
                focus(
                    1,
                    "$template",
                    "without it the same tainted word reaches a live code sink",
                ),
            ],
        },
    ),
    (
        "credential_options",
        Example {
            code: "http::geturl $url -headers [list Authorization \"Bearer s3cr3t\"]",
            focuses: &[
                focus(0, "-headers", "a listed option makes its value a secret"),
                focus(
                    0,
                    "\"Bearer s3cr3t\"",
                    "a literal here is a hard-coded credential finding",
                ),
            ],
        },
    ),
    (
        "sensitive_headers",
        Example {
            code: "HTTP::header insert Authorization \"Basic dXNlcjpwYXNz\"\nHTTP::header insert X-Request-Id $id",
            focuses: &[
                focus(0, "Authorization", "a listed name makes the value a secret"),
                focus(
                    0,
                    "\"Basic dXNlcjpwYXNz\"",
                    "a literal secret is a hard-coded credential finding",
                ),
                focus(1, "X-Request-Id", "an unlisted header is ordinary data"),
            ],
        },
    ),
    (
        "setter_constraints",
        Example {
            code: "HTTP::uri \"/app$path\"\nHTTP::uri $path",
            focuses: &[
                focus(
                    0,
                    "\"/app$path\"",
                    "the literal prefix satisfies the setter rule",
                ),
                focus(
                    1,
                    "$path",
                    "no literal prefix: IRULE3101 requires the value to start with /",
                ),
            ],
        },
    ),
    (
        "xc_translatable",
        Example {
            code: "set ip [IP::client_addr]\nafter 100 { log local0. delayed }",
            focuses: &[
                focus(
                    0,
                    "IP::client_addr",
                    "overridden to translatable despite its namespace prefix",
                ),
                focus(
                    1,
                    "after",
                    "overridden to never translate, so the translator reports it",
                ),
            ],
        },
    ),
    (
        "deprecated_replacement",
        Example {
            code: "set ip [remote_addr]\nset ip [IP::remote_addr]",
            focuses: &[
                focus(
                    0,
                    "remote_addr",
                    "the deprecation warning names the replacement",
                ),
                focus(1, "IP::remote_addr", "what the quick fix offers instead"),
            ],
        },
    ),
    (
        "deprecated_replacement_drop_in",
        Example {
            code: "set ip [remote_addr]\nredirect \"https://example.com/\"",
            focuses: &[
                focus(
                    0,
                    "remote_addr",
                    "drop-in: the quick fix rewrites the head to IP::remote_addr",
                ),
                focus(
                    1,
                    "redirect",
                    "not drop-in: the warning points at HTTP::redirect and leaves the arguments alone",
                ),
            ],
        },
    ),
    (
        "byte_array_payload",
        Example {
            code: "set bytes [HTTP::payload]\nHTTP::payload replace 0 4 [string toupper $bytes]",
            focuses: &[
                focus(0, "[HTTP::payload]", "the getter returns raw bytes"),
                focus(
                    1,
                    "[string toupper $bytes]",
                    "a string operation on them: S110 corruption",
                ),
                focus(
                    1,
                    "replace 0 4",
                    "the data word at the declared index is the byte sink",
                ),
            ],
        },
    ),
    (
        "byte_array_effect",
        Example {
            code: "set raw [binary format H* deadbeef]\nset lower [string tolower $raw]",
            focuses: &[
                focus(0, "[binary format H* deadbeef]", "a byte array"),
                focus(
                    1,
                    "[string tolower $raw]",
                    "case-folding is declared lossy: S110 warns the bytes are corrupted",
                ),
            ],
        },
    ),
    (
        "definition_body",
        Example {
            code: "oo::class create Stack {\n    variable items\n    method push {x} { lappend items $x }\n}",
            focuses: &[
                focus(1, "variable items", "a member keyword the grammar names"),
                focus(
                    2,
                    "method push {x}",
                    "name and parameter list are known words",
                ),
                focus(
                    2,
                    "{ lappend items $x }",
                    "the body word is analysed as a script",
                ),
            ],
        },
    ),
    (
        "manufacturer_methods",
        Example {
            code: "oo::class create Counter { method incr {} { } }\nset c [Counter new]\nCounter create total",
            focuses: &[
                focus(1, "new", "manufactures an instance: $c is a known object"),
                focus(
                    2,
                    "create total",
                    "the name argument becomes a known command",
                ),
            ],
        },
    ),
    (
        "case_list",
        Example {
            code: "switch -glob -- $path {\n    /api/* { serve_api }\n    default { serve_static }\n}",
            focuses: &[
                focus(
                    1,
                    "/api/*",
                    "read in the selected pattern language, glob here",
                ),
                focus(1, "{ serve_api }", "each body is analysed as a script"),
            ],
        },
    ),
    (
        "oo_context_facts",
        Example {
            code: "oo::class create Shape {\n    method kind {} { return [self class] }\n}",
            focuses: &[
                focus(0, "Shape", "the defining class fixes the value"),
                focus(
                    1,
                    "[self class]",
                    "folds to ::Shape from the enclosing definition",
                ),
            ],
        },
    ),
    (
        "self_receiver_words",
        Example {
            code: "oo::class create Buffer {\n    method reset {} { [self object] clear }\n}",
            focuses: &[
                focus(
                    1,
                    "[self object]",
                    "a listed word means the receiving object itself",
                ),
                focus(
                    1,
                    "clear",
                    "so this dispatches like my clear, against this class",
                ),
            ],
        },
    ),
    (
        "object_class",
        Example {
            code: "entry .name\n.name insert end Ada\n.name get",
            focuses: &[
                focus(0, "entry .name", "the factory attaches the class to .name"),
                focus(
                    1,
                    "insert end Ada",
                    "dispatches against the declared instance methods",
                ),
                focus(2, "get", "completion and arity checks know this one too"),
            ],
        },
    ),
    (
        "defines_symbol",
        Example {
            code: "tcltest::test parse-1.1 {empty input} -body { parse \"\" } -result {}",
            focuses: &[
                focus(0, "parse-1.1", "the name argument appears in the outline"),
                focus(
                    0,
                    "{empty input}",
                    "the description argument becomes its detail",
                ),
            ],
        },
    ),
    (
        "body_scope",
        Example {
            code: "::report::defstyle plain {} {\n    data set {| | |}\n    top set {+ - +}\n}\nputs [data set]",
            focuses: &[
                focus(1, "data set", "resolves against the body-only vocabulary"),
                focus(
                    4,
                    "data set",
                    "outside the body the same word is an unknown command",
                ),
            ],
        },
    ),
    (
        "binds_handle",
        Example {
            code: "set axis [::verticalAxis $win.a]\n$axis configure -min 0",
            focuses: &[
                focus(0, "axis", "the variable that receives the handle"),
                focus(0, "::verticalAxis", "this word names the handle's class"),
                focus(
                    1,
                    "$axis configure",
                    "later method calls resolve against that class",
                ),
            ],
        },
    ),
    (
        "remote_method",
        Example {
            code: "set rpc [ILX::init my_plugin my_extension]\nset result [ILX::call $rpc -timeout 500 lookupUser $name]",
            focuses: &[
                focus(
                    0,
                    "[ILX::init my_plugin my_extension]",
                    "opens a handle onto the extension",
                ),
                focus(1, "$rpc", "the handle word"),
                focus(
                    1,
                    "lookupUser",
                    "the method word after the options: definition crosses into ILXServer.addMethod",
                ),
            ],
        },
    ),
    (
        "context_gate",
        Example {
            code: "proc check {} { return -code error bad }\nwhen HTTP_REQUEST {\n    return -code error bad\n}",
            focuses: &[
                focus(
                    0,
                    "return -code error bad",
                    "inside a procedure the spelling is valid",
                ),
                focus(
                    2,
                    "return -code error bad",
                    "directly in an event body the gate reports it",
                ),
            ],
        },
    ),
    (
        "pure",
        Example {
            code: "string length $name\nset n [string length $name]",
            focuses: &[
                focus(
                    0,
                    "string length $name",
                    "result discarded: the call does nothing, so a warning fires",
                ),
                focus(
                    1,
                    "[string length $name]",
                    "the optimiser may fold or reuse this",
                ),
            ],
        },
    ),
    (
        "mutator",
        Example {
            code: "dict set config port 8080\ndict get $config port",
            focuses: &[
                focus(
                    0,
                    "dict set",
                    "changes the variable, so the call is never dead",
                ),
                focus(
                    1,
                    "dict get",
                    "not a mutator: with the result unused it is removable",
                ),
            ],
        },
    ),
    (
        "subcommand_forms",
        Example {
            code: "entry .editor\n.editor selection present\n.editor selection clear",
            focuses: &[
                focus(1, "present", "selects the nested read-only operation form"),
                focus(2, "clear", "keeps the parent method's mutation effects"),
            ],
        },
    ),
    (
        "loop_list_header",
        Example {
            code: "dict for {key value} $config {\n    puts \"$key=$value\"\n}",
            focuses: &[
                focus(
                    0,
                    "$config",
                    "the list expression is evaluated once, before any iteration",
                ),
                focus(
                    1,
                    "puts \"$key=$value\"",
                    "a loop body: break and continue are inside a loop here",
                ),
            ],
        },
    ),
    (
        "creates_scope_alias",
        Example {
            code: "namespace upvar ::app config cfg\nset cfg(port) 8080",
            focuses: &[
                focus(0, "cfg", "after this, cfg is ::app::config in disguise"),
                focus(
                    1,
                    "set cfg(port) 8080",
                    "counts as a write to the real variable",
                ),
            ],
        },
    ),
    (
        "credential_arg",
        Example {
            code: "HTTP::header insert X-Api-Key k-0123456789",
            focuses: &[
                focus(0, "insert", "counts as word 0 for this index alone"),
                focus(
                    0,
                    "k-0123456789",
                    "word 2: a literal here is a hard-coded credential",
                ),
            ],
        },
    ),
    (
        "destructive",
        Example {
            code: "if {[file exists $tmp]} {\n    file delete $tmp\n}",
            focuses: &[
                focus(0, "file exists", "a read: nothing to caution about"),
                focus(
                    1,
                    "file delete",
                    "irreversible: the caution applies and quick fixes stay away",
                ),
            ],
        },
    ),
    (
        "returns_path",
        Example {
            code: "set target [file join $root $name]\nopen $target",
            focuses: &[
                focus(
                    0,
                    "[file join $root $name]",
                    "the result is a path, so the path colours follow it",
                ),
                focus(
                    1,
                    "$target",
                    "the open sees a path rooted at $root, not opaque text",
                ),
            ],
        },
    ),
    (
        "is_unescape",
        Example {
            code: "set text [encoding convertfrom utf-8 $wire]\nputs $text",
            focuses: &[
                focus(
                    0,
                    "[encoding convertfrom utf-8 $wire]",
                    "decodes, so any encoded-safe proof on $wire is dropped",
                ),
                focus(1, "$text", "arrives at the sink as raw taint again"),
            ],
        },
    ),
    (
        "cfg_rewrite_name",
        Example {
            code: "dict for {k v} $d {\n    puts $k\n}",
            focuses: &[
                focus(
                    0,
                    "dict for",
                    "lowered as ::tcl::dict::for, the plain name the CFG sees",
                ),
                focus(1, "puts $k", "sits in the rewritten loop's body block"),
            ],
        },
    ),
    (
        "method_prefix_matching",
        Example {
            code: "entry .editor\n.editor g\n.editor c",
            focuses: &[
                focus(
                    1,
                    "g",
                    "resolves to the one matching method, get, when Enabled",
                ),
                focus(
                    2,
                    "c",
                    "stays unresolved because cget and configure are ambiguous",
                ),
            ],
        },
    ),
    (
        "container_policy",
        Example {
            code: "pack .a -in .panel\ngrid .b -in .panel\nplace .c -in .panel -x 0 -y 0",
            focuses: &[
                focus(
                    0,
                    "pack .a -in .panel",
                    "an Exclusive manager claims .panel",
                ),
                focus(
                    1,
                    "grid .b -in .panel",
                    "a second Exclusive claim on the same container: TK1001",
                ),
                focus(
                    2,
                    "place .c",
                    "Independent: positions without claiming ownership",
                ),
            ],
        },
    ),
    (
        "container_option",
        Example {
            code: "frame .panel\nlabel .name\npack .name -in .panel",
            focuses: &[
                focus(2, ".name", "the lexical parent, ., would be the container"),
                focus(
                    2,
                    "-in .panel",
                    "the declared option's literal value replaces it as the effective container",
                ),
            ],
        },
    ),
    (
        "direct_form",
        Example {
            code: "pack .name -side left\npack configure .name -side left",
            focuses: &[
                focus(
                    0,
                    "pack .name",
                    "with the flag set, the bare form itself places the widget",
                ),
                focus(
                    1,
                    "configure",
                    "clear it and only the named subcommand places",
                ),
            ],
        },
    ),
    (
        "placement_subcommand",
        Example {
            code: "grid .name -row 0\ngrid configure .name -padx 8",
            focuses: &[
                focus(0, "grid .name", "the first placement"),
                focus(
                    1,
                    "configure .name",
                    "the declared subcommand reconfigures its widget arguments' placement",
                ),
            ],
        },
    ),
    (
        "release_subcommands",
        Example {
            code: "grid .name -row 0\ngrid remove .name\ngrid forget .name",
            focuses: &[
                focus(
                    1,
                    "remove .name",
                    "stops managing it but keeps the options for a later re-grid",
                ),
                focus(
                    2,
                    "forget .name",
                    "stops managing it and drops the options; both are listed",
                ),
            ],
        },
    ),
];
