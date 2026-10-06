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

//! Jim oracle discovery and behavioural capability selection.
//! Extensions are build options, so version or command presence cannot stand
//! in for executing the operation a conformance scenario requires.

use std::path::PathBuf;

use crate::{OracleError, run_script, which_on_path};

/// Current upstream reference selection, refreshed deliberately after probing.
/// The reported patchlevel includes the built source revision; a release-only
/// `0.84` binary is insufficient for a requested current-upstream lane.
pub const JIM_REFERENCE_MANIFEST: &str = include_str!("../jim-reference.txt");

/// One pinned upstream Jim build, independently of its configurable features.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JimReference {
    /// Upstream source repository URL.
    pub repository: &'static str,
    /// Full source commit used by the provisioning helper.
    pub revision: &'static str,
    /// Upstream author/commit timestamp recorded with the revision.
    pub commit_date: &'static str,
    /// Expected Jim major/minor version.
    pub version: &'static str,
    /// Expected revision-bearing patchlevel; release-only versions do not match.
    pub patchlevel: &'static str,
}

/// The same manifest used by `scripts/dev/ensure-jim-oracle.sh`.
#[must_use]
pub fn jim_reference() -> JimReference {
    let field = |name: &str| {
        JIM_REFERENCE_MANIFEST
            .lines()
            .find_map(|line| {
                line.split_once('=')
                    .and_then(|(key, value)| (key == name).then_some(value))
            })
            .unwrap_or_else(|| panic!("Jim reference manifest lacks {name}"))
    };
    JimReference {
        repository: field("repository"),
        revision: field("revision"),
        commit_date: field("commit_date"),
        version: field("version"),
        patchlevel: field("patchlevel"),
    }
}

/// Operations measured against the selected binary's actual extensions.
/// A present `interp` command does not imply C Tcl's `interp alias` API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JimCapability {
    /// Namespace creation and deletion.
    Namespace,
    /// Export/import creation and dispatch; identity semantics remain separately tested.
    NamespaceImport,
    /// Namespace command search paths.
    NamespacePath,
    /// C-style child interpreter creation/deletion.
    InterpCreate,
    /// C-style interpreter aliases.
    InterpAlias,
    /// Jim child creation and evaluation through the returned command handle.
    InterpHandleCreate,
    /// A Jim child-handle alias invoking a parent command with retained arguments.
    InterpHandleAlias,
    /// Execution trace registration/removal.
    ExecutionTrace,
    /// Command lifecycle trace registration/removal.
    CommandTrace,
    /// Variable trace registration/removal.
    VariableTrace,
    /// Lambda invocation.
    Apply,
    /// Argument expansion with `{*}`.
    Expansion,
    /// Package provision.
    PackageProvide,
    /// Requiring an already-provided package.
    PackageRequire,
    /// Registering and invoking an ifneeded loader.
    PackageIfneeded,
    /// Package preference selection.
    PackagePrefer,
    /// The standard-distribution `auto_qualify` helper.
    Autoload,
}

impl JimCapability {
    /// Every currently modelled probe, in stable order.
    pub const ALL: [Self; 17] = [
        Self::Namespace,
        Self::NamespaceImport,
        Self::NamespacePath,
        Self::InterpCreate,
        Self::InterpAlias,
        Self::InterpHandleCreate,
        Self::InterpHandleAlias,
        Self::ExecutionTrace,
        Self::CommandTrace,
        Self::VariableTrace,
        Self::Apply,
        Self::Expansion,
        Self::PackageProvide,
        Self::PackageRequire,
        Self::PackageIfneeded,
        Self::PackagePrefer,
        Self::Autoload,
    ];

    /// Stable spelling used by shared vector capability requirements.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Namespace => "namespace",
            Self::NamespaceImport => "namespace-import",
            Self::NamespacePath => "namespace-path",
            Self::InterpCreate => "interp-create",
            Self::InterpAlias => "interp-alias",
            Self::InterpHandleCreate => "interp-handle-create",
            Self::InterpHandleAlias => "interp-handle-alias",
            Self::ExecutionTrace => "execution-trace",
            Self::CommandTrace => "command-trace",
            Self::VariableTrace => "variable-trace",
            Self::Apply => "apply",
            Self::Expansion => "expansion",
            Self::PackageProvide => "package-provide",
            Self::PackageRequire => "package-require",
            Self::PackageIfneeded => "package-ifneeded",
            Self::PackagePrefer => "package-prefer",
            Self::Autoload => "autoload",
        }
    }

    /// Parse a capability requirement; unknown names are not supported features.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|capability| capability.name() == name)
    }

    const fn probe(self) -> &'static str {
        match self {
            Self::Namespace => "namespace eval ::probe {}; namespace delete ::probe",
            Self::NamespaceImport => {
                "namespace eval ::probe {proc command {} {return ok}; namespace export command}; namespace eval ::probeImport {namespace import ::probe::command; command}"
            }
            Self::NamespacePath => "namespace path",
            Self::InterpCreate => "set child [interp create]; interp delete $child",
            Self::InterpAlias => "interp alias {} ::probeAlias {} list; rename ::probeAlias {}",
            Self::InterpHandleCreate => {
                "set child [interp]; set result [$child eval {set probe ok}]; $child delete; if {$result ne {ok}} {error {child eval mismatch}}"
            }
            Self::InterpHandleAlias => {
                "set child [interp]; $child alias probe list prefix; set result [$child eval {probe arg}]; $child delete; if {$result ne {prefix arg}} {error {child alias mismatch}}"
            }
            Self::ExecutionTrace => {
                "proc ::probeTrace {} {}; trace add execution ::probeTrace enter list; trace remove execution ::probeTrace enter list"
            }
            Self::CommandTrace => {
                "proc ::probeCommandTrace {} {}; trace add command ::probeCommandTrace delete list; trace remove command ::probeCommandTrace delete list"
            }
            Self::VariableTrace => {
                "set ::probeVariable 1; trace add variable ::probeVariable write list; trace remove variable ::probeVariable write list"
            }
            Self::Apply => "apply {{} {return ok}}",
            Self::Expansion => "list {*}{a b}",
            Self::PackageProvide => "package provide ProbePackage 1.0",
            Self::PackageRequire => {
                "package provide ProbePackage 1.0; package require ProbePackage"
            }
            Self::PackageIfneeded => {
                "package ifneeded ProbePackage 1.0 {package provide ProbePackage 1.0}; package require ProbePackage 1.0"
            }
            Self::PackagePrefer => "package prefer",
            Self::Autoload => "auto_qualify probe ::",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Reported Jim identity plus measured features, never an assumed C Tcl level.
pub struct Jimsh {
    pub version: String,
    pub patchlevel: String,
    pub path: PathBuf,
    pub capabilities: Vec<JimCapability>,
}

impl Jimsh {
    #[must_use]
    pub fn supports(&self, capability: JimCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// Locate an optional Jim binary, rejecting a malformed explicit override.
///
/// Extensions must be selected from `capabilities`, rather than inferred from
/// `version` or the presence of a similarly named command.
pub fn locate_jimsh() -> Result<Option<Jimsh>, OracleError> {
    let path = if let Some(explicit) = std::env::var_os("TCL_LSP_JIMSH") {
        let path = PathBuf::from(explicit);
        if !path.is_file() {
            return Err(OracleError::InvalidOverride(format!(
                "TCL_LSP_JIMSH={} is not a file",
                path.display()
            )));
        }
        path
    } else if let Some(path) = which_on_path("jimsh") {
        path
    } else {
        return Ok(None);
    };
    let identity = run_script(&path, b"puts [info version]\nputs [info patchlevel]\n")?
        .strict_text()
        .map_err(OracleError::InvalidInterpreter)?;
    let mut lines = identity.lines();
    let version = lines.next().unwrap_or_default().to_owned();
    let patchlevel = lines.next().unwrap_or_default().to_owned();
    if version.is_empty() || patchlevel.is_empty() || lines.next().is_some() {
        return Err(OracleError::InvalidInterpreter(format!(
            "{} does not report a Jim identity",
            path.display()
        )));
    }
    let mut capabilities = Vec::new();
    for capability in JimCapability::ALL {
        let script = format!("puts [catch {{{}}}]\n", capability.probe());
        let result = run_script(&path, script.as_bytes())?
            .strict_text()
            .map_err(OracleError::InvalidInterpreter)?;
        match result.as_str() {
            "0" => capabilities.push(capability),
            "1" => {}
            _ => {
                return Err(OracleError::InvalidInterpreter(format!(
                    "{} returned invalid capability result {result:?}",
                    path.display()
                )));
            }
        }
    }
    Ok(Some(Jimsh {
        version,
        patchlevel,
        path,
        capabilities,
    }))
}

/// Select the pinned current upstream Jim and probe its extension surface.
///
/// ```no_run
/// use tcl_test_support::{JimCapability, require_jimsh, run_script};
/// # fn example() -> Result<(), tcl_test_support::OracleError> {
/// let jim = require_jimsh()?;
/// if jim.supports(JimCapability::NamespaceImport) {
///     let outcome = run_script(&jim.path, b"puts [namespace current]\n")?;
///     assert!(outcome.success());
/// }
/// # Ok(()) }
/// ```
pub fn require_jimsh() -> Result<Jimsh, OracleError> {
    let interpreter = locate_jimsh()?.ok_or_else(|| {
        OracleError::InvalidInterpreter(
            "missing required Jim Tcl oracle; set TCL_LSP_JIMSH".to_owned(),
        )
    })?;
    let reference = jim_reference();
    if interpreter.version != reference.version || interpreter.patchlevel != reference.patchlevel {
        return Err(OracleError::InvalidInterpreter(format!(
            "{} reports Jim {} {}, expected {} {} from {}",
            interpreter.path.display(),
            interpreter.version,
            interpreter.patchlevel,
            reference.version,
            reference.patchlevel,
            reference.revision
        )));
    }
    Ok(interpreter)
}
