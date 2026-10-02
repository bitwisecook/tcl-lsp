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

//! The probe `tcl spec test` runs in a real shell, and the report it reads
//! back.
//!
//! A pack declares facts about a package's commands, and nothing but running
//! the package can say whether they are true. The probe is one Tcl script,
//! generated from what the pack declares and fed to the shell on standard
//! input; it requires the package, asks each command the questions below, and
//! prints one `SPEC-TEST` line per divergence, which [`parse_report`] reads. It
//! says each command it has finished with, and ends with a `done` line that
//! counts them, so a shell the package stopped is told from one that finished
//! whatever status it exited with. It is written for every shell the pack's
//! package may target, 8.4 to 9.1, so it uses no command a release added since
//! 8.4 and never relies on one's absence.
//!
//! The questions, per command:
//!
//! - **presence** — the package must define the command, and a command it does
//!   not define is one row and no other question;
//! - **arity** — a call with one argument fewer than the declared minimum, and
//!   one more than the declared maximum, must fail with `wrong # args`, and a
//!   call with the minimum, and with the maximum when no step is declared, must
//!   not;
//! - **examples** — each `example` row of the pack's `hover` block must run, and
//!   when the command declares `returns`, its answer must be a value of that
//!   type;
//! - **the reference body** — a command a Tcl body backs is run by that body,
//!   in a child interpreter that has the package, on each example, and must
//!   answer as the command does;
//! - **purity** — a command declared `pure` is run again on each example with a
//!   write trace on every variable of every namespace, and must change none;
//!   the questions above are asked inside the same window, so a first call that
//!   creates a variable is caught too. The probe's own namespace, the shell's own
//!   `::tcl`, where one that reads its script from standard input keeps the
//!   history of every statement it reads, and the global `errorInfo` and
//!   `errorCode`, which the shell writes, are not counted.

use std::fmt::Write as _;

use tcl_registry::TclType;
use tcl_registry::traits::Traits;
use tcl_registry::{BodySource, RuntimeBacking};
use tcl_spectcl::PackSet;
use tcl_syntax::list::list_element;

/// The argument counts a command declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArityProbe {
    /// The fewest arguments.
    pub min: u16,
    /// The most arguments, `None` when unlimited.
    pub max: Option<u16>,
    /// The step between valid counts above the minimum, `0` for none.
    pub step: u16,
}

/// What one command declares that a shell can be asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandProbe {
    /// The command's name, as the pack declares it.
    pub name: String,
    /// The declared argument counts. `None` when the pack declares no limit at
    /// either end, so there is nothing to probe.
    pub arity: Option<ArityProbe>,
    /// Whether the command is declared side-effect free.
    pub pure: bool,
    /// The declared type of the command's answer.
    pub return_type: Option<TclType>,
    /// The `example` rows of the hover block, each one a command line.
    pub examples: Vec<String>,
    /// The `proc` definition a Tcl-body backing names, when the pack has the
    /// text.
    pub reference: Option<String>,
}

/// The commands `set` declares, in pack and declaration order.
#[must_use]
pub fn probes_of(set: &PackSet) -> Vec<CommandProbe> {
    set.packs
        .iter()
        .flat_map(|pack| &pack.commands)
        .map(|command| {
            let spec = command.spec;
            let arity = spec.arity;
            let limited = arity.min > 0 || !arity.is_unlimited();
            CommandProbe {
                name: spec.name.to_owned(),
                arity: limited.then(|| ArityProbe {
                    min: arity.min,
                    max: (!arity.is_unlimited()).then_some(arity.max),
                    step: arity.step,
                }),
                pure: spec.traits.contains(Traits::PURE),
                return_type: spec.return_type,
                examples: spec
                    .hover
                    .as_ref()
                    .map(|hover| {
                        hover
                            .examples
                            .lines()
                            .map(str::trim)
                            .filter(|line| !line.is_empty() && !line.starts_with('#'))
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
                reference: match spec.runtime_backing {
                    RuntimeBacking::TclBody {
                        source: BodySource::PackText { text },
                        ..
                    } => Some(text.to_owned()),
                    RuntimeBacking::TclBody {
                        source: BodySource::PackageSource { .. },
                        ..
                    } => command.reference_text.as_deref().map(str::to_owned),
                    _ => None,
                },
            }
        })
        .collect()
}

/// The package every command of `set` that names one declares with
/// `required_package`, when they all name the same.
#[must_use]
pub fn required_package(set: &PackSet) -> Option<String> {
    let mut names = set
        .packs
        .iter()
        .flat_map(|pack| &pack.commands)
        .filter_map(|command| command.spec.required_package)
        .filter(|name| !name.is_empty());
    let first = names.next()?;
    names.all(|name| name == first).then(|| first.to_owned())
}

/// The shell-side half of the probe: the reporter and one procedure per
/// question. `@PACKAGE@` is the package to require.
const PRELUDE: &str = r#"namespace eval ::__spec_test {}
set ::__spec_test::count 0
proc ::__spec_test::say {kind command detail} {
    set detail [string map [list "\\" "\\\\" "\t" "\\t" "\n" "\\n" "\r" "\\r"] $detail]
    puts "\nSPEC-TEST\t$kind\t$command\t$detail"
}
proc ::__spec_test::asked {name} {
    incr ::__spec_test::count
    ::__spec_test::say asked $name {}
}
proc ::__spec_test::finish {} {
    ::__spec_test::say done - $::__spec_test::count
}
if {[catch {package require @PACKAGE@} ::__spec_test::loaded]} {
    ::__spec_test::say load - $::__spec_test::loaded
    ::__spec_test::finish
    exit 0
}
proc ::__spec_test::words {count} {
    set words {}
    for {set i 0} {$i < $count} {incr i} {lappend words x}
    return $words
}
proc ::__spec_test::attempt {name count} {
    set call [linsert [::__spec_test::words $count] 0 $name]
    set code [catch {uplevel #0 $call} result]
    return [list $code $result $call]
}
proc ::__spec_test::refused {name count} {
    foreach {code result call} [::__spec_test::attempt $name $count] break
    return [list [expr {$code != 0 && [string match "wrong # args*" $result]}] $call]
}
set ::__spec_test::skip 0
proc ::__spec_test::present {name} {
    set ::__spec_test::skip 0
    set full ::[string trimleft $name :]
    if {[llength [info commands $full]] > 0} return
    if {![catch {auto_load $full} loaded] && $loaded} return
    set ::__spec_test::skip 1
    ::__spec_test::say missing $name "declared, but the package does not define it"
}
proc ::__spec_test::arity {name min max step} {
    if {$::__spec_test::skip} return
    if {$min > 0} {
        foreach {refused call} [::__spec_test::refused $name [expr {$min - 1}]] break
        if {!$refused} {
            ::__spec_test::say arity $name "declares at least $min argument(s), but `$call` was not refused as `wrong # args`"
        }
    }
    if {$max >= 0} {
        foreach {refused call} [::__spec_test::refused $name [expr {$max + 1}]] break
        if {!$refused} {
            ::__spec_test::say arity $name "declares at most $max argument(s), but `$call` was not refused as `wrong # args`"
        }
    }
    set inside $min
    if {$max >= 0 && $step == 0 && $max != $min} {lappend inside $max}
    foreach count $inside {
        foreach {refused call} [::__spec_test::refused $name $count] break
        if {$refused} {
            ::__spec_test::say arity $name "declares $count argument(s) valid, but `$call` was refused as `wrong # args`"
        }
    }
}
proc ::__spec_test::is {type value} {
    switch -- $type {
        int { return [expr {[string is integer -strict $value] || [regexp {^[-+]?[0-9]+$} $value]}] }
        double { return [string is double -strict $value] }
        bool { return [string is boolean -strict $value] }
        numeric { return [expr {[::__spec_test::is int $value] || [string is double -strict $value]}] }
        list { return [expr {![catch {llength $value}]}] }
        dict { return [expr {![catch {llength $value}] && [llength $value] % 2 == 0}] }
        default { return 1 }
    }
}
proc ::__spec_test::example {name type line} {
    if {$::__spec_test::skip} return
    if {[catch {uplevel #0 $line} result]} {
        ::__spec_test::say example $name "`$line` raised: $result"
        return
    }
    if {$type ne "" && ![::__spec_test::is $type $result]} {
        ::__spec_test::say returns $name "declares it returns $type, but `$line` answered `$result`"
    }
}
proc ::__spec_test::reference {name text lines} {
    if {$::__spec_test::skip} return
    set child [interp create]
    $child eval [list set ::auto_path $::auto_path]
    if {[catch {$child eval {package require @PACKAGE@}} failure] || [catch {$child eval $text} failure]} {
        ::__spec_test::say reference $name "the reference body could not be run: $failure"
        interp delete $child
        return
    }
    foreach line $lines {
        set command [catch {uplevel #0 $line} commandResult]
        set body [catch {$child eval $line} bodyResult]
        if {$command != $body || $commandResult ne $bodyResult} {
            ::__spec_test::say reference $name "`$line` answers `$commandResult` ($command) as the command and `$bodyResult` ($body) as its reference body"
        }
    }
    interp delete $child
}
proc ::__spec_test::ours {name} {
    set name [string trimleft $name :]
    return [expr {[string match __spec_test* $name] || [lsearch -exact {errorInfo errorCode} $name] >= 0}]
}
proc ::__spec_test::namespaces {ns} {
    set found [list $ns]
    foreach child [namespace children $ns] {
        if {$child eq "::__spec_test" || $child eq "::tcl"} continue
        foreach inner [::__spec_test::namespaces $child] {lappend found $inner}
    }
    return $found
}
proc ::__spec_test::variables {} {
    set names {}
    foreach ns [::__spec_test::namespaces ::] {
        foreach name [info vars [expr {$ns eq "::" ? "::*" : "${ns}::*"}]] {
            if {![::__spec_test::ours $name]} {lappend names $name}
        }
    }
    return [lsort -unique $names]
}
proc ::__spec_test::shown {name} {
    set bare [string trimleft $name :]
    if {[string first :: $bare] < 0} {return $bare}
    return $name
}
proc ::__spec_test::snapshot {} {
    set state {}
    foreach name [::__spec_test::variables] {
        if {[array exists $name]} {
            set pairs {}
            foreach key [lsort [array names $name]] {lappend pairs $key [set ${name}($key)]}
            lappend state $name [list array $pairs]
        } elseif {[info exists $name]} {
            lappend state $name [list scalar [set $name]]
        }
    }
    return $state
}
proc ::__spec_test::changed {before after} {
    array set was $before
    array set now $after
    set names {}
    foreach name [concat [array names was] [array names now]] {
        if {![info exists was($name)] || ![info exists now($name)] || $was($name) ne $now($name)} {
            lappend names $name
        }
    }
    return [lsort -unique $names]
}
proc ::__spec_test::begin {} {
    if {$::__spec_test::skip} return
    set ::__spec_test::mark [::__spec_test::snapshot]
}
proc ::__spec_test::wrote {name args} {
    lappend ::__spec_test::writes $name
}
proc ::__spec_test::pure {name lines} {
    if {$::__spec_test::skip} return
    set reported {}
    foreach line $lines {
        set ::__spec_test::writes {}
        set traced {}
        foreach qualified [::__spec_test::variables] {
            set prefix [list ::__spec_test::wrote $qualified]
            if {![catch {trace add variable $qualified {write unset} $prefix}]} {lappend traced $qualified $prefix}
        }
        catch {uplevel #0 $line}
        foreach {qualified prefix} $traced {
            catch {trace remove variable $qualified {write unset} $prefix}
        }
        set names {}
        foreach written [lsort -unique $::__spec_test::writes] {lappend names [::__spec_test::shown $written]}
        if {[llength $names] > 0} {
            eval lappend reported $names
            ::__spec_test::say pure $name "declared pure, but `$line` wrote the variable(s) $names"
        }
    }
    set names {}
    foreach qualified [::__spec_test::changed $::__spec_test::mark [::__spec_test::snapshot]] {
        set shown [::__spec_test::shown $qualified]
        if {[lsearch -exact $reported $shown] < 0} {lappend names $shown}
    }
    if {[llength $names] > 0} {
        ::__spec_test::say pure $name "declared pure, but running it created or changed the variable(s) $names"
    }
}
"#;

/// The probe for `probes` against `package`: the prelude, then one call per
/// question each command raises.
#[must_use]
pub fn render_script(package: &str, probes: &[CommandProbe]) -> String {
    let mut script = PRELUDE.replace("@PACKAGE@", &list_element(package));
    for probe in probes {
        let name = list_element(&probe.name);
        let _ = writeln!(script, "::__spec_test::present {name}");
        let examples = list_element(
            &probe
                .examples
                .iter()
                .map(|line| list_element(line))
                .collect::<Vec<_>>()
                .join(" "),
        );
        if probe.pure {
            script.push_str("::__spec_test::begin\n");
        }
        if let Some(arity) = probe.arity {
            let max = arity
                .max
                .map_or_else(|| "-1".to_owned(), |max| max.to_string());
            let _ = writeln!(
                script,
                "::__spec_test::arity {name} {} {max} {}",
                arity.min, arity.step
            );
        }
        let kind = probe.return_type.map_or("", |kind| match kind {
            TclType::Int => "int",
            TclType::Double => "double",
            TclType::Boolean => "bool",
            TclType::Numeric => "numeric",
            TclType::List => "list",
            TclType::Dict => "dict",
            _ => "",
        });
        for line in &probe.examples {
            let _ = writeln!(
                script,
                "::__spec_test::example {name} {} {}",
                list_element(kind),
                list_element(line)
            );
        }
        if let Some(text) = &probe.reference {
            let _ = writeln!(
                script,
                "::__spec_test::reference {name} {} {examples}",
                list_element(text)
            );
        }
        if probe.pure {
            let _ = writeln!(script, "::__spec_test::pure {name} {examples}");
        }
        let _ = writeln!(script, "::__spec_test::asked {name}");
    }
    script.push_str("::__spec_test::finish\n");
    script
}

/// One place the shell disagreed with the pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    /// The question: `arity`, `example`, `returns`, `reference`, `pure`,
    /// `missing` for a command the package does not define, or `load` for a
    /// package the shell could not require.
    pub kind: String,
    /// The command the divergence is about; `-` for the package.
    pub command: String,
    /// What the pack declared and what the shell did.
    pub detail: String,
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}: {}", self.command, self.kind, self.detail)
    }
}

/// The divergence the probe reports when the shell cannot require the package.
const UNREQUIRABLE: &str = "load";

/// What a probe reported: the divergences it found, the commands it finished
/// asking about, and the count it ended on when the shell got that far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The divergences, in the order they were found.
    pub divergences: Vec<Divergence>,
    /// The commands the shell finished asking, in the order it asked them.
    pub asked: Vec<String>,
    /// The `done` line's count, which is the last thing a probe says: absent
    /// when the shell stopped first, whatever status it stopped with.
    pub done: Option<usize>,
}

impl Report {
    /// Whether the shell could not require the package, in which case nothing
    /// more was asked of it.
    #[must_use]
    pub fn package_missing(&self) -> bool {
        self.divergences.iter().any(|row| row.kind == UNREQUIRABLE)
    }
}

/// The `SPEC-TEST` lines a probe printed. Anything else the package printed is
/// not part of the report and is ignored.
#[must_use]
pub fn parse_report(output: &str) -> Report {
    let mut report = Report::default();
    for line in output.lines() {
        let Some(rest) = line.strip_prefix("SPEC-TEST\t") else {
            continue;
        };
        let mut fields = rest.splitn(3, '\t');
        let (Some(kind), Some(command), Some(detail)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        match kind {
            "asked" => report.asked.push(command.to_owned()),
            "done" => report.done = detail.parse().ok(),
            _ => report.divergences.push(Divergence {
                kind: kind.to_owned(),
                command: command.to_owned(),
                detail: unescape(detail),
            }),
        }
    }
    report
}

/// Undo the probe's escaping of backslash, tab, newline and carriage return.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(source: &str) -> PackSet {
        tcl_spectcl::pack::load_in_memory(vec![(
            tcl_spectcl::PackFile {
                tier: tcl_spectcl::Tier::Workspace,
                path: std::path::PathBuf::from("demo.tclspec"),
                origin: tcl_spectcl::discovery::Origin::Setting,
                dependency_tier: None,
            },
            source.to_owned(),
        )])
    }

    fn probe(name: &str) -> CommandProbe {
        CommandProbe {
            name: name.to_owned(),
            arity: None,
            pure: false,
            return_type: None,
            examples: Vec::new(),
            reference: None,
        }
    }

    #[test]
    fn a_report_is_the_probes_own_lines_and_nothing_the_package_printed() {
        let output = "hello from the package\n\
                      SPEC-TEST\tarity\tdemo::two\tdeclares at most 2, but `demo::two x x x` was accepted\n\
                      noise SPEC-TEST\tarity\tx\tnot at the start\n\
                      SPEC-TEST\texample\tdemo::two\tline one\\nline two\\ttabbed\\\\slash\n\
                      SPEC-TEST\tasked\tdemo::two\t\n\
                      SPEC-TEST\tdone\t-\t1\n";
        assert_eq!(
            parse_report(output),
            Report {
                divergences: vec![
                    Divergence {
                        kind: "arity".to_owned(),
                        command: "demo::two".to_owned(),
                        detail: "declares at most 2, but `demo::two x x x` was accepted".to_owned(),
                    },
                    Divergence {
                        kind: "example".to_owned(),
                        command: "demo::two".to_owned(),
                        detail: "line one\nline two\ttabbed\\slash".to_owned(),
                    },
                ],
                asked: vec!["demo::two".to_owned()],
                done: Some(1),
            }
        );
        assert_eq!(parse_report(""), Report::default());
    }

    #[test]
    fn a_report_without_its_done_line_is_one_the_shell_did_not_finish() {
        // The package printed a `done` of its own in the middle of a line, which is
        // not the probe's; the real one never came.
        let output = "text SPEC-TEST\tdone\t-\t2\n\
                      SPEC-TEST\tasked\tdemo::one\t\n";
        let report = parse_report(output);
        assert_eq!(report.done, None);
        assert_eq!(report.asked, ["demo::one"]);
        assert_eq!(
            parse_report("SPEC-TEST\tdone\t-\tmany\n").done,
            None,
            "a count that is not a number says nothing"
        );
    }

    #[test]
    fn the_script_asks_each_command_only_what_the_pack_declares() {
        let mut full = probe("demo::full");
        full.arity = Some(ArityProbe {
            min: 1,
            max: Some(2),
            step: 0,
        });
        full.pure = true;
        full.return_type = Some(TclType::Int);
        full.examples = vec!["demo::full 1".to_owned(), "demo::full {a b}".to_owned()];
        full.reference = Some("proc demo::full {x} {return $x}".to_owned());
        let script = render_script("demo", &[full, probe("demo::bare")]);
        assert!(script.contains("package require demo}"));
        assert!(
            script.contains("::__spec_test::arity demo::full 1 2 0\n"),
            "{script}"
        );
        assert!(script.contains("::__spec_test::example demo::full int {demo::full 1}\n"));
        assert!(script.contains("::__spec_test::example demo::full int {demo::full {a b}}\n"));
        assert!(script.contains(
            "::__spec_test::reference demo::full {proc demo::full {x} {return $x}} {{demo::full 1} {demo::full {a b}}}\n"
        ));
        assert!(
            script.contains("::__spec_test::pure demo::full {{demo::full 1} {demo::full {a b}}}\n")
        );
        // A command declared pure is marked before its first question, so a write
        // the first call makes is seen and not only a repeated one.
        let begin = script.find("::__spec_test::begin\n").expect("marked");
        let first = script
            .find("::__spec_test::arity demo::full")
            .expect("asked");
        assert!(begin < first, "{script}");
        assert_eq!(
            script.matches("::__spec_test::begin\n").count(),
            1,
            "{script}"
        );
        assert!(
            !script.contains("::__spec_test::arity demo::bare")
                && !script.contains("::__spec_test::example demo::bare")
                && !script.contains("::__spec_test::pure demo::bare"),
            "a command that declares nothing is asked nothing"
        );
        assert!(
            script.contains("::__spec_test::present demo::bare\n")
                && script.contains("::__spec_test::present demo::full\n"),
            "but the package must define it: {script}"
        );
        // Each command says it is finished with, in order, and the script ends
        // with the count: a shell that stops anywhere before that line is known
        // to have.
        let finished: Vec<&str> = script
            .lines()
            .filter(|line| line.starts_with("::__spec_test::asked "))
            .collect();
        assert_eq!(
            finished,
            [
                "::__spec_test::asked demo::full",
                "::__spec_test::asked demo::bare"
            ]
        );
        assert!(script.ends_with("::__spec_test::finish\n"), "{script}");
        // An unlimited maximum is the sentinel the probe skips.
        let mut open = probe("demo::open");
        open.arity = Some(ArityProbe {
            min: 2,
            max: None,
            step: 2,
        });
        assert!(
            render_script("demo", &[open]).contains("::__spec_test::arity demo::open 2 -1 2\n")
        );
    }

    #[test]
    fn what_the_pack_declares_reaches_the_probe() {
        let source = "speclib demo 2.0 {\n    command demo::two {\n        arity 1..2\n        \
                      required_package demo\n        traits {PURE}\n        return_type Int\n        \
                      hover {\n            summary {Two.}\n            example {demo::two 1}\n            \
                      example {# a comment}\n            example {demo::two 1 2}\n        }\n    }\n    \
                      command demo::open {\n        arity 0..\n        required_package demo\n    }\n}\n";
        let set = loaded(source);
        let probes = probes_of(&set);
        let two = probes
            .iter()
            .find(|p| p.name == "demo::two")
            .expect("declared");
        assert_eq!(
            two.arity,
            Some(ArityProbe {
                min: 1,
                max: Some(2),
                step: 0
            })
        );
        assert_eq!(two.examples, vec!["demo::two 1", "demo::two 1 2"]);
        assert_eq!(two.return_type, Some(TclType::Int));
        assert!(two.pure);
        let open = probes
            .iter()
            .find(|p| p.name == "demo::open")
            .expect("declared");
        assert_eq!(open.arity, None, "no limit at either end: nothing to probe");
        assert_eq!(required_package(&set).as_deref(), Some("demo"));
    }

    #[test]
    fn the_package_is_the_one_every_command_that_names_one_agrees_on() {
        let pack = |second: &str| {
            loaded(&format!(
                "speclib demo 2.0 {{\n    command demo::a {{\n        arity 1\n        \
                 required_package alpha\n    }}\n    command demo::b {{\n        arity 1\n        \
                 required_package {second}\n    }}\n    command demo::c {{\n        arity 1\n    }}\n}}\n"
            ))
        };
        assert_eq!(required_package(&pack("alpha")).as_deref(), Some("alpha"));
        assert_eq!(
            required_package(&pack("beta")),
            None,
            "two packages are none: the author names one"
        );
    }
}
