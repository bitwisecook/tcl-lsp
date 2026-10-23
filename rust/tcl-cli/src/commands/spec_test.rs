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
//! - **purity** — a command declared `pure` is run again on each example with
//!   write/unset traces on namespace variables, and must change none. Execution
//!   steps reveal new variables visible between Tcl commands. The questions
//!   above share the same trace window. The probe's own namespace, the shell's own
//!   `::tcl`, where one that reads its script from standard input keeps the
//!   history of every statement it reads, and the global `errorInfo` and
//!   `errorCode`, which the shell writes, are not counted. Unsupported namespace
//!   addresses or interpreter queries are reported as unavailable purity checks.

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
proc ::__spec_test::present {name cpattern jimpattern supported} {
    set ::__spec_test::skip 0
    if {[catch {::__tcl_name_presence::in_root $cpattern $jimpattern $supported} present]} {
        set ::__spec_test::skip 1
        ::__spec_test::say unsupported $name "command presence unavailable: $present"
        return
    }
    if {$present} return
    if {![catch {uplevel #0 [list auto_load $name]} loaded] && $loaded} return
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
proc ::__spec_test::variables {} {
    set variables {}
    foreach coordinate [::__tcl_name_snapshot::variables {::__spec_test ::tcl}] {
        foreach {ns name} $coordinate break
        if {$ns eq "::" && [lsearch -exact {errorInfo errorCode} $name] >= 0} continue
        lappend variables $coordinate
    }
    return $variables
}
proc ::__spec_test::snapshot {} {
    set state {}
    foreach coordinate [::__spec_test::variables] {
        if {[::__tcl_name_snapshot::call $coordinate {::array exists}]} {
            set pairs {}
            foreach key [lsort [::__tcl_name_snapshot::call $coordinate {::array names}]] {
                lappend pairs $key [::__tcl_name_snapshot::element_value $coordinate $key]
            }
            lappend state $coordinate [list array $pairs]
        } elseif {[::__tcl_name_snapshot::call $coordinate {::info exists}]} {
            lappend state $coordinate [list scalar [::__tcl_name_snapshot::call $coordinate {::set}]]
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
    set ::__spec_test::snapshot_ready 0
    set ::__spec_test::snapshot_reason {}
    if {$::__spec_test::skip} return
    set ::__spec_test::writes {}
    catch {unset ::__spec_test::traced}
    array set ::__spec_test::traced {}
    if {[catch {::__spec_test::snapshot} state]} {
        set ::__spec_test::snapshot_reason $state
        return
    }
    set ::__spec_test::mark $state
    if {[catch {::__spec_test::watch 0} reason]} {
        set ::__spec_test::snapshot_reason $reason
        ::__spec_test::end
        return
    }
    if {[catch {
        foreach helper {attempt example reference pure} {
            trace add execution ::__spec_test::$helper leavestep ::__spec_test::step
        }
    } reason]} {
        set ::__spec_test::snapshot_reason $reason
        ::__spec_test::end
        return
    }
    set ::__spec_test::snapshot_ready 1
}
proc ::__spec_test::wrote {coordinate name1 name2 op} {
    lappend ::__spec_test::writes $coordinate
    if {$op eq "unset"} {
        if {[catch {::__tcl_name_snapshot::call $coordinate {::array exists}} present] || !$present} {
            catch {unset ::__spec_test::traced($coordinate)}
        }
    }
}
proc ::__spec_test::watch {created} {
    foreach coordinate [::__spec_test::variables] {
        if {[info exists ::__spec_test::traced($coordinate)]} continue
        set prefix [list ::__spec_test::wrote $coordinate]
        ::__tcl_name_snapshot::call $coordinate {::trace add variable} [list {write unset} $prefix]
        set ::__spec_test::traced($coordinate) $prefix
        if {$created} {lappend ::__spec_test::writes $coordinate}
    }
}
proc ::__spec_test::step {args} {
    if {!$::__spec_test::snapshot_ready} return
    if {[catch {::__spec_test::watch 1} reason]} {
        set ::__spec_test::snapshot_ready 0
        set ::__spec_test::snapshot_reason $reason
    }
}
proc ::__spec_test::end {} {
    foreach helper {attempt example reference pure} {
        catch {trace remove execution ::__spec_test::$helper leavestep ::__spec_test::step}
    }
    if {![array exists ::__spec_test::traced]} return
    foreach coordinate [array names ::__spec_test::traced] {
        catch {::__tcl_name_snapshot::call $coordinate {::trace remove variable} [list {write unset} $::__spec_test::traced($coordinate)]}
    }
}
proc ::__spec_test::pure {name lines} {
    if {$::__spec_test::skip} return
    if {!$::__spec_test::snapshot_ready} {
        ::__spec_test::end
        ::__spec_test::say unsupported $name "purity snapshot unavailable: $::__spec_test::snapshot_reason"
        return
    }
    set earlier $::__spec_test::writes
    set reported {}
    foreach line $lines {
        set ::__spec_test::writes {}
        catch {uplevel #0 $line}
        set names {}
        foreach written [lsort -unique $::__spec_test::writes] {lappend names [::__tcl_name_snapshot::shown $written]}
        if {[llength $names] > 0} {
            eval lappend reported $names
            ::__spec_test::say pure $name "declared pure, but `$line` wrote the variable(s) $names"
        }
    }
    ::__spec_test::end
    if {!$::__spec_test::snapshot_ready} {
        ::__spec_test::say unsupported $name "purity snapshot unavailable: $::__spec_test::snapshot_reason"
        return
    }
    if {[catch {::__spec_test::snapshot} after]} {
        ::__spec_test::say unsupported $name "purity snapshot unavailable: $after"
        return
    }
    set names {}
    set changed [::__spec_test::changed $::__spec_test::mark $after]
    foreach coordinate [lsort -unique [concat $earlier $changed]] {
        set shown [::__tcl_name_snapshot::shown $coordinate]
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
    let mut script =
        String::from(tcl_syntax::naming::original_namespace_variable_snapshot_support());
    script.push_str(tcl_syntax::naming::original_script_name_query_support());
    script.push_str(tcl_syntax::naming::original_command_presence_support());
    script.push_str(&PRELUDE.replace("@PACKAGE@", &list_element(package)));
    for probe in probes {
        let name = list_element(&probe.name);
        let plan = tcl_syntax::naming::ScriptRootCommandPresencePlan::new(&probe.name);
        let (c_pattern, jim_pattern, supported) = plan.as_ref().map_or_else(
            || (list_element(""), list_element(""), 0),
            |plan| {
                (
                    list_element(plan.c_pattern()),
                    list_element(plan.jim_pattern()),
                    1,
                )
            },
        );
        let _ = writeln!(
            script,
            "::__spec_test::present {name} {c_pattern} {jim_pattern} {supported}"
        );
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
    /// `missing` for an absent command, `load` for an unavailable package, or
    /// `unsupported` when the interpreter cannot provide a purity snapshot.
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
            script.contains("::__spec_test::present demo::bare demo::bare demo::bare 1\n")
                && script.contains("::__spec_test::present demo::full demo::full demo::full 1\n"),
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
    const MUTATING_FIXTURE: &str = r"namespace eval ::demo {}
namespace eval {::N[0]} {variable x BEFORE; variable a; array set a {:key BEFORE}}
namespace eval ::N0 {variable y SIBLING}
set {:errorInfo} BEFORE
set ::__spec_testOther BEFORE
proc ::demo::mutate {} {
    ::uplevel #0 [list ::set :errorInfo AFTER]
    ::set ::__spec_testOther AFTER
    ::set {::N[0]::x} AFTER
    ::set {::N[0]::a(:key)} AFTER
    ::return OK
}
package provide demo 1.0
";

    const REFUSAL_FIXTURE: &str = r"namespace eval ::demo {}
namespace eval {:N} {variable x COLON_NAMESPACE}
namespace eval ::N {variable x ORDINARY_NAMESPACE}
proc ::demo::quiet {} {return OK}
package provide demo 1.0
";

    const SHADOW_FIXTURE: &str = r"namespace eval ::demo {}
namespace eval ::shadow {
    variable x BEFORE
    proc namespace args {error {SHADOW namespace}}
    proc info args {error {SHADOW info}}
    proc set args {error {SHADOW set}}
    proc array args {error {SHADOW array}}
    proc trace args {error {SHADOW trace}}
}
proc ::demo::mutate {} {::set ::shadow::x AFTER; ::return OK}
package provide demo 1.0
";

    fn original_purity_report(
        interpreter: &std::path::Path,
        fixture: &str,
        command: &str,
    ) -> Report {
        let mut selected = probe(command);
        selected.pure = true;
        selected.examples.push(command.to_owned());
        let script = format!("{fixture}\n{}", render_script("demo", &[selected]));
        let outcome = tcl_test_support::run_script(interpreter, script.as_bytes())
            .expect("original interpreter must execute the probe");
        let text = outcome.strict_text().unwrap_or_else(|reason| {
            panic!("{}: {reason}", interpreter.display());
        });
        let report = parse_report(&text);
        assert_eq!(report.done, Some(1), "{text}");
        assert_eq!(report.asked, [command], "{text}");
        report
    }

    #[test]
    fn original_purity_snapshots_keep_colon_root_and_bracket_namespace_operands() {
        for interpreter in tcl_test_support::available_tclshs() {
            let report =
                original_purity_report(&interpreter.path, MUTATING_FIXTURE, "demo::mutate");
            assert_eq!(report.divergences.len(), 1, "{report:?}");
            let divergence = &report.divergences[0];
            assert_eq!(divergence.kind, "pure", "{report:?}");
            for variable in [":errorInfo", "__spec_testOther", "::N[0]::a", "::N[0]::x"] {
                assert!(divergence.detail.contains(variable), "{report:?}");
            }
            assert!(!divergence.detail.contains("::N0::y"), "{report:?}");
        }
    }

    #[test]
    fn original_purity_snapshots_refuse_ambiguous_namespace_reports() {
        for interpreter in tcl_test_support::available_tclshs() {
            let report = original_purity_report(&interpreter.path, REFUSAL_FIXTURE, "demo::quiet");
            assert_eq!(report.divergences.len(), 1, "{report:?}");
            assert_eq!(report.divergences[0].kind, "unsupported", "{report:?}");
            assert!(
                report.divergences[0]
                    .detail
                    .contains("namespace report has no unambiguous snapshot address"),
                "{report:?}"
            );
        }
    }

    #[test]
    fn original_purity_snapshot_operations_do_not_use_namespace_command_shadows() {
        for interpreter in tcl_test_support::available_tclshs() {
            let report = original_purity_report(&interpreter.path, SHADOW_FIXTURE, "demo::mutate");
            assert_eq!(report.divergences.len(), 1, "{report:?}");
            assert_eq!(report.divergences[0].kind, "pure", "{report:?}");
            assert!(
                report.divergences[0].detail.contains("::shadow::x"),
                "{report:?}"
            );
            assert!(
                !report.divergences[0].detail.contains("SHADOW"),
                "{report:?}"
            );
        }
    }

    #[test]
    fn original_jim_purity_probe_reports_unavailable_namespace_enumeration() {
        let interpreter = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
            Some(tcl_test_support::require_jimsh().expect("required original Jim oracle"))
        } else {
            tcl_test_support::locate_jimsh().expect("valid optional original Jim oracle")
        };
        let Some(interpreter) = interpreter else {
            eprintln!("skipping original Jim snapshot control: no Jim oracle");
            return;
        };
        let fixture = "namespace eval ::demo {}\nproc ::demo::quiet {} {return OK}\npackage provide demo 1.0\n";
        let report = original_purity_report(&interpreter.path, fixture, "demo::quiet");
        assert_eq!(report.divergences.len(), 1, "{report:?}");
        assert_eq!(report.divergences[0].kind, "unsupported", "{report:?}");
        assert!(
            report.divergences[0].detail.contains("children"),
            "{report:?}"
        );
    }

    const PRESENCE_FIXTURE: &str = r"
foreach ns {::N[0] ::N0 ::Nmissing ::W* ::WZ ::Q? ::Qa {::B\slash}} {::namespace eval $ns {}}
foreach name {::N[0]::p ::N0::p ::Nmissing::absent ::W*::p ::WZ::p ::Q?::p ::Qa::p {::B\slash::p} ::N[0]::p* ::N[0]::pZ ::N[0]::p? ::N[0]::pA ::N[0]::p[0] ::N[0]::p0 {::N[0]::p\tail} ::N[0]:: ::N[0]::absentZ :p p* pZ absentZ} {
    ::proc $name {} {::return EXACT}
}
package provide demo 1.0
";

    fn original_presence_report(interpreter: &std::path::Path) {
        let names = [
            "::N[0]::p",
            "::W*::p",
            "::Q?::p",
            "::B\\slash::p",
            "::N[0]::p*",
            "::N[0]::p?",
            "::N[0]::p[0]",
            "::N[0]::p\\tail",
            "::N[0]::",
            ":p",
            "p*",
            "::N[missing]::absent",
            "::N[0]::absent*",
            "absent*",
        ];
        let probes: Vec<_> = names.iter().map(|name| probe(name)).collect();
        let script = format!("{PRESENCE_FIXTURE}\n{}", render_script("demo", &probes));
        let outcome = tcl_test_support::run_script(interpreter, script.as_bytes())
            .expect("original interpreter must execute exact presence questions");
        let text = outcome.strict_text().expect("original presence protocol");
        let report = parse_report(&text);
        assert_eq!(report.done, Some(names.len()), "{text}");
        assert_eq!(report.asked, names, "{text}");
        assert_eq!(report.divergences.len(), 3, "{text}");
        for (divergence, missing) in report.divergences.iter().zip(&names[11..]) {
            assert_eq!(divergence.kind, "missing", "{text}");
            assert_eq!(divergence.command, *missing, "{text}");
        }
    }

    #[test]
    fn original_c_presence_keeps_namespace_prefix_literal_and_simple_tail_exact() {
        for interpreter in tcl_test_support::available_tclshs() {
            original_presence_report(&interpreter.path);
        }
    }

    #[test]
    fn original_jim_presence_uses_flat_inventory_without_lone_colon_which() {
        let interpreter = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
            Some(tcl_test_support::require_jimsh().expect("required original Jim oracle"))
        } else {
            tcl_test_support::locate_jimsh().expect("valid optional original Jim oracle")
        };
        let Some(interpreter) = interpreter else {
            eprintln!("skipping original Jim presence control: no Jim oracle");
            return;
        };
        original_presence_report(&interpreter.path);
    }
}
