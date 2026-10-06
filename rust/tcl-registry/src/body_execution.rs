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

//! Registry-owned grammar and control-flow contracts for evaluated scripts.

use crate::InvocationArguments;

/// Authored stock loader facts. A driver must explicitly select this provider;
/// catalogue availability and `package require` alone do not prove it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StockBodyProvider {
    /// Native execution family whose frame and completion contracts were audited.
    pub required_core_family: tcl_dialect::model::Family,
    /// Package independent of the Tcl core release.
    pub package: &'static str,
    /// Stable implementation identity accepted by the execution descriptor.
    pub implementation_id: &'static str,
    /// Namespace installed by the provider.
    pub namespace: &'static str,
    /// Lookup contexts used by loader code and installed implementation code.
    pub core_lookup_namespaces: &'static [&'static str],
    /// Private namespace state influencing callbacks and phase selection.
    pub state_dependency_namespaces: &'static [&'static str],
    /// State writes whose effects are explicitly represented by the region.
    pub modelled_state_variables: &'static [&'static str],
    /// Exact package versions with audited command/export surfaces.
    pub versions: &'static [StockBodyProviderVersion],
}

/// Audited package surface for one implementation version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StockBodyProviderVersion {
    /// Exact package version, not a minimum or a core version.
    pub version: &'static str,
    /// Commands installed in the provider namespace, whitespace-separated.
    pub commands: &'static str,
    /// Export patterns in the provider namespace, whitespace-separated.
    pub exports: &'static str,
    /// Literal core lookup closure from C Tcl parsing of stock loader/proc scripts.
    /// Includes conservative potential-script descent, rather than claiming an exact call graph.
    pub core_lookups: &'static [&'static str],
}

impl StockBodyProvider {
    /// Potential core heads reached by at least one authored package version.
    #[must_use]
    pub fn possible_core_lookups(self) -> Vec<&'static str> {
        self.versions
            .iter()
            .flat_map(|version| version.core_lookups.iter().copied())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Core heads required by every authored implementation alternative.
    #[must_use]
    pub fn common_core_lookups(self) -> Vec<&'static str> {
        let Some(first) = self.versions.first() else {
            return Vec::new();
        };
        first
            .core_lookups
            .iter()
            .copied()
            .filter(|head| {
                self.versions
                    .iter()
                    .all(|version| version.core_lookups.contains(head))
            })
            .collect()
    }
    /// Every command supplied by at least one authored implementation version.
    #[must_use]
    pub fn possible_commands(self) -> Vec<&'static str> {
        self.versions
            .iter()
            .flat_map(|version| version.commands.split_whitespace())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Every export supplied by at least one authored implementation version.
    #[must_use]
    pub fn possible_exports(self) -> Vec<&'static str> {
        self.versions
            .iter()
            .flat_map(|version| version.exports.split_whitespace())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    /// Commands common to every authored version, preserving absence of
    /// optional extension hooks without inferring a package version from core.
    #[must_use]
    pub fn common_commands(self) -> Vec<&'static str> {
        let Some(first) = self.versions.first() else {
            return Vec::new();
        };
        first
            .commands
            .split_whitespace()
            .filter(|command| {
                self.versions.iter().all(|version| {
                    version
                        .commands
                        .split_whitespace()
                        .any(|candidate| candidate == *command)
                })
            })
            .collect()
    }

    /// Public exports common to every authored version.
    #[must_use]
    pub fn common_exports(self) -> Vec<&'static str> {
        let Some(first) = self.versions.first() else {
            return Vec::new();
        };
        first
            .exports
            .split_whitespace()
            .filter(|export| {
                self.versions.iter().all(|version| {
                    version
                        .exports
                        .split_whitespace()
                        .any(|candidate| candidate == *export)
                })
            })
            .collect()
    }
}

/// Stock C Tcl tcltest implementation, audited across the five oracle releases.
/// This provider is selected explicitly and never applies to Jim's independent
/// tcltest implementation, even when its package advertisement has this name.
pub const TCLTEST_STOCK_PROVIDER: StockBodyProvider = StockBodyProvider {
    required_core_family: tcl_dialect::model::Family::Tcl,
    package: "tcltest",
    implementation_id: "c-tcl-distribution:tcltest",
    namespace: "::tcltest",
    core_lookup_namespaces: &["::", "::tcltest"],
    state_dependency_namespaces: &["::tcltest"],
    modelled_state_variables: &[
        "::tcltest::Option(-iterations)",
        "::tcltest::testIterations",
    ],
    versions: &[
        StockBodyProviderVersion {
            version: "2.2.11",
            commands: "::tcltest::AcceptAbsolutePath ::tcltest::AcceptAll ::tcltest::AcceptBoolean ::tcltest::AcceptDirectory ::tcltest::AcceptInteger ::tcltest::AcceptList ::tcltest::AcceptLoadFile ::tcltest::AcceptOutFile ::tcltest::AcceptPattern ::tcltest::AcceptReadable ::tcltest::AcceptScript ::tcltest::AcceptTemporaryDirectory ::tcltest::AcceptVerbose ::tcltest::AddToSkippedBecause ::tcltest::ArrayDefault ::tcltest::ClearUnselectedConstraints ::tcltest::CompareStrings ::tcltest::Configure ::tcltest::ConfigureFromEnvironment ::tcltest::ConstraintInitializer ::tcltest::DebugDo ::tcltest::DebugPArray ::tcltest::DebugPuts ::tcltest::Default ::tcltest::DefineConstraintInitializers ::tcltest::EstablishAutoConfigureTraces ::tcltest::Eval ::tcltest::FillFilesExisted ::tcltest::GetMatchingDirectories ::tcltest::GetMatchingFiles ::tcltest::InitConstraints ::tcltest::IsVerbose ::tcltest::LeakFiles ::tcltest::LoadTimeCmdLineArgParsingRequired ::tcltest::MatchingOption ::tcltest::OpenFiles ::tcltest::Option ::tcltest::PrintError ::tcltest::PrintUsageInfo ::tcltest::PrintUsageInfoHook ::tcltest::ProcessCmdLineArgs ::tcltest::ProcessFlags ::tcltest::ReadLoadScript ::tcltest::RemoveAutoConfigureTraces ::tcltest::RestoreLocale ::tcltest::RunTest ::tcltest::SafeFetch ::tcltest::SetIso8859_1_Locale ::tcltest::SetSelectedConstraints ::tcltest::Skipped ::tcltest::SubstArguments ::tcltest::Usage ::tcltest::Warn ::tcltest::bytestring ::tcltest::cleanupTests ::tcltest::cleanupTestsHook ::tcltest::configure ::tcltest::customMatch ::tcltest::debug ::tcltest::errorChannel ::tcltest::errorFile ::tcltest::getMatchingFiles ::tcltest::initConstraintsHook ::tcltest::interpreter ::tcltest::limitConstraints ::tcltest::loadFile ::tcltest::loadScript ::tcltest::loadTestedCommands ::tcltest::mainThread ::tcltest::makeDirectory ::tcltest::makeFile ::tcltest::match ::tcltest::matchDirectories ::tcltest::matchFiles ::tcltest::normalizeMsg ::tcltest::normalizePath ::tcltest::outputChannel ::tcltest::outputFile ::tcltest::parray ::tcltest::preserveCore ::tcltest::processCmdLineArgsAddFlagsHook ::tcltest::processCmdLineArgsHook ::tcltest::removeDirectory ::tcltest::removeFile ::tcltest::restoreState ::tcltest::runAllTests ::tcltest::saveState ::tcltest::singleProcess ::tcltest::skip ::tcltest::skipDirectories ::tcltest::skipFiles ::tcltest::temporaryDirectory ::tcltest::test ::tcltest::testConstraint ::tcltest::testsDirectory ::tcltest::threadReap ::tcltest::verbose ::tcltest::viewFile ::tcltest::workingDirectory",
            exports: "cleanupTests loadTestedCommands makeDirectory makeFile removeDirectory removeFile runAllTests test configure customMatch errorChannel interpreter outputChannel testConstraint bytestring debug errorFile limitConstraints loadFile loadScript match matchFiles matchDirectories normalizeMsg normalizePath outputFile preserveCore singleProcess skip skipFiles skipDirectories temporaryDirectory testsDirectory verbose viewFile workingDirectory getMatchingFiles mainThread restoreState saveState threadReap",
            core_lookups: &[
                "after",
                "append",
                "array",
                "auto_execok",
                "auto_load",
                "break",
                "catch",
                "cd",
                "clock",
                "close",
                "concat",
                "continue",
                "encoding",
                "error",
                "eval",
                "exec",
                "exit",
                "expr",
                "fconfigure",
                "file",
                "flush",
                "for",
                "foreach",
                "format",
                "gets",
                "glob",
                "global",
                "if",
                "incr",
                "info",
                "interp",
                "join",
                "lappend",
                "lindex",
                "linsert",
                "list",
                "llength",
                "lrange",
                "lreplace",
                "lsearch",
                "lsort",
                "namespace",
                "open",
                "package",
                "proc",
                "puts",
                "pwd",
                "read",
                "regexp",
                "regsub",
                "rename",
                "return",
                "set",
                "socket",
                "split",
                "string",
                "subst",
                "switch",
                "trace",
                "unset",
                "uplevel",
                "upvar",
                "variable",
                "while",
            ],
        },
        StockBodyProviderVersion {
            version: "2.3.8",
            commands: "::tcltest::AcceptAbsolutePath ::tcltest::AcceptAll ::tcltest::AcceptBoolean ::tcltest::AcceptDirectory ::tcltest::AcceptInteger ::tcltest::AcceptList ::tcltest::AcceptLoadFile ::tcltest::AcceptOutFile ::tcltest::AcceptPattern ::tcltest::AcceptReadable ::tcltest::AcceptScript ::tcltest::AcceptTemporaryDirectory ::tcltest::AcceptVerbose ::tcltest::AddToSkippedBecause ::tcltest::ArrayDefault ::tcltest::ClearUnselectedConstraints ::tcltest::CompareStrings ::tcltest::Configure ::tcltest::ConfigureFromEnvironment ::tcltest::ConstraintInitializer ::tcltest::DebugDo ::tcltest::DebugPArray ::tcltest::DebugPuts ::tcltest::Default ::tcltest::DefineConstraintInitializers ::tcltest::EstablishAutoConfigureTraces ::tcltest::Eval ::tcltest::FillFilesExisted ::tcltest::GetMatchingDirectories ::tcltest::GetMatchingFiles ::tcltest::InitConstraints ::tcltest::IsVerbose ::tcltest::LeakFiles ::tcltest::LoadTimeCmdLineArgParsingRequired ::tcltest::MatchingOption ::tcltest::OpenFiles ::tcltest::Option ::tcltest::PrintError ::tcltest::PrintUsageInfo ::tcltest::PrintUsageInfoHook ::tcltest::ProcessCmdLineArgs ::tcltest::ProcessFlags ::tcltest::ReadLoadScript ::tcltest::RemoveAutoConfigureTraces ::tcltest::ReportedFromSlave ::tcltest::RestoreLocale ::tcltest::RunTest ::tcltest::SafeFetch ::tcltest::SetIso8859_1_Locale ::tcltest::SetSelectedConstraints ::tcltest::Skipped ::tcltest::SubstArguments ::tcltest::Usage ::tcltest::Warn ::tcltest::bytestring ::tcltest::cleanupTests ::tcltest::cleanupTestsHook ::tcltest::configure ::tcltest::customMatch ::tcltest::debug ::tcltest::errorChannel ::tcltest::errorFile ::tcltest::getMatchingFiles ::tcltest::initConstraintsHook ::tcltest::interpreter ::tcltest::limitConstraints ::tcltest::loadFile ::tcltest::loadIntoSlaveInterpreter ::tcltest::loadScript ::tcltest::loadTestedCommands ::tcltest::mainThread ::tcltest::makeDirectory ::tcltest::makeFile ::tcltest::match ::tcltest::matchDirectories ::tcltest::matchFiles ::tcltest::normalizeMsg ::tcltest::normalizePath ::tcltest::outputChannel ::tcltest::outputFile ::tcltest::parray ::tcltest::preserveCore ::tcltest::processCmdLineArgsAddFlagsHook ::tcltest::processCmdLineArgsHook ::tcltest::removeDirectory ::tcltest::removeFile ::tcltest::restoreState ::tcltest::runAllTests ::tcltest::saveState ::tcltest::singleProcess ::tcltest::skip ::tcltest::skipDirectories ::tcltest::skipFiles ::tcltest::temporaryDirectory ::tcltest::test ::tcltest::testConstraint ::tcltest::testsDirectory ::tcltest::threadReap ::tcltest::verbose ::tcltest::viewFile ::tcltest::workingDirectory",
            exports: "cleanupTests loadTestedCommands makeDirectory makeFile removeDirectory removeFile runAllTests test configure customMatch errorChannel interpreter outputChannel testConstraint bytestring debug errorFile limitConstraints loadFile loadScript match matchFiles matchDirectories normalizeMsg normalizePath outputFile preserveCore singleProcess skip skipFiles skipDirectories temporaryDirectory testsDirectory verbose viewFile workingDirectory getMatchingFiles mainThread restoreState saveState threadReap",
            core_lookups: &[
                "after",
                "append",
                "array",
                "auto_execok",
                "auto_load",
                "break",
                "catch",
                "cd",
                "chan",
                "clock",
                "close",
                "concat",
                "continue",
                "dict",
                "encoding",
                "error",
                "eval",
                "exec",
                "exit",
                "expr",
                "file",
                "flush",
                "for",
                "foreach",
                "format",
                "gets",
                "glob",
                "global",
                "if",
                "incr",
                "info",
                "interp",
                "join",
                "lappend",
                "lassign",
                "lindex",
                "linsert",
                "list",
                "llength",
                "lrange",
                "lreplace",
                "lsearch",
                "lsort",
                "namespace",
                "open",
                "package",
                "proc",
                "puts",
                "pwd",
                "read",
                "regexp",
                "regsub",
                "rename",
                "return",
                "set",
                "socket",
                "split",
                "string",
                "subst",
                "switch",
                "trace",
                "unset",
                "uplevel",
                "upvar",
                "variable",
                "while",
            ],
        },
        StockBodyProviderVersion {
            version: "2.5.11",
            commands: "::tcltest::AcceptAbsolutePath ::tcltest::AcceptAll ::tcltest::AcceptBoolean ::tcltest::AcceptDirectory ::tcltest::AcceptInteger ::tcltest::AcceptList ::tcltest::AcceptLoadFile ::tcltest::AcceptOutFile ::tcltest::AcceptPattern ::tcltest::AcceptReadable ::tcltest::AcceptScript ::tcltest::AcceptTemporaryDirectory ::tcltest::AcceptVerbose ::tcltest::AddToSkippedBecause ::tcltest::ArrayDefault ::tcltest::Asciify ::tcltest::ClearUnselectedConstraints ::tcltest::CompareStrings ::tcltest::Configure ::tcltest::ConfigureFromEnvironment ::tcltest::ConstraintInitializer ::tcltest::DebugDo ::tcltest::DebugPArray ::tcltest::DebugPuts ::tcltest::Default ::tcltest::DefineConstraintInitializers ::tcltest::EstablishAutoConfigureTraces ::tcltest::Eval ::tcltest::FillFilesExisted ::tcltest::GetMatchingDirectories ::tcltest::GetMatchingFiles ::tcltest::InitConstraints ::tcltest::IsVerbose ::tcltest::LeakFiles ::tcltest::LoadTimeCmdLineArgParsingRequired ::tcltest::MatchingOption ::tcltest::OpenFiles ::tcltest::Option ::tcltest::PrintError ::tcltest::PrintUsageInfo ::tcltest::PrintUsageInfoHook ::tcltest::ProcessCmdLineArgs ::tcltest::ProcessFlags ::tcltest::ReadLoadScript ::tcltest::RemoveAutoConfigureTraces ::tcltest::ReportedFromChild ::tcltest::RestoreLocale ::tcltest::RunTest ::tcltest::SafeFetch ::tcltest::SetIso8859_1_Locale ::tcltest::SetSelectedConstraints ::tcltest::Skip ::tcltest::Skipped ::tcltest::SubstArguments ::tcltest::Usage ::tcltest::Warn ::tcltest::_noticeSkipped ::tcltest::bytestring ::tcltest::cleanupTests ::tcltest::cleanupTestsHook ::tcltest::configure ::tcltest::customMatch ::tcltest::debug ::tcltest::errorChannel ::tcltest::errorFile ::tcltest::getMatchingFiles ::tcltest::initConstraintsHook ::tcltest::interpreter ::tcltest::limitConstraints ::tcltest::loadFile ::tcltest::loadIntoChildInterpreter ::tcltest::loadScript ::tcltest::loadTestedCommands ::tcltest::mainThread ::tcltest::makeDirectory ::tcltest::makeFile ::tcltest::match ::tcltest::matchDirectories ::tcltest::matchFiles ::tcltest::normalizeMsg ::tcltest::normalizePath ::tcltest::outputChannel ::tcltest::outputFile ::tcltest::parray ::tcltest::preserveCore ::tcltest::processCmdLineArgsAddFlagsHook ::tcltest::processCmdLineArgsHook ::tcltest::removeDirectory ::tcltest::removeFile ::tcltest::restoreState ::tcltest::runAllTests ::tcltest::saveState ::tcltest::singleProcess ::tcltest::skip ::tcltest::skipDirectories ::tcltest::skipFiles ::tcltest::temporaryDirectory ::tcltest::test ::tcltest::testConstraint ::tcltest::testsDirectory ::tcltest::threadReap ::tcltest::verbose ::tcltest::viewFile ::tcltest::workingDirectory",
            exports: "cleanupTests loadTestedCommands makeDirectory makeFile removeDirectory removeFile runAllTests test configure customMatch errorChannel interpreter outputChannel testConstraint bytestring debug errorFile limitConstraints loadFile loadScript match matchFiles matchDirectories normalizeMsg normalizePath outputFile preserveCore singleProcess skip skipFiles skipDirectories temporaryDirectory testsDirectory verbose viewFile workingDirectory getMatchingFiles mainThread restoreState saveState threadReap",
            core_lookups: &[
                "after",
                "append",
                "array",
                "auto_execok",
                "auto_load",
                "break",
                "catch",
                "cd",
                "clock",
                "close",
                "concat",
                "continue",
                "dict",
                "encoding",
                "error",
                "eval",
                "exec",
                "exit",
                "expr",
                "fconfigure",
                "file",
                "flush",
                "for",
                "foreach",
                "format",
                "gets",
                "glob",
                "global",
                "if",
                "incr",
                "info",
                "interp",
                "join",
                "lappend",
                "lassign",
                "lindex",
                "linsert",
                "list",
                "llength",
                "lrange",
                "lreplace",
                "lsearch",
                "lsort",
                "namespace",
                "open",
                "package",
                "proc",
                "puts",
                "pwd",
                "read",
                "regexp",
                "regsub",
                "rename",
                "return",
                "scan",
                "set",
                "socket",
                "split",
                "string",
                "subst",
                "switch",
                "trace",
                "unset",
                "uplevel",
                "upvar",
                "variable",
                "while",
            ],
        },
        StockBodyProviderVersion {
            version: "2.6.0",
            commands: "::tcltest::AcceptAbsolutePath ::tcltest::AcceptAll ::tcltest::AcceptBoolean ::tcltest::AcceptDirectory ::tcltest::AcceptInteger ::tcltest::AcceptList ::tcltest::AcceptLoadFile ::tcltest::AcceptOutFile ::tcltest::AcceptPattern ::tcltest::AcceptReadable ::tcltest::AcceptScript ::tcltest::AcceptTemporaryDirectory ::tcltest::AcceptVerbose ::tcltest::AddToSkippedBecause ::tcltest::ArrayDefault ::tcltest::Asciify ::tcltest::ClearUnselectedConstraints ::tcltest::CompareStrings ::tcltest::Configure ::tcltest::ConfigureFromEnvironment ::tcltest::ConstraintInitializer ::tcltest::DebugDo ::tcltest::DebugPArray ::tcltest::DebugPuts ::tcltest::Default ::tcltest::DefineConstraintInitializers ::tcltest::EstablishAutoConfigureTraces ::tcltest::Eval ::tcltest::FillFilesExisted ::tcltest::GetMatchingDirectories ::tcltest::GetMatchingFiles ::tcltest::InitConstraints ::tcltest::IsVerbose ::tcltest::LeakFiles ::tcltest::LoadTimeCmdLineArgParsingRequired ::tcltest::MatchingOption ::tcltest::OpenFiles ::tcltest::Option ::tcltest::PrintError ::tcltest::PrintUsageInfo ::tcltest::PrintUsageInfoHook ::tcltest::ProcessCmdLineArgs ::tcltest::ProcessFlags ::tcltest::ReadLoadScript ::tcltest::RemoveAutoConfigureTraces ::tcltest::ReportedFromChild ::tcltest::RestoreLocale ::tcltest::RunTest ::tcltest::SafeFetch ::tcltest::SetIso8859_1_Locale ::tcltest::SetSelectedConstraints ::tcltest::Skip ::tcltest::Skipped ::tcltest::SubstArguments ::tcltest::TestOnce ::tcltest::Usage ::tcltest::Warn ::tcltest::_noticeSkipped ::tcltest::cleanupTests ::tcltest::cleanupTestsHook ::tcltest::configure ::tcltest::customMatch ::tcltest::debug ::tcltest::errorChannel ::tcltest::errorFile ::tcltest::getMatchingFiles ::tcltest::initConstraintsHook ::tcltest::interpreter ::tcltest::limitConstraints ::tcltest::loadFile ::tcltest::loadIntoChildInterpreter ::tcltest::loadScript ::tcltest::loadTestedCommands ::tcltest::mainThread ::tcltest::makeDirectory ::tcltest::makeFile ::tcltest::match ::tcltest::matchDirectories ::tcltest::matchFiles ::tcltest::normalizeMsg ::tcltest::normalizePath ::tcltest::outputChannel ::tcltest::outputFile ::tcltest::parray ::tcltest::preserveCore ::tcltest::processCmdLineArgsAddFlagsHook ::tcltest::processCmdLineArgsHook ::tcltest::removeDirectory ::tcltest::removeFile ::tcltest::restoreState ::tcltest::runAllTests ::tcltest::saveState ::tcltest::singleProcess ::tcltest::skip ::tcltest::skipDirectories ::tcltest::skipFiles ::tcltest::temporaryDirectory ::tcltest::test ::tcltest::testConstraint ::tcltest::testIterations ::tcltest::testsDirectory ::tcltest::threadReap ::tcltest::verbose ::tcltest::viewFile ::tcltest::workingDirectory",
            exports: "cleanupTests loadTestedCommands makeDirectory makeFile removeDirectory removeFile runAllTests test configure customMatch errorChannel interpreter outputChannel testConstraint debug errorFile limitConstraints loadFile loadScript match matchFiles matchDirectories normalizeMsg normalizePath outputFile preserveCore singleProcess skip skipFiles skipDirectories temporaryDirectory testsDirectory verbose viewFile workingDirectory getMatchingFiles mainThread restoreState saveState threadReap",
            core_lookups: &[
                "after",
                "append",
                "array",
                "auto_execok",
                "auto_load",
                "break",
                "catch",
                "cd",
                "clock",
                "close",
                "concat",
                "continue",
                "dict",
                "encoding",
                "error",
                "eval",
                "exec",
                "exit",
                "expr",
                "fconfigure",
                "file",
                "flush",
                "for",
                "foreach",
                "format",
                "gets",
                "glob",
                "global",
                "if",
                "incr",
                "info",
                "interp",
                "join",
                "lappend",
                "lassign",
                "lindex",
                "linsert",
                "list",
                "llength",
                "lrange",
                "lreplace",
                "lsearch",
                "lsort",
                "namespace",
                "open",
                "package",
                "proc",
                "puts",
                "pwd",
                "read",
                "regexp",
                "regsub",
                "rename",
                "return",
                "scan",
                "set",
                "socket",
                "split",
                "string",
                "subst",
                "switch",
                "trace",
                "unset",
                "uplevel",
                "upvar",
                "variable",
                "while",
            ],
        },
    ],
};

/// A descriptor selected only after proving the live command implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyExecutionSpec {
    /// Each selected Body-role operand executes once in the caller's frame.
    CallerFrameSequence,
    /// C Tcl 9 array lookup and sequential key/value stores precede each caller-frame body.
    ArrayIteration,
    /// Actual file bytes are supplied by the execution driver after native argv selection.
    SourceFile,
    /// Substitute the selected template value in the caller's frame.
    /// Native variable reads remain ordered and may execute read observers.
    SubstitutionTemplate,
    /// Store selected script operands for a later global-frame invocation.
    DeferredGlobalScript,
    /// Dictionary mapping plus a completion-sensitive caller-frame epilogue.
    DictionaryScope(crate::dictionary_scope::DictionaryScopeSpec),
    /// Three caller-frame phases; setup failure omits body, cleanup captures all
    /// completions, and an external selection can omit the entire lifecycle.
    CapturedLifecycle(&'static CapturedLifecycleSpec),
}

/// Frame entered by an independently scheduled script, after registration.
/// This describes future entry; it does not enter the body on the normal edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeferredBodyFrame {
    /// A fresh global activation, outside the registering procedure/namespace.
    Global,
}

/// Argument grammar for a captured setup/body/cleanup lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapturedLifecycleSpec {
    /// Selected stock source contract required before exposing script effects.
    pub provider: StockBodyProvider,
    /// Native entry used by each selected lifecycle phase in the caller frame.
    pub phase_compilation: crate::native_compilation::NativeBodyCompilation,
    /// Arguments before the option/legacy-script grammar.
    pub leading_arguments: usize,
    /// Setup script option.
    pub setup_option: &'static str,
    /// Main script option.
    pub body_option: &'static str,
    /// Cleanup script option.
    pub cleanup_option: &'static str,
    /// Complete option vocabulary; unknown options fail before entering scripts.
    pub valid_options: &'static [&'static str],
    /// Options whose pre-phase callbacks/effects need a separate proof.
    pub pre_phase_effect_options: &'static [&'static str],
    /// Options whose values must pass the shared list parser before phases.
    pub list_valued_options: &'static [&'static str],
    /// Exact authored version option vocabularies; absence uses intersection.
    pub option_versions: &'static [(&'static str, &'static [&'static str])],
    /// Whether a single argument carries an option list.
    pub option_list: bool,
    /// Whether the legacy form is body/result or constraints/body/result.
    pub legacy_positional: bool,
    /// Optional commands that can replace scripts in hook-enabled packages.
    pub hook_commands: &'static [&'static str],
    /// Verified package-version contracts; unlisted versions remain unknown.
    pub hook_versions: &'static [(&'static str, BodyHookPolicy)],
    /// Exact version iteration contracts; unaudited/unspecified means may-repeat.
    pub repetition_versions: &'static [(&'static str, BodyRepetition)],
}

/// Version-specific wrapper extension behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyHookPolicy {
    /// The package never calls these optional hooks.
    Disabled,
    /// Bound hooks can replace authored scripts.
    OptionalOverride,
    /// The selected implementation/version has not established hook behaviour.
    Unknown,
}

/// Iteration topology of an authored lifecycle implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyRepetition {
    /// This implementation enters its lifecycle at most once per invocation.
    Once,
    /// Framework configuration can repeat the entire lifecycle.
    MayRepeat,
}

/// A script operand in the effective post-alias argument vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyOperand {
    /// Argument after the command head.
    pub argument: usize,
    /// Element of a list-valued argument; none for a direct argument.
    pub list_element: Option<usize>,
}

/// The last selected operand for each lifecycle phase.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CapturedLifecycleSelection {
    /// Optional setup phase; omission means an empty script.
    pub setup: Option<BodyOperand>,
    /// Optional main phase; omission means an empty script.
    pub body: Option<BodyOperand>,
    /// Optional cleanup phase; omission means an empty script.
    pub cleanup: Option<BodyOperand>,
}

/// Grammar knowledge; uncertainty never licenses script execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BodyExecutionSelection {
    /// Known phase operands, in semantic rather than argument order.
    CapturedLifecycle(CapturedLifecycleSelection),
    /// Tcl rejects the argument grammar before executing a phase.
    InvalidArguments,
    /// Computed option names, expansion, or unreadable list shape.
    UnknownArguments,
}

impl CapturedLifecycleSpec {
    /// Whether selection can run callbacks or alter the execution environment
    /// before the authored scripts. Consumers must model those effects or
    /// retain an opaque wrapper; syntax-only operand selection is insufficient.
    /// This includes the single-list substitution protocol and legacy scripted
    /// constraints, independently of the runtime spelling of the command.
    #[must_use]
    pub fn has_unmodelled_pre_phase_effects(self, arguments: InvocationArguments<'_>) -> bool {
        let Some(count) = arguments.exact_argv_len() else {
            return true;
        };
        let Some(remaining) = count.checked_sub(self.leading_arguments) else {
            return true;
        };
        if remaining == 0 {
            return false;
        }
        if remaining == 1 && self.option_list {
            return true;
        }
        let Some(first) = arguments.literal_at(self.leading_arguments) else {
            return true;
        };
        if !first.starts_with('-') && self.legacy_positional {
            return remaining != 2;
        }
        if !remaining.is_multiple_of(2) {
            return true;
        }
        (self.leading_arguments..count).step_by(2).any(|index| {
            arguments.literal_at(index).is_none_or(|option| {
                !self.valid_options.contains(&option)
                    || self.pre_phase_effect_options.contains(&option)
            })
        })
    }

    /// Retain possible repeats when a package version/configuration is unknown.
    #[must_use]
    pub fn repetition(self, version: Option<&str>) -> BodyRepetition {
        version
            .and_then(|version| {
                self.repetition_versions
                    .iter()
                    .find(|(known, _)| *known == version)
                    .map(|(_, repetition)| *repetition)
            })
            .unwrap_or(BodyRepetition::MayRepeat)
    }
    /// Whether every represented implementation accepts this option.
    #[must_use]
    pub fn supports_option(self, version: Option<&str>, option: &str) -> bool {
        if let Some(version) = version {
            self.option_versions
                .iter()
                .find(|(known, _)| *known == version)
                .is_some_and(|(_, options)| options.contains(&option))
        } else {
            !self.option_versions.is_empty()
                && self
                    .option_versions
                    .iter()
                    .all(|(_, options)| options.contains(&option))
        }
    }
    /// Resolve only authored package-version contracts, independently of Tcl core.
    #[must_use]
    pub fn hook_policy(self, package_version: Option<&str>) -> BodyHookPolicy {
        package_version
            .and_then(|version| {
                self.hook_versions
                    .iter()
                    .find(|(known, _)| *known == version)
                    .map(|(_, policy)| *policy)
            })
            .unwrap_or(BodyHookPolicy::Unknown)
    }

    /// Hooks whose absence must be proved before expanding the authored scripts.
    #[must_use]
    pub fn required_absent_hooks(self, package_version: Option<&str>) -> &'static [&'static str] {
        if self.hook_policy(package_version) == BodyHookPolicy::Disabled {
            &[]
        } else {
            self.hook_commands
        }
    }

    /// Select operands once, keeping last-option-wins and legacy/list forms.
    ///
    /// Selection establishes grammar, not the live command implementation.
    /// Computed script values retain known operand positions; computed option
    /// names or expansions retain `UnknownArguments`.
    ///
    /// ```
    /// use tcl_registry::{CommandRegistry, InvocationArguments};
    /// use tcl_registry::body_execution::{BodyExecutionSelection};
    /// let registry = CommandRegistry::build_default();
    /// let contract = registry.get("tcltest::test").unwrap().body_execution.unwrap();
    /// let selected = contract.select(InvocationArguments::literals(&[
    ///     "case", "description", "-body", "puts $x", "-setup", "set x 1",
    /// ]));
    /// let BodyExecutionSelection::CapturedLifecycle(phases) = selected else { panic!() };
    /// assert_eq!(phases.setup.unwrap().argument, 5);
    /// assert_eq!(phases.body.unwrap().argument, 3);
    /// ```
    #[must_use]
    pub fn select(self, arguments: InvocationArguments<'_>) -> BodyExecutionSelection {
        let Some(count) = arguments.exact_argv_len() else {
            return BodyExecutionSelection::UnknownArguments;
        };
        if count < self.leading_arguments {
            return BodyExecutionSelection::InvalidArguments;
        }
        let remaining = count - self.leading_arguments;
        if remaining == 1 && self.option_list {
            let Some(value) = arguments.literal_at(self.leading_arguments) else {
                return BodyExecutionSelection::UnknownArguments;
            };
            let rules = arguments
                .dialect()
                .map_or(tcl_syntax::word_rules::WordValueRules::TCL, |dialect| {
                    dialect.word_values
                });
            let Ok(elements) = rules.split_list(value) else {
                return BodyExecutionSelection::InvalidArguments;
            };
            let refs: Vec<&str> = elements.iter().map(AsRef::as_ref).collect();
            return self.select_options(&refs, |index| BodyOperand {
                argument: self.leading_arguments,
                list_element: Some(index),
            });
        }
        let Some(first) = arguments.literal_at(self.leading_arguments) else {
            return if remaining == 0 {
                BodyExecutionSelection::CapturedLifecycle(CapturedLifecycleSelection::default())
            } else {
                BodyExecutionSelection::UnknownArguments
            };
        };
        if !first.starts_with('-') && self.legacy_positional {
            return if matches!(remaining, 2 | 3) {
                BodyExecutionSelection::CapturedLifecycle(CapturedLifecycleSelection {
                    body: Some(BodyOperand {
                        argument: count - 2,
                        list_element: None,
                    }),
                    ..CapturedLifecycleSelection::default()
                })
            } else {
                BodyExecutionSelection::InvalidArguments
            };
        }
        if !remaining.is_multiple_of(2) {
            return BodyExecutionSelection::InvalidArguments;
        }
        let mut selected = CapturedLifecycleSelection::default();
        for index in (self.leading_arguments..count).step_by(2) {
            let Some(option) = arguments.literal_at(index) else {
                return BodyExecutionSelection::UnknownArguments;
            };
            if !self.valid_options.contains(&option) {
                return BodyExecutionSelection::InvalidArguments;
            }
            self.select_phase(
                &mut selected,
                option,
                BodyOperand {
                    argument: index + 1,
                    list_element: None,
                },
            );
        }
        BodyExecutionSelection::CapturedLifecycle(selected)
    }

    fn select_options(
        self,
        options: &[&str],
        operand: impl Fn(usize) -> BodyOperand,
    ) -> BodyExecutionSelection {
        if !options.len().is_multiple_of(2) {
            return BodyExecutionSelection::InvalidArguments;
        }
        let mut selected = CapturedLifecycleSelection::default();
        for index in (0..options.len()).step_by(2) {
            if !self.valid_options.contains(&options[index]) {
                return BodyExecutionSelection::InvalidArguments;
            }
            self.select_phase(&mut selected, options[index], operand(index + 1));
        }
        BodyExecutionSelection::CapturedLifecycle(selected)
    }

    fn select_phase(
        self,
        selected: &mut CapturedLifecycleSelection,
        option: &str,
        operand: BodyOperand,
    ) {
        if option == self.setup_option {
            selected.setup = Some(operand);
        } else if option == self.body_option {
            selected.body = Some(operand);
        } else if option == self.cleanup_option {
            selected.cleanup = Some(operand);
        }
    }
}

impl BodyExecutionSpec {
    /// Actual native scheduling frame, separate from body roles and timing.
    /// Unknown embeddings cannot inherit a native frame from catalogue ancestry.
    #[must_use]
    pub fn deferred_entry_frame(
        self,
        dialect: crate::InvocationDialect,
    ) -> Option<DeferredBodyFrame> {
        use tcl_dialect::model::{Family, Release};
        if self != Self::DeferredGlobalScript {
            return None;
        }
        let native = match dialect.family() {
            Some(Family::Tcl) => dialect.tcl_version.is_some(),
            Some(Family::Jim) => dialect
                .core_point
                .is_some_and(|point| point.release() == Release::JIM_0_84),
            _ => false,
        };
        native.then_some(DeferredBodyFrame::Global)
    }

    /// Query the declared grammar using evaluated argument facts.
    #[must_use]
    pub fn select(self, arguments: InvocationArguments<'_>) -> BodyExecutionSelection {
        match self {
            Self::CapturedLifecycle(spec) => spec.select(arguments),
            Self::CallerFrameSequence
            | Self::ArrayIteration
            | Self::DictionaryScope(_)
            | Self::SourceFile
            | Self::SubstitutionTemplate
            | Self::DeferredGlobalScript => BodyExecutionSelection::UnknownArguments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandRegistry, InvocationWord};

    #[test]
    fn deferred_global_entry_requires_actual_native_scheduling_policy() {
        use tcl_dialect::model::{DialectPoint, Release};
        let descriptor = BodyExecutionSpec::DeferredGlobalScript;
        for release in [
            Release::TCL_8_4,
            Release::TCL_8_5,
            Release::TCL_8_6,
            Release::TCL_9_0,
            Release::TCL_9_1,
            Release::JIM_0_84,
        ] {
            let dialect = crate::InvocationDialect::of_point(DialectPoint::canonical(release));
            assert_eq!(
                descriptor.deferred_entry_frame(dialect),
                Some(DeferredBodyFrame::Global)
            );
        }
        let host = crate::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find("f5-irules").expect("host profile"),
        );
        assert_eq!(descriptor.deferred_entry_frame(host), None);
        let unknown = crate::InvocationDialect {
            native_family: None,
            core_point: None,
            tcl_version: None,
            ..host
        };
        assert_eq!(descriptor.deferred_entry_frame(unknown), None);
        let registry = CommandRegistry::build_default();
        let after = registry.get("after").expect("authored after");
        assert_eq!(after.body_execution, Some(descriptor));
        assert_eq!(
            after
                .subcommands
                .iter()
                .find(|sub| sub.name == "idle")
                .unwrap()
                .body_execution,
            Some(descriptor)
        );
    }

    fn contract() -> CapturedLifecycleSpec {
        let registry = CommandRegistry::build_default();
        let BodyExecutionSpec::CapturedLifecycle(spec) = registry
            .get("tcltest::test")
            .unwrap()
            .body_execution
            .unwrap()
        else {
            panic!("expected captured lifecycle")
        };
        *spec
    }

    fn selected(arguments: &[&str]) -> CapturedLifecycleSelection {
        let BodyExecutionSelection::CapturedLifecycle(selected) =
            contract().select(InvocationArguments::literals(arguments))
        else {
            panic!("expected determinate lifecycle")
        };
        selected
    }

    #[test]
    fn semantic_order_and_last_option_value_are_retained() {
        let selected = selected(&[
            "name", "desc", "-cleanup", "cleanup", "-body", "first", "-setup", "setup", "-body",
            "last",
        ]);
        assert_eq!(selected.setup.unwrap().argument, 7);
        assert_eq!(selected.body.unwrap().argument, 9);
        assert_eq!(selected.cleanup.unwrap().argument, 3);
    }

    #[test]
    fn legacy_and_one_list_forms_retain_operand_origins() {
        assert_eq!(
            selected(&["name", "desc", "body", "result"])
                .body
                .unwrap()
                .argument,
            2
        );
        assert_eq!(
            selected(&["name", "desc", "constraint", "body", "result"])
                .body
                .unwrap()
                .argument,
            3
        );
        let selected = selected(&["name", "desc", "-cleanup cleanup -setup setup -body body"]);
        assert_eq!(
            selected.body.unwrap(),
            BodyOperand {
                argument: 2,
                list_element: Some(5)
            }
        );
        assert_eq!(selected.setup.unwrap().list_element, Some(3));
    }

    #[test]
    fn wrapper_pre_phase_effects_are_shared_by_all_execution_consumers() {
        for arguments in [
            &["name", "desc", "-body", "script", "-setup", "setup"][..],
            &["name", "desc", "script", "result"][..],
            &["name", "desc", "-result", "-constraints"][..],
        ] {
            assert!(
                !contract()
                    .has_unmodelled_pre_phase_effects(InvocationArguments::literals(arguments))
            );
        }
        for arguments in [
            &[
                "name",
                "desc",
                "-constraints",
                "{callback}",
                "-body",
                "script",
            ][..],
            &["name", "desc", "-output", "expected", "-body", "script"][..],
            &["name", "desc", "constraints", "script", "result"][..],
            &["name", "desc", "-body script -setup setup"][..],
        ] {
            assert!(
                contract()
                    .has_unmodelled_pre_phase_effects(InvocationArguments::literals(arguments))
            );
        }
    }

    #[test]
    fn option_like_values_are_data_and_invalid_grammar_never_enters_scripts() {
        assert!(
            selected(&["name", "desc", "-result", "-body"])
                .body
                .is_none()
        );
        for args in [
            &["name", "desc", "-body"][..],
            &["name", "desc", "-invalid", "script"][..],
        ] {
            assert_eq!(
                contract().select(InvocationArguments::literals(args)),
                BodyExecutionSelection::InvalidArguments
            );
        }
    }

    #[test]
    fn computed_script_values_do_not_hide_known_option_grammar() {
        let args = [
            InvocationWord::Literal("name"),
            InvocationWord::Literal("desc"),
            InvocationWord::Literal("-body"),
            InvocationWord::Dynamic,
        ];
        assert!(matches!(
            contract().select(InvocationArguments::structured(&args)),
            BodyExecutionSelection::CapturedLifecycle(_)
        ));
        let unknown = [
            InvocationWord::Literal("name"),
            InvocationWord::Literal("desc"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("script"),
        ];
        assert_eq!(
            contract().select(InvocationArguments::structured(&unknown)),
            BodyExecutionSelection::UnknownArguments
        );
    }

    #[test]
    fn package_hook_contracts_do_not_guess_a_release_boundary() {
        assert_eq!(
            contract().hook_policy(Some("2.3.8")),
            BodyHookPolicy::Disabled
        );
        assert_eq!(
            contract().hook_policy(Some("2.5.11")),
            BodyHookPolicy::OptionalOverride
        );
        assert_eq!(
            contract().hook_policy(Some("2.4.0")),
            BodyHookPolicy::Unknown
        );
        assert!(contract().required_absent_hooks(Some("2.3.8")).is_empty());
        assert_eq!(contract().required_absent_hooks(None).len(), 3);
    }

    #[test]
    fn package_repetition_is_independent_of_core_and_unknown_versions() {
        assert_eq!(contract().repetition(Some("2.5.11")), BodyRepetition::Once);
        assert_eq!(
            contract().repetition(Some("2.6.0")),
            BodyRepetition::MayRepeat
        );
        assert_eq!(contract().repetition(None), BodyRepetition::MayRepeat);
        assert_eq!(
            contract().repetition(Some("future")),
            BodyRepetition::MayRepeat
        );
    }
}
