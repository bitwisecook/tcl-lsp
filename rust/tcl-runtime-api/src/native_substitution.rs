// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C substitution-template compilation, independently of scripts.

use crate::native_compilation::{NativeCommandImplementation, NativeCompilationBinding};
use crate::{CompileError, NativeCompilationEntry, SourceImage};

/// Native substitution switches. These bits are C's public `TCL_SUBST_*` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeSubstitutionFlags(u8);
impl NativeSubstitutionFlags {
    /// Retain the three independently selected switches.
    #[must_use]
    pub const fn new(backslashes: bool, commands: bool, variables: bool) -> Self {
        Self(((backslashes as u8) * 4) | (commands as u8) | ((variables as u8) * 2))
    }
    /// Original C flag mask.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
    /// Shared template-parser switches, without word delimiters or expansion.
    #[must_use]
    pub fn lexer_flags(self) -> tcl_lexer::word_parts::SubstFlags {
        tcl_lexer::word_parts::SubstFlags {
            backslashes: self.0 & 4 != 0,
            cmds: self.0 & 1 != 0,
            vars: self.0 & 2 != 0,
            ..Default::default()
        }
    }
}

/// A template is native value bytes, not a command or procedure source word.
#[derive(Debug, Clone, Copy)]
pub struct NativeSubstitutionTarget<'a> {
    /// Complete immutable original template, including counted zero bytes.
    pub source: &'a SourceImage,
    /// Already constructed namespace of this actual substitution invocation.
    pub namespace: &'a crate::ByteNamespacePath,
}

/// Independently selected stock handler, before original source getters run.
/// Capturing it requires a unique actual row in its own interpreter snapshot.
#[derive(Debug, Clone, Copy)]
pub struct NativeSelectedSubstitutionHandler {
    interpreter: crate::native_compilation::NativeInterpreterIdentity,
    version: tcl_dialect::TclVersion,
}
impl NativeSelectedSubstitutionHandler {
    /// Capture the selected handler registration in its actual native world.
    ///
    /// # Errors
    /// Refuses foreign/missing rows, open worlds and non-stock/non-C86+ handlers.
    pub fn capture(
        snapshot: &NativeCompilationEntry,
        selected: &NativeCompilationBinding,
    ) -> Result<Self, CompileError> {
        let unavailable =
            || CompileError::Unsupported("selected native substitution handler".into());
        let Some(version) = snapshot
            .execution_point
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)
        else {
            return Err(unavailable());
        };
        if !matches!(
            version,
            tcl_dialect::TclVersion::V8_6
                | tcl_dialect::TclVersion::V9_0
                | tcl_dialect::TclVersion::V9_1
        ) || !snapshot.closed
            || snapshot
                .commands
                .iter()
                .filter(|row| *row == selected)
                .count()
                != 1
            || !matches!(&selected.implementation, NativeCommandImplementation::Registry { identity, .. } if identity == "subst")
        {
            return Err(unavailable());
        }
        Ok(Self {
            interpreter: snapshot.interpreter,
            version,
        })
    }
}

/// Actual stock substitution handler and its independent compilation world.
/// It cannot be used as a script/procedure compilation entry.
#[derive(Debug, Clone, Copy)]
pub struct NativeSubstitutionCompilationEntry<'a> {
    snapshot: &'a NativeCompilationEntry,
    version: tcl_dialect::TclVersion,
    flags: NativeSubstitutionFlags,
}
impl<'a> NativeSubstitutionCompilationEntry<'a> {
    /// Issue a C86+ template compiler entry from the independently selected
    /// actual handler registration and interpreter snapshot. Catalogue lookup
    /// or a same-spelled custom command cannot provide that registration.
    ///
    /// # Errors
    /// Refuses other handlers, unproved native string/engine axes, an open
    /// command world, or engines whose Subst implementation is not compiled.
    pub fn from_selected_handler(
        snapshot: &'a NativeCompilationEntry,
        selected: NativeSelectedSubstitutionHandler,
        flags: NativeSubstitutionFlags,
    ) -> Result<Self, CompileError> {
        let unavailable = || CompileError::Unsupported("native substitution compiler entry".into());
        let Some(version) = snapshot
            .execution_point
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)
        else {
            return Err(unavailable());
        };
        if !matches!(
            version,
            tcl_dialect::TclVersion::V8_6
                | tcl_dialect::TclVersion::V9_0
                | tcl_dialect::TclVersion::V9_1
        ) || !snapshot.closed
            || snapshot.source_string_protocol
                != Some(tcl_syntax::native_string::NativeStringProtocol::C(version))
            || selected.interpreter != snapshot.interpreter
            || selected.version != version
        {
            return Err(unavailable());
        }
        Ok(Self {
            snapshot,
            version,
            flags,
        })
    }
    /// Original interpreter world; this does not convert the template to a script.
    #[must_use]
    pub const fn snapshot(self) -> &'a NativeCompilationEntry {
        self.snapshot
    }
    /// Actual independently selected native compiler release.
    #[must_use]
    pub const fn version(self) -> tcl_dialect::TclVersion {
        self.version
    }
    /// Complete flag mask retained in the original substcode cache.
    #[must_use]
    pub const fn flags(self) -> NativeSubstitutionFlags {
        self.flags
    }
}
