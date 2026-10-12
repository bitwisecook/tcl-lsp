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

//! The measured split of iRules' 31 "disabled" stock-Tcl builtins into
//! its **two distinct mechanisms** — environment/realm policy data for
//! the `f5-irules` environment
//! (`docs/design/f5/bigip-irule-parser-measurements.md` §4b, re-probed
//! through `eval` at runtime on BIG-IP 21.1.0.1).
//!
//! A literal reference to any of the 31 is refused when the rule is
//! **loaded** (`command is disabled: "X"`, §5), which is why none of them
//! carries the `IRULES` bit in its command spec. But the refusals are not
//! one fact:
//!
//! - **16 are absent from TMM's interpreter** — `invalid command name`
//!   even when reached through `eval` at runtime. A smaller interpreter
//!   build: a *language fact* about what exists in TMM.
//! - **15 are present in the interpreter and refused only by the rule
//!   compiler** — reachable via `eval`, and `rename` demonstrably works.
//!   Pure load-time policy about rule *source*: the commands are right
//!   there, and only the compiler's opinion stops you.
//!
//! The distinction pins diagnostic severity: an interpreter-absent
//! command is unconditionally unavailable (error-grade, like any unknown
//! command under the closed world), while a compiler-refused one is a
//! **policy warning about rule source** — the same name reached through
//! dynamic evaluation is real, so an analyser that follows §4c's dynamic
//! code must not claim the command does not exist.
//!
//! measurements §4b: this module is the data layer only. The consumer
//! wiring lives in the analyser: an interpreter-absent literal head
//! keeps the language-fact unavailable-command diagnostic (W002), a
//! compiler-refused literal head draws the distinct IRULE2004 policy
//! warning instead (and is excluded from "Unknown command" claims),
//! and the §4c lexical-scan mirroring recurses the load-time checks
//! through braced `eval`/`uplevel` literals while a variable-held
//! script widens the realm state (`tcl-compiler`'s
//! `analyser::diagnostics::validity` / `analyser::commands`).

/// Which of the two measured mechanisms keeps one stock builtin out of
/// iRules source (§4b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrulesDisabledClass {
    /// Absent from TMM's interpreter: `invalid command name` even via
    /// `eval` at runtime — a **language fact** (the smaller interpreter
    /// build), diagnosable as an error.
    InterpreterAbsent,
    /// Present in TMM's interpreter but refused by the rule compiler at
    /// load: reachable via `eval` at runtime — a **rule-source policy**,
    /// diagnosable as a policy warning, never as "this command does not
    /// exist".
    CompilerRefused,
}

impl IrulesDisabledClass {
    /// The diagnostic severity class the mechanism pins: `true` when the
    /// finding is a statement about the language (error-grade), `false`
    /// when it is a policy statement about rule source (warning-grade).
    #[must_use]
    pub const fn is_language_fact(self) -> bool {
        matches!(self, Self::InterpreterAbsent)
    }
}

/// The 16 commands **absent from TMM's interpreter** (measurements §4b:
/// `invalid command name` even through `eval`).
pub const IRULES_INTERPRETER_ABSENT: &[&str] = &[
    "auto_execok",
    "auto_import",
    "auto_load",
    "auto_qualify",
    "cd",
    "exec",
    "exit",
    "fconfigure",
    "file",
    "glob",
    "load",
    "open",
    "pwd",
    "socket",
    "source",
    "unknown",
];

/// The 15 commands **present in TMM's interpreter but refused by the rule
/// compiler** at load (measurements §4b: reachable via `eval`; `rename`
/// demonstrably works).
pub const IRULES_COMPILER_REFUSED: &[&str] = &[
    "eof",
    "fblocked",
    "fcopy",
    "flush",
    "gets",
    "interp",
    "namespace",
    "package",
    "pid",
    "rename",
    "seek",
    "tell",
    "time",
    "update",
    "vwait",
];

/// The mechanism that keeps `command` out of iRules source, or `None`
/// when the command is not one of the 31 measured "disabled" builtins.
#[must_use]
pub fn irules_disabled_class(command: &str) -> Option<IrulesDisabledClass> {
    if IRULES_INTERPRETER_ABSENT.contains(&command) {
        Some(IrulesDisabledClass::InterpreterAbsent)
    } else if IRULES_COMPILER_REFUSED.contains(&command) {
        Some(IrulesDisabledClass::CompilerRefused)
    } else {
        None
    }
}

/// The measured interpreter table adds these native commands to the positive
/// rule-source surface. It does not admit the full ancestor command set.
#[must_use]
pub fn runtime_surface_admits(
    spec: &crate::CommandSpec,
    query: tcl_dialect::model::SurfaceQuery<'_>,
) -> bool {
    use tcl_dialect::model::{Family, InvocationRealm};
    if query.realm != InvocationRealm::InterpreterRuntime
        || query
            .core
            .nearest()
            .is_none_or(|(family, _)| family != Family::F5Irules)
    {
        return spec.supports_dialect(Some(query));
    }
    let loader = query.with_realm(InvocationRealm::RuleLoader);
    spec.supports_dialect(Some(loader))
        || (irules_disabled_class(spec.name) == Some(IrulesDisabledClass::CompilerRefused)
            && spec.owning_package().is_none()
            && spec.supports_dialect(Some(query)))
}

/// Whether the measured native command is subject to authored-source refusal
/// at this phase. Runtime availability remains a separate question.
#[must_use]
pub fn rule_loader_refuses(command: &str, realm: tcl_dialect::model::InvocationRealm) -> bool {
    realm == tcl_dialect::model::InvocationRealm::RuleLoader
        && irules_disabled_class(command) == Some(IrulesDisabledClass::CompilerRefused)
}

/// Explicit appliance measurement selection for original rule-source loading.
/// It issues no physical Tcl implementation, parser, ABI or compiler capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeasuredIrulesLoaderProfile {
    /// BIG-IP 21.1.0.1, build 0.0.26, Point Release 1, one group/four TMMs.
    BigIp21_1_0_1Build0_0_26,
}

/// Build-scoped rule-source findings, independent of runtime value support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeasuredIrulesSourceRefusal {
    /// The measured configuration parser rejects an original literal NUL byte.
    LiteralNulRejected,
    /// This exact original non-ASCII source was rejected by the measured loader.
    NonAsciiRejected,
    /// Original non-ASCII source has no supported accepted loader domain here.
    /// This does not claim rejection of every Unicode source or normalization.
    NonAsciiUnsupported,
}

impl MeasuredIrulesLoaderProfile {
    /// Classify original rule-source bytes at the explicitly selected loader.
    /// Runtime values and native parser/ABI capabilities remain independent.
    #[must_use]
    pub fn original_source_refusal(self, source: &[u8]) -> Option<MeasuredIrulesSourceRefusal> {
        match self {
            Self::BigIp21_1_0_1Build0_0_26 => {
                if source.contains(&0) {
                    Some(MeasuredIrulesSourceRefusal::LiteralNulRejected)
                } else if let Some(accepted) = measured_utf8_source_outcome(source) {
                    (!accepted).then_some(MeasuredIrulesSourceRefusal::NonAsciiRejected)
                } else if !source.is_ascii() {
                    Some(MeasuredIrulesSourceRefusal::NonAsciiUnsupported)
                } else {
                    None
                }
            }
        }
    }
}

const MEASURED_UTF8_SOURCES: &[(&[u8], bool)] = &[
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_bmp_copyright.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_bmp_heart.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_bmp_snowman.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_decomposed.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_family.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_flag.tcl"),
        true,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_grinning.tcl"),
        true,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_keycap.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_rainbow_flag.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_skin_tone.tcl"),
        true,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_text_vs.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_emoji_zwj.tcl"),
        false,
    ),
    (
        include_bytes!("f5/measured_rule_sources/unicode_literal_precomposed.tcl"),
        false,
    ),
];

fn measured_utf8_source_outcome(source: &[u8]) -> Option<bool> {
    MEASURED_UTF8_SOURCES
        .iter()
        .find_map(|(original, accepted)| (*original == source).then_some(*accepted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, SurfaceQuery, surface_admits};

    #[test]
    fn measured_loader_bytes_do_not_define_runtime_name_or_object_policy() {
        let profile = MeasuredIrulesLoaderProfile::BigIp21_1_0_1Build0_0_26;
        for source in [
            b"when HTTP_REQUEST {set x A\0B}".as_slice(),
            "when HTTP_REQUEST {set x é}".as_bytes(),
            "when HTTP_REQUEST {set x e\u{301}}".as_bytes(),
        ] {
            assert!(profile.original_source_refusal(source).is_some());
        }
        assert!(
            profile
                .original_source_refusal(b"when HTTP_REQUEST {set x [binary format H* 410042]}")
                .is_none()
        );
        assert!(
            profile
                .original_source_refusal(
                    b"when HTTP_REQUEST {set x {A\\
B}}"
                )
                .is_none()
        );
    }

    #[test]
    fn measured_utf8_loader_outcomes_require_the_complete_attested_source() {
        let profile = MeasuredIrulesLoaderProfile::BigIp21_1_0_1Build0_0_26;
        for &(source, accepted) in MEASURED_UTF8_SOURCES {
            assert_eq!(
                profile.original_source_refusal(source),
                (!accepted).then_some(MeasuredIrulesSourceRefusal::NonAsciiRejected)
            );
            let mut changed = source.to_vec();
            changed.push(b' ');
            assert_eq!(
                profile.original_source_refusal(&changed),
                Some(MeasuredIrulesSourceRefusal::NonAsciiUnsupported)
            );
        }
        assert_eq!(
            profile.original_source_refusal("when HTTP_REQUEST {set 😀 1}".as_bytes()),
            Some(MeasuredIrulesSourceRefusal::NonAsciiUnsupported)
        );
    }

    /// The §4b split is exact: 16 + 15 disjoint commands, together the
    /// §5 31-command disabled list.
    #[test]
    fn the_split_is_sixteen_plus_fifteen_and_disjoint() {
        assert_eq!(IRULES_INTERPRETER_ABSENT.len(), 16);
        assert_eq!(IRULES_COMPILER_REFUSED.len(), 15);
        for absent in IRULES_INTERPRETER_ABSENT {
            assert!(
                !IRULES_COMPILER_REFUSED.contains(absent),
                "{absent} cannot be in both classes"
            );
        }
        // The §5 disabled list, verbatim.
        let disabled = [
            "auto_execok",
            "auto_import",
            "auto_load",
            "auto_qualify",
            "cd",
            "eof",
            "exec",
            "exit",
            "fblocked",
            "fconfigure",
            "fcopy",
            "file",
            "flush",
            "gets",
            "glob",
            "interp",
            "load",
            "namespace",
            "open",
            "package",
            "pid",
            "pwd",
            "rename",
            "seek",
            "socket",
            "source",
            "tell",
            "time",
            "unknown",
            "update",
            "vwait",
        ];
        assert_eq!(disabled.len(), 31);
        for command in disabled {
            assert!(
                irules_disabled_class(command).is_some(),
                "{command} must classify"
            );
        }
        assert_eq!(irules_disabled_class("set"), None);
        assert_eq!(irules_disabled_class("HTTP::uri"), None);
        assert_eq!(
            irules_disabled_class("exec"),
            Some(IrulesDisabledClass::InterpreterAbsent)
        );
        assert_eq!(
            irules_disabled_class("rename"),
            Some(IrulesDisabledClass::CompilerRefused)
        );
        assert!(IrulesDisabledClass::InterpreterAbsent.is_language_fact());
        assert!(!IrulesDisabledClass::CompilerRefused.is_language_fact());
    }

    #[test]
    fn explicit_runtime_phase_admits_only_the_measured_native_table() {
        use tcl_dialect::model::InvocationRealm;
        let context = crate::model::semantic::SemanticContext::for_environment("f5-irules");
        let registry = context.commands();
        let loader = context.context().authoring_query();
        let runtime = loader.with_realm(InvocationRealm::InterpreterRuntime);
        for &command in IRULES_COMPILER_REFUSED {
            assert!(
                registry.get_for_surface(command, Some(loader)).is_none(),
                "{command}: loader"
            );
            assert!(
                registry.get_for_surface(command, Some(runtime)).is_some(),
                "{command}: runtime"
            );
            assert!(
                context
                    .context()
                    .resolve_spec_in_realm(registry, command, InvocationRealm::InterpreterRuntime,)
                    .is_some(),
                "{command}: contextual runtime"
            );
            assert!(rule_loader_refuses(command, InvocationRealm::RuleLoader));
            assert!(!rule_loader_refuses(
                command,
                InvocationRealm::InterpreterRuntime
            ));
        }
        for command in IRULES_INTERPRETER_ABSENT
            .iter()
            .copied()
            .chain(["lassign", "const"])
        {
            assert!(
                registry.get_for_surface(command, Some(runtime)).is_none(),
                "{command}: no ancestor donation"
            );
            assert!(
                context
                    .context()
                    .resolve_spec_in_realm(registry, command, InvocationRealm::InterpreterRuntime,)
                    .is_none(),
                "{command}: contextual absence"
            );
        }
        for command in ["set", "HTTP::uri"] {
            assert!(
                registry.get_for_surface(command, Some(loader)).is_some(),
                "{command}: positive loader {loader:?}, rows {:?}",
                registry
                    .specs(command)
                    .iter()
                    .map(|spec| (spec.name, spec.surface, spec.supports_dialect(Some(loader))))
                    .collect::<Vec<_>>()
            );
            assert!(
                registry.get_for_surface(command, Some(runtime)).is_some(),
                "{command}: positive runtime"
            );
        }
        let dialect = crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
        let loader_table = registry.effective_semantics_for_dialect(dialect);
        let runtime_table = registry
            .effective_semantics_for_dialect_in_realm(dialect, InvocationRealm::InterpreterRuntime);
        for &command in IRULES_COMPILER_REFUSED {
            let name = tcl_syntax::naming::normalise_qualified_name(command);
            assert!(
                !loader_table.binding_names().contains(&name),
                "{command}: source table"
            );
            assert!(
                runtime_table.binding_names().contains(&name),
                "{command}: physical table"
            );
        }
        assert_eq!(runtime.with_realm(InvocationRealm::RuleLoader), loader);
    }

    /// Every classified command exists in the compiled universe as a Tcl spec
    /// that does NOT carry an iRules row — the two lists refine the spec-level
    /// exclusion, they never contradict it.
    #[test]
    fn the_classes_refine_the_spec_level_exclusion() {
        let registry = crate::CommandRegistry::build_default();
        for command in IRULES_INTERPRETER_ABSENT
            .iter()
            .chain(IRULES_COMPILER_REFUSED)
        {
            let specs = registry.specs(command);
            assert!(!specs.is_empty(), "{command} must have a spec");
            for spec in specs {
                let gate = spec.surface.expect("stock builtins carry a gate");
                assert!(
                    !surface_admits(gate, Some(&SurfaceQuery::any_release(Family::F5Irules))),
                    "{command} must not carry the IRULES bit"
                );
            }
        }
    }
}
