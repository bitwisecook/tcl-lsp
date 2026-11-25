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

//! The injection, `subst`, pipeline, `ReDoS` and credential checks, and the fire-and-forget W302.

use super::*;

#[test]
fn w302_fire_and_forget_bare_close_silent() {
    // FP-STY-05: `catch {close $fh}` is the documented fire-and-forget
    // idiom — no W302.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { catch {close $fh} }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "W302 must be suppressed on `catch {{close ...}}`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w302_fire_and_forget_ensemble_chan_close_silent() {
    let mut a = Analyser::new();
    let r = a.analyse("proc f {} { catch {chan close $fh} }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "W302 must be suppressed on `catch {{chan close ...}}`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w302_constructive_subcommand_still_fires() {
    // TP control: `chan configure` is constructive, not fire-and-forget.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {} { catch {chan configure $fh -blocking 0} }",
        "tcl",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "W302 must still fire on a constructive `chan configure`; got {:?}",
        r.diagnostics,
    );
}

// Security-injection checks (W300 / W301 / W309 / W312).
//
// Each fixture asserts the expected security-diagnostic set.

fn sec_codes(src: &str, code: &str) -> usize {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .count()
}

#[test]
fn w300_source_with_variable_path() {
    assert_eq!(sec_codes("source $path\n", "W300"), 1);
    // `-encoding ENC` is skipped to find the file argument.
    assert_eq!(sec_codes("source -encoding utf-8 $path\n", "W300"), 1);
    // A command-substituted path is just as dynamic as a `$var` one.
    assert_eq!(sec_codes("source [locate_lib]\n", "W300"), 1);
    // A literal path is fine.
    assert_eq!(sec_codes("source ./lib.tcl\n", "W300"), 0);
    // FP gate: a `$var` that provably holds a compile-time literal path is a
    // known file — the same as the literal form above — so it is silent.
    assert_eq!(sec_codes("set p \"./lib.tcl\"\nsource $p\n", "W300"), 0);
    // But a `$var` reassigned dynamically before the use is still flagged.
    assert_eq!(
        sec_codes("set p \"./lib.tcl\"\nset p [get_path]\nsource $p\n", "W300"),
        1
    );
    // A proc parameter is not a compile-time literal — still flagged.
    assert_eq!(sec_codes("proc f {p} { source $p }\n", "W300"), 1);
}

#[test]
fn w309_eval_uplevel_with_subst() {
    let mut a = Analyser::new();
    let r = a.analyse("eval [subst $template]\n", "tcl8.6");
    let w309: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W309)
        .collect();
    assert_eq!(w309.len(), 1);
    assert_eq!(w309[0].severity, Severity::Error);
    assert!(w309[0].message.starts_with("eval with [subst]"));
    assert_eq!(sec_codes("uplevel [subst {$x}]\n", "W309"), 1);
    // No `[subst]` → no W309.
    assert_eq!(sec_codes("eval [list set x $y]\n", "W309"), 0);
}

#[test]
fn w301_uplevel_injection() {
    // Single unbraced substituted script.
    assert_eq!(sec_codes("uplevel 1 \"set x $y\"\n", "W301"), 1);
    // Multiple args concatenate.
    assert_eq!(sec_codes("uplevel $a $b\n", "W301"), 1);
    // Braced body and the `[list …]` idiom are safe.
    assert_eq!(sec_codes("uplevel 1 {set x 1}\n", "W301"), 0);
    assert_eq!(sec_codes("uplevel 1 [list set x $y]\n", "W301"), 0);
    // A single *pure* variable script is the safe single-substitution
    // idiom — no W301; a concatenation still fires.
    assert_eq!(sec_codes("proc f {body} { uplevel 1 $body }\n", "W301"), 0);
    assert_eq!(sec_codes("uplevel 1 $body\n", "W301"), 0);
    assert_eq!(sec_codes("uplevel 1 pre$body\n", "W301"), 1);
}

#[test]
fn w312_interp_eval_injection() {
    assert_eq!(sec_codes("interp eval $child $script\n", "W312"), 1);
    assert_eq!(sec_codes("interp eval $child \"set x $y\"\n", "W312"), 1);
    // Multiple script words concatenate.
    assert_eq!(sec_codes("interp eval $foo $a $b\n", "W312"), 1);
    // invokehidden flags the hidden command word.
    assert_eq!(
        sec_codes("interp invokehidden $child $cmd $arg\n", "W312"),
        1
    );
    // Braced body is safe.
    assert_eq!(sec_codes("interp eval $child {set x 1}\n", "W312"), 0);
}

#[test]
fn w312_message_names_subcommand() {
    let mut a = Analyser::new();
    let r = a.analyse("interp eval $child $script\n", "tcl8.6");
    let w312 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W312)
        .unwrap();
    assert!(
        w312.message.contains("interp eval $child {...}"),
        "{w312:?}"
    );
}

/// tclsh 9.0.4 and 9.1b0, `set x {[format INJECTED]}`:
/// `puts [subst $x]` → `INJECTED`, and `puts [subst "$x"]` → `INJECTED`
/// too — an operand that reaches `subst` already substituted once is
/// evaluated a second time whichever way it was written.
/// `puts [subst -nocommands -novariables $x]` → `[format INJECTED]`.
#[test]
fn w102_subst_variable_argument() {
    // Bare `$var` operand fires; the message lists both kinds.
    let mut a = Analyser::new();
    let r = a.analyse("subst $x\n", "tcl8.6");
    let w102 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W102)
        .unwrap();
    assert!(
        w102.message.contains("any [cmd] and $var in the string"),
        "{w102:?}"
    );
    assert!(
        w102.message
            .contains("Add -nocommands -novariables to limit")
    );
    // A braced operand is the template as written — nothing is spliced in.
    assert_eq!(sec_codes("subst {literal $y}\n", "W102"), 0);
    // A quoted operand carrying a substitution is spliced in just as a bare
    // `$var` one is, and is reported the same way.
    assert_eq!(sec_codes("subst \"$x\"\n", "W102"), 1);
    assert_eq!(sec_codes("subst \"literal\"\n", "W102"), 0);
    assert_eq!(sec_codes("subst -nocommands -novariables $x\n", "W102"), 0);
}

/// tclsh 9.0.4 and 9.1b0, `set secret hunter2; set x {$secret}`:
/// `puts [subst -nocommands $x]` → `hunter2` — command substitution is off
/// but the variable read still happens.
#[test]
fn w102_message_narrows_with_flags() {
    let mut a = Analyser::new();
    let r = a.analyse("subst -nocommands $x\n", "tcl8.6");
    let w102 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W102)
        .unwrap();
    // Only `$var` remains active; only `-novariables` is suggested.
    assert!(w102.message.contains("any $var in the string"), "{w102:?}");
    assert!(!w102.message.contains("[cmd]"), "{w102:?}");
    assert!(w102.message.contains("Add -novariables to limit"));
}

/// A computed word before the operand is a *switch*, not the template.
///
/// tclsh 9.0.4 and 9.1b0, `set opt -novariables; set name world`:
/// `puts [subst $opt {hello $name}]` → `hello $name`. The braced literal is
/// what is substituted, and nothing can be injected into it; the computed
/// word decides only which substitutions run.
#[test]
fn w102_computed_switch_word_is_not_the_operand() {
    assert_eq!(
        sec_codes(
            "set opt -novariables
subst $opt {hello $name}
",
            "W102"
        ),
        0
    );
}

/// Tcl 9.1's positive family names the *only* substitutions that run.
///
/// tclsh 9.1b0, `set tmpl {hello $name}; set name world`:
/// `puts [subst -backslashes $tmpl]` → `hello $name` — neither a `[cmd]` nor
/// a `$var` in the operand is evaluated, so there is nothing to report
/// On 9.0.4 the same call is
/// `bad option "-backslashes": must be -nobackslashes, -nocommands, or
/// -novariables`, which is W004's finding, not this one.
#[test]
fn w102_tcl91_positive_family_backslashes_only_is_silent() {
    let mut a = Analyser::new();
    let r = a.analyse("subst -backslashes $tmpl\n", "tcl9.1");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W102),
        "{:?}",
        r.diagnostics
    );
}

/// The advice never mixes the two switch families.
///
/// tclsh 9.1b0: `subst -backslashes -nocommands $tmpl` →
/// `cannot combine positive and negative options`, so a call already using
/// the positive family gets no switch advice at all — only the templating
/// alternatives.
#[test]
fn w102_advice_never_mixes_switch_families() {
    let mut a = Analyser::new();
    let r = a.analyse("subst -commands $x\n", "tcl9.1");
    let w102 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W102)
        .unwrap();
    assert!(w102.message.contains("any [cmd] in the string"), "{w102:?}");
    assert!(!w102.message.contains("Add "), "{w102:?}");
    assert!(
        w102.message
            .contains("Use [format] / [string map] for safe templating."),
        "{w102:?}"
    );
}

/// W102 reads the lattice's template-word plan: `set opt
/// -novariables; subst $opt $x` warns of `[cmd]` alone and advises
/// `-nocommands`, exactly as `subst -novariables $x` does (tclsh 8.4 to 9.1,
/// `set x {[set y 1]$y}`: both run the command and leave `$y`), where the
/// computed switch word had made the call unreadable; a switch the lattice
/// does not prove — a parameter — keeps every kind and advises nothing; and
/// proven switches that turn both kinds off warn of nothing.
#[test]
fn w102_narrows_a_proven_switch_word() {
    let w102 = |src: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W102)
            .map(|d| d.message.clone())
            .collect()
    };
    let literal = w102("proc f {x} { subst -novariables $x }\n");
    let proven = w102("proc f {x} { set opt -novariables\n subst $opt $x }\n");
    assert_eq!(proven, literal);
    assert_eq!(proven.len(), 1, "{proven:?}");
    assert!(proven[0].contains("any [cmd] in the string"), "{proven:?}");
    assert!(proven[0].contains("Add -nocommands to limit"), "{proven:?}");
    let unproven = w102("proc f {opt x} { subst $opt $x }\n");
    assert_eq!(unproven.len(), 1, "{unproven:?}");
    assert!(
        unproven[0].contains("any [cmd] and $var") && !unproven[0].contains("Add "),
        "{unproven:?}"
    );
    assert_eq!(
        w102("proc f {x} { set opt -nocommands\n subst $opt -novariables $x }\n"),
        [] as [std::string::String; 0]
    );
}

/// A `subst` a word substitutes narrows by its proven switch as a
/// statement-level one does (#2451).
#[test]
fn w102_narrows_a_proven_switch_in_a_substituted_subst() {
    let w102 = |src: &str| -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W102)
            .map(|d| d.message.clone())
            .collect()
    };
    let statement = w102("proc f {x} { set opt -novariables; subst $opt $x }\n");
    assert_eq!(statement.len(), 1, "{statement:?}");
    for src in [
        "proc g {x} { set opt -novariables; set r [subst $opt $x] }\n",
        "proc p {t} { set opt -novariables; return [subst $opt $t] }\n",
        "proc q {t} { set opt -novariables; puts [subst $opt $t] }\n",
    ] {
        assert_eq!(w102(src), statement, "{src}");
    }
    let unproven = w102("proc g {opt x} { set r [subst $opt $x] }\n");
    assert_eq!(unproven.len(), 1, "{unproven:?}");
    assert!(!unproven[0].contains("Add "), "{unproven:?}");
}

#[test]
fn w103_open_pipeline() {
    // `|`-pipeline with substitution → WARNING (injection).
    let mut a = Analyser::new();
    let r = a.analyse("open \"|$cmd\"\n", "tcl8.6");
    let w103 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W103)
        .unwrap();
    assert_eq!(w103.severity, Severity::Warning);
    assert!(w103.message.contains("command injection"), "{w103:?}");
    // Literal `|`-pipeline → HINT.
    assert_eq!(code_sevs("open |ls\n", "W103"), vec!["Hint"]);
    assert_eq!(code_sevs("open \"|cat file\"\n", "W103"), vec!["Hint"]);
    // Bare `$var` argument → WARNING (may resolve to a pipeline).
    assert_eq!(code_sevs("open $f\n", "W103"), vec!["Warning"]);
    // A command-substituted argument is just as dynamic as a `$var` one.
    assert_eq!(code_sevs("open [pick_target]\n", "W103"), vec!["Warning"]);
    // A literal filename is fine.
    assert_eq!(sec_codes("open \"file.txt\"\n", "W103"), 0);
    // FP gate: a `$var` proven to hold a compile-time literal is treated like
    // that literal inline — a benign filename is silent, a `|`-prefixed literal
    // is the pipeline Hint.
    assert_eq!(sec_codes("set f \"data.txt\"\nopen $f\n", "W103"), 0);
    assert_eq!(code_sevs("set f \"|ls\"\nopen $f\n", "W103"), vec!["Hint"]);
    // A proc parameter is not a literal — still a Warning.
    assert_eq!(
        code_sevs("proc g {f} { open $f }\n", "W103"),
        vec!["Warning"]
    );
}

// Registry-trait gate swaps (W101 / W103 / W309 / W312 / W302).
//
// The security emitters resolve their eligible commands from registry
// traits/fields, never from command-name literals. Each swapped gate keeps
// one fire and one silent case here (beyond the per-code fixtures above).

#[test]
fn w101_gate_trait_pair_keeps_eval_only() {
    // Fire: the concat-reparse shape (EVALUATES_CODE + TAINT_SINK with a
    // static Body role at arg 0) — `eval`.
    assert_eq!(sec_codes("eval $cmd\n", "W101"), 1);
    // Silent: `uplevel` carries the trait pair but its script position is
    // resolver-driven (optional level word) — owned by W301, no W101.
    assert_eq!(sec_codes("uplevel 1 \"set x $y\"\n", "W101"), 0);
    // Silent: the coroutine injectors carry EVALUATES_CODE but take a
    // coroutine name + command *prefix* — nothing is re-parsed as script.
    assert_eq!(sec_codes("coroinject c $cmd\n", "W101"), 0);
    assert_eq!(sec_codes("coroprobe c $cmd\n", "W101"), 0);
}

#[test]
fn w309_inner_subst_head_resolves_through_registry() {
    // The `[subst …]` head check is a registry PERFORMS_SUBSTITUTION
    // lookup, so the fully-qualified spelling is caught too (`get`
    // resolves a leading `::`), where a literal-prefix match would miss it.
    assert_eq!(sec_codes("eval [::subst $template]\n", "W309"), 1);
    // A non-substituting inner head stays silent.
    assert_eq!(sec_codes("eval [format %s $x]\n", "W309"), 0);
}

#[test]
fn w103_socket_is_not_a_pipeline_opener() {
    // `socket` carries OPENS_CHANNEL too, but its first argument is an
    // address (`Tcl_OpenTcpClient`) — never a `|` exec spec — and its spec
    // carries TAINT_SOURCE, the excluder the gate pairs with the trait.
    assert_eq!(sec_codes("socket $host 80\n", "W103"), 0);
    assert_eq!(sec_codes("socket -server accept 8080\n", "W103"), 0);
    // The file opener still fires.
    assert_eq!(code_sevs("open $f\n", "W103"), vec!["Warning"]);
}

#[test]
fn w312_registry_taint_list_widens_to_console_and_abbreviations() {
    // `console eval` / `consoleinterp eval|record` declare
    // `taint_interp_eval_subcommands` in the registry, so the same
    // injection check applies — an intentional widening from the former
    // `cmd_name == "interp"` gate.
    assert_eq!(sec_codes("console eval $script\n", "W312"), 1);
    assert_eq!(sec_codes("consoleinterp record $script\n", "W312"), 1);
    assert_eq!(sec_codes("console eval {set x 1}\n", "W312"), 0);
    // The subcommand word resolves via the ensemble rule, matching C Tcl:
    // `interp ev` is the unique-prefix spelling tclsh accepts…
    assert_eq!(sec_codes("interp ev $child $script\n", "W312"), 1);
    // …and `interp e` is ambiguous (eval/exists/expose), a tclsh error, so
    // no diagnostic pretends it dispatched.
    assert_eq!(sec_codes("interp e $child $script\n", "W312"), 0);
}

#[test]
fn w302_fire_and_forget_is_registry_destructive_data() {
    // The suppression reads FIRE_AND_FORGET_TEARDOWN from the registry —
    // deliberately NOT the wider HAS_DESTRUCTIVE_OPS + SubCommand.destructive
    // axis (W313's set): `file mkdir` / `file rename` failures are real
    // errors (permissions, missing source), exactly what W302 asks the
    // caller to capture, so their bare-catch forms keep the hint. Only the
    // teardown idioms — where "target already gone" is the expected,
    // intentionally-ignored failure — suppress.
    assert_eq!(
        sec_codes("proc f {d} { catch {file mkdir $d} }\n", "W302"),
        1
    );
    assert_eq!(
        sec_codes("proc f {a b} { catch {file rename $a $b} }\n", "W302"),
        1
    );
    // Ensemble subcommand words resolve via the registry's unique-prefix
    // rule, matching C Tcl's `Tcl_GetIndexFromObj` (`file del` works in
    // tclsh).
    assert_eq!(sec_codes("proc f {p} { catch {file del $p} }\n", "W302"), 0);
    // Non-destructive subforms of a destructive-capable ensemble still
    // fire.
    assert_eq!(
        sec_codes("proc f {p} { catch {file exists $p} }\n", "W302"),
        1
    );
    assert_eq!(
        sec_codes("proc f {} { catch {array set a {x 1}} }\n", "W302"),
        1
    );
    // Newly stamped bare commands and destructive subforms suppress.
    assert_eq!(sec_codes("proc f {} { catch {unset gone} }\n", "W302"), 0);
    assert_eq!(
        sec_codes("proc f {} { catch {rename foo {}} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {id} { catch {after cancel $id} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {} { catch {namespace delete ::tmp} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {} { catch {dict unset cfg key} }\n", "W302"),
        0
    );
    assert_eq!(
        sec_codes("proc f {c} { catch {interp delete $c} }\n", "W302"),
        0
    );
}

#[test]
fn w303_redos_nested_quantifiers() {
    // Backreferences with repetition or alternatives, in regexp /
    // regsub and a `switch -regexp` braced case list.
    assert_eq!(sec_codes("regexp {^((a+)+)\\1$} $str\n", "W303"), 1);
    assert_eq!(sec_codes("regexp {^((a|aa)+)\\1$} $str\n", "W303"), 1);
    assert_eq!(sec_codes("regsub {^((x*)*)\\1$} $s y out\n", "W303"), 1);
    assert_eq!(
        sec_codes(
            "switch -regexp $x {{^((a+)+)\\1$} {body} default {x}}\n",
            "W303"
        ),
        1
    );
    // Option flags before the pattern are skipped.
    assert_eq!(sec_codes("regexp -nocase {^((a+)+)\\1$} $s\n", "W303"), 1);
    // Anchored nested quantifier still fires.
    assert_eq!(sec_codes("regexp {^((a+)+)\\1$} $x\n", "W303"), 1);
    // Regular patterns without backreferences don't fire.
    assert_eq!(sec_codes("regexp {(a+)+} $str", "W303"), 0);
    assert_eq!(sec_codes("regexp {(a|a)+} $str", "W303"), 0);
    assert_eq!(sec_codes("regexp {abc} $str\n", "W303"), 0);
    assert_eq!(sec_codes("regexp {[0-9]+} $s\n", "W303"), 0);
    assert_eq!(sec_codes(r"regexp {\(a|b\)+} $str", "W303"), 0);
    // FP gate: glob-pattern commands are not regexes — the guard is the
    // spec's `pattern_type == Regex`, so regex-looking text under
    // `string match` / `lsearch` must not fire.
    assert_eq!(sec_codes("string match {^((a+)+)\\1$} $x\n", "W303"), 0);
    assert_eq!(sec_codes("lsearch $l {^((a+)+)\\1$}\n", "W303"), 0);
}

#[test]
fn w303_message_and_severity() {
    let mut a = Analyser::new();
    let r = a.analyse("regexp {^((a+)+)\\1$} $s\n", "tcl8.6");
    let w303 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W303)
        .unwrap();
    assert_eq!(w303.severity, Severity::Warning);
    assert!(w303.message.contains("backreference"), "{w303:?}");
}

#[test]
fn w310_hardcoded_credential_option() {
    // A literal value after a default credential option fires.
    assert_eq!(sec_codes("mycmd -password literalsecret123\n", "W310"), 1);
    assert_eq!(sec_codes("mycmd -token abc123\n", "W310"), 1);
    // Case-insensitive option matching.
    assert_eq!(sec_codes("mycmd -Password hunter2\n", "W310"), 1);
    // Only one diagnostic per command.
    assert_eq!(sec_codes("mycmd -pass a -secret b\n", "W310"), 1);
    // A `$var` / `[cmd]` value is not a hardcoded credential.
    assert_eq!(sec_codes("mycmd -password $env_pw\n", "W310"), 0);
    assert_eq!(sec_codes("mycmd -password [getpw]\n", "W310"), 0);
    // No credential option → nothing.
    assert_eq!(sec_codes("mycmd -name literalvalue\n", "W310"), 0);
}

#[test]
fn w310_message_names_option() {
    let mut a = Analyser::new();
    let r = a.analyse("mycmd -password literalsecret\n", "tcl8.6");
    let w310 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W310)
        .unwrap();
    assert!(
        w310.message
            .starts_with("Hardcoded credential in -password argument."),
        "{w310:?}"
    );
}

#[test]
fn w310_registry_credential_option() {
    // `http::geturl`'s registry `credential_options` adds `-headers`
    // to the default flag set (Strategy 1 augmentation).
    let mut a = Analyser::new();
    let r = a.analyse(
        "package require http\nset url http://example.invalid/\n\
         http::geturl $url -headers {Authorization \"Bearer abc123def456\"}\n",
        "tcl8.6",
    );
    let w310 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W310)
        .unwrap();
    assert!(
        w310.message
            .starts_with("Hardcoded credential in -headers argument."),
        "{w310:?}"
    );
}

#[test]
fn w310_subcommand_sensitive_header() {
    // `HTTP::header insert authorization <literal>` — the subcommand's
    // registry credential_arg + sensitive_headers (Strategy 2).
    let mut a = Analyser::new();
    let r = a.analyse(
        "HTTP::header insert authorization \"Bearer secrettoken123\"\n",
        "f5-irules",
    );
    let w310 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W310)
        .unwrap();
    assert!(
        w310.message
            .starts_with("Hardcoded credential in authorization header value."),
        "{w310:?}"
    );
    // A non-sensitive header is fine; a `$var` value is not literal.
    assert!(
        !a.analyse(
            "HTTP::header insert content-type \"text/html\"\n",
            "f5-irules"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W310)
    );
    assert!(
        !a.analyse("HTTP::header insert authorization $tok\n", "f5-irules")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W310)
    );
}

#[test]
fn w306_literal_expected_in_regexp_pattern() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // The explicit terminator establishes the pattern role independently of its unknown value.
    fn has_w306(src: &str) -> bool {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W306)
    }
    // A quoted `"$var"` / `"${var}"` pattern is byte-for-byte identical at
    // runtime to the bare `$var` idiom — the quotes group nothing — so it is
    // the canonical parameterised-pattern form, not a foot-gun: exempt.
    assert!(!has_w306("regexp -- \"$pat\" $s\n"));
    assert!(!has_w306("regexp -- \"${pat}\" $s\n"));
    // A quoted `"[cmd]"` computes the pattern dynamically — the foot-gun: fires.
    assert!(has_w306("regexp -- \"[clock seconds]\" $s\n"));
    // A quoted var *concatenated* with literal text (`"prefix$pat"`) is no
    // longer a single pure reference — a literal *was* expected there: fires.
    assert!(has_w306("regexp -- \"prefix$pat\" $s\n"));
    // A bare `$var` is the canonical parameterised-pattern idiom — exempt.
    assert!(!has_w306("regexp -- $pat $s\n"));
    // A braced pattern suppresses substitution — exempt.
    assert!(!has_w306("regexp -- {[abc]+} $s\n"));
    // An escaped `\[` in a quoted pattern is a literal regex char — exempt.
    assert!(!has_w306("regexp -- \"\\[abc\\]+\" $s\n"));
    // A bare `[cmd]` pattern is the foot-gun (parsed as command sub) — fires.
    assert!(has_w306("regexp -- [join $parts] $s\n"));
}

#[test]
fn w101_anchors_at_the_substituted_argument() {
    fn w101_span_text(src: &str) -> String {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W101)
            .unwrap_or_else(|| panic!("no W101 in {:?}", r.diagnostics));
        src[d.span.start() as usize..d.span.end() as usize].to_string()
    }
    // The substitution is in the *second* argument — anchor on `$x`, not on
    // the safe literal prefix `"safeprefix"`.
    assert_eq!(w101_span_text("eval \"safeprefix\" $x\n"), "$x");
    // The common single-quoted-string shape still anchors on that string
    // (the substitution is inside it), starting at column 0.
    let mut a = Analyser::new();
    let r = a.analyse("eval \"cmd $a\"\n", "tcl8.6");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W101)
        .unwrap();
    assert_eq!(d.span.start(), 5, "should anchor on the quoted string arg");
}

#[test]
fn w301_list_built_uplevel_script_is_silent_across_lines_2505() {
    let quoted = crate::analyser::Analyser::new().analyse(
        "proc f {x} {\n    uplevel #0 \"puts $x \\\n    \"\n}\n",
        "tcl8.6",
    );
    assert!(
        quoted.diagnostics.iter().any(|d| d.code == DiagCode::W301),
        "an interpolated script in a folded body still warns: {:?}",
        quoted.diagnostics
    );
    for source in [
        "uplevel #0 [list \\\n    puts hello \\\n]\n",
        "uplevel #0 [list puts hello]\n",
        "set x 1\nuplevel #0 [list \\\n    puts $x \\\n]\n",
        "proc f {x} {\n    uplevel #0 [list \\\n        puts $x \\\n    ]\n}\n",
        "proc f {x} {\n    uplevel #0 [list puts $x \\\n    ]\n}\n",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W301),
            "{source:?}: {:?}",
            result.diagnostics
        );
    }
}
