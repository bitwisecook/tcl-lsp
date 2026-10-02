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

//! The probe `tcl spec import --probe PACKAGE` runs in a real shell, and the
//! report it reads back.
//!
//! A package a C extension provides names its commands nowhere a Tcl script can
//! read, so the one witness is a shell that requires it: the probe lists every
//! command of every namespace, requires the package, lists them again and
//! prints the difference. A shell's first `package require` loads the library
//! commands that look the package up (`tclPkgUnknown`, `::tcl::tm::*`), which are
//! the shell's and not the package's, so the probe makes that require of a name
//! that does not exist before it lists anything. It is one Tcl script on the
//! shell's standard input, written for every release from 8.4 to 9.1, and it ends
//! with a `done` line that counts the commands it printed, so a package that
//! stopped the shell part way is told from one that finished.

use std::fmt::Write as _;

/// The Tcl script that requires `package` and reports what it added.
#[must_use]
pub fn render_script(package: &str) -> String {
    let mut script = String::new();
    script.push_str(
        r#"namespace eval ::__spec_probe {}
proc ::__spec_probe::say {kind a b} {
    set a [string map [list \\ \\\\ \t \\t \n \\n \r \\r] $a]
    set b [string map [list \\ \\\\ \t \\t \n \\n \r \\r] $b]
    puts "\nSPEC-PROBE\t$kind\t$a\t$b"
}
proc ::__spec_probe::commands {} {
    set found {}
    set todo ::
    while {[llength $todo]} {
        set ns [lindex $todo 0]
        set todo [lrange $todo 1 end]
        if {$ns eq "::__spec_probe"} continue
        if {$ns eq "::"} {set pattern ::*} else {set pattern ${ns}::*}
        foreach name [info commands $pattern] {lappend found $name}
        foreach child [namespace children $ns] {lappend todo $child}
    }
    return $found
}
catch {package require __spec_probe_warmup}
array set ::__spec_probe::before {}
foreach name [::__spec_probe::commands] {set ::__spec_probe::before($name) 1}
"#,
    );
    let _ = write!(
        script,
        "if {{[catch {{package require {package}}} version]}} {{\n    \
         ::__spec_probe::say failure {{}} $version\n    \
         ::__spec_probe::say done 0 {{}}\n    exit 0\n}}\n"
    );
    script.push_str(
        r"::__spec_probe::say provided {} $version
set ::__spec_probe::count 0
foreach name [lsort [::__spec_probe::commands]] {
    if {![info exists ::__spec_probe::before($name)]} {
        incr ::__spec_probe::count
        ::__spec_probe::say added $name {}
    }
}
::__spec_probe::say done $::__spec_probe::count {}
",
    );
    script
}

/// What the shell reported.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProbeOutput {
    /// The version the package provided, when it was required.
    pub version: Option<String>,
    /// Why the package could not be required, when it could not.
    pub error: Option<String>,
    /// The commands the package added, qualified, in the order printed.
    pub commands: Vec<String>,
    /// How many commands the shell said it printed, when it got as far as
    /// saying.
    pub done: Option<usize>,
}

impl ProbeOutput {
    /// Whether the shell finished: it said how many commands it printed and
    /// every one of them arrived.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.done == Some(self.commands.len())
    }
}

/// Read the `SPEC-PROBE` lines the shell printed; anything else the package
/// printed is ignored.
#[must_use]
pub fn parse_output(output: &str) -> ProbeOutput {
    let mut parsed = ProbeOutput::default();
    for line in output.lines() {
        let Some(rest) = line.strip_prefix("SPEC-PROBE\t") else {
            continue;
        };
        let mut fields = rest.splitn(3, '\t');
        let (Some(kind), Some(first), Some(second)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        match kind {
            "provided" => parsed.version = Some(unescape(second)),
            "failure" => parsed.error = Some(unescape(second)),
            "added" => parsed.commands.push(unescape(first)),
            "done" => parsed.done = first.parse().ok(),
            _ => {}
        }
    }
    parsed
}

/// Whether `name` can be written into the probe script as a package name: the
/// characters a package name is made of, and nothing a Tcl word could treat as
/// syntax.
#[must_use]
pub fn is_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '+'))
}

/// Whether `name` can sit in a pack as a command name: no whitespace and no
/// control character, which a package could use to print a line of its own.
#[must_use]
pub fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| !c.is_whitespace() && !c.is_control())
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

    #[test]
    fn the_report_is_the_lines_the_probe_printed() {
        let output = "banner the package printed\n\
                      \n\
                      SPEC-PROBE\tprovided\t\t1.2\n\
                      SPEC-PROBE\tadded\t::pkga_eq\t\n\
                      noise\n\
                      SPEC-PROBE\tadded\t::ns::calc\t\n\
                      SPEC-PROBE\tdone\t2\t\n";
        let parsed = parse_output(output);
        assert_eq!(parsed.version.as_deref(), Some("1.2"));
        assert_eq!(parsed.commands, ["::pkga_eq", "::ns::calc"]);
        assert!(parsed.finished());
        assert!(parsed.error.is_none());
    }

    #[test]
    fn a_shell_that_stopped_before_it_said_done_has_not_finished() {
        let parsed = parse_output("SPEC-PROBE\tadded\t::a\t\n");
        assert!(!parsed.finished());
        let short = parse_output("SPEC-PROBE\tadded\t::a\t\nSPEC-PROBE\tdone\t2\t\n");
        assert!(!short.finished(), "it said two and only one arrived");
    }

    #[test]
    fn a_package_that_cannot_be_required_reports_why() {
        let parsed =
            parse_output("SPEC-PROBE\tfailure\t\tcan't find package nope\nSPEC-PROBE\tdone\t0\t\n");
        assert_eq!(parsed.error.as_deref(), Some("can't find package nope"));
        assert!(parsed.finished());
    }

    #[test]
    fn a_name_with_whitespace_or_a_control_character_is_not_a_command_name() {
        assert!(is_plain_name("::ns::cmd"));
        assert!(!is_plain_name("two words"));
        assert!(!is_plain_name("tab\tname"));
        assert!(!is_plain_name("esc\u{1b}ape"));
        assert!(!is_plain_name(""));
    }

    #[test]
    fn a_package_name_is_never_tcl_syntax() {
        assert!(is_package_name("pkga"));
        assert!(is_package_name("Tcl-thread_2.1+x"));
        for hostile in [
            "", "a b", "a}b", "a[b]", "a$b", "a;b", "a\nb", "a\\b", "a\"b",
        ] {
            assert!(!is_package_name(hostile), "{hostile:?}");
        }
    }

    #[test]
    fn the_script_requires_the_package_between_two_listings() {
        let script = render_script("pkga");
        let warmup = script
            .find("package require __spec_probe_warmup")
            .expect("warmup");
        let before = script.find("::__spec_probe::before").expect("before");
        let require = script.find("package require pkga").expect("require");
        let report = script.find("say added").expect("report");
        assert!(warmup < before && before < require && require < report);
        assert!(script.contains("say done"));
    }

    #[test]
    fn the_probe_escapes_are_undone() {
        let parsed = parse_output("SPEC-PROBE\tadded\t::a\\tb\\nc\\\\d\\re\t\n");
        assert_eq!(parsed.commands, ["::a\tb\nc\\d\re"]);
    }
}
