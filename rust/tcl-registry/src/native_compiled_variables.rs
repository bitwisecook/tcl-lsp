// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently authenticated compiled-local policies and logical providers.

use crate::InvocationDialect;
use tcl_dialect::TclVersion;
pub use tcl_syntax::naming::{
    NativeCompiledVariableAuthority, NativeCompiledVariableEnvironment,
    NativeCompiledVariableLookup, NativeCompiledVariableProtocol, NativeCompiledVariableRecipe,
};

/// Explicit logical compiler provider, independent of the actual host compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalCompiledVariableProvider {
    /// Authored F5 Tcl8.4 core compiled-local simulation.
    Tcl84CoreSimulation,
}

impl InvocationDialect {
    /// Issue a compiled-local policy from the actual audited native engine.
    /// Logical/source compatibility and native numeral availability supply no
    /// compiler evidence. Conflicting retained engine metadata withdraws it.
    #[must_use]
    pub fn native_compiled_variable_protocol(self) -> Option<NativeCompiledVariableProtocol> {
        let point = self.execution_point()?;
        if self
            .tcl_version
            .is_some_and(|version| point.tcl_version() != Some(version))
        {
            return None;
        }
        NativeCompiledVariableProtocol::for_native_point(point)
    }

    /// Validate a separately requested F5 logical compiler provider.
    /// This does not authenticate its physical C host or runtime frame.
    #[must_use]
    pub fn authored_logical_compiled_variable_protocol(
        self,
        provider: LogicalCompiledVariableProvider,
    ) -> Option<NativeCompiledVariableProtocol> {
        self.authored_f5_tcl84_core()?;
        match provider {
            LogicalCompiledVariableProvider::Tcl84CoreSimulation => Some(
                NativeCompiledVariableProtocol::authored_tcl(TclVersion::V8_4),
            ),
        }
    }

    /// Select an actual compiler or an explicitly requested logical provider.
    /// An invalid explicit request does not silently select the native host.
    #[must_use]
    pub fn compiled_variable_protocol(
        self,
        logical: Option<LogicalCompiledVariableProvider>,
    ) -> Option<NativeCompiledVariableProtocol> {
        match logical {
            Some(provider) => self.authored_logical_compiled_variable_protocol(provider),
            None => self.native_compiled_variable_protocol(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{DialectPoint, Release};

    #[test]
    fn canonical_native_compilers_and_jim_have_distinct_recipes() {
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let policy = dialect.native_compiled_variable_protocol().unwrap();
            assert_eq!(policy.authority(), NativeCompiledVariableAuthority::Native);
            assert_eq!(policy.recipe(), NativeCompiledVariableRecipe::C(version));
        }
        let dialect = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        let policy = dialect.native_compiled_variable_protocol().unwrap();
        assert_eq!(policy.recipe(), NativeCompiledVariableRecipe::Jim084);
        assert!(!policy.has_indexed_locals());
    }

    #[test]
    fn f5_compiler_simulation_requires_an_explicit_consistent_provider() {
        let f5 = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        assert!(f5.native_compiled_variable_protocol().is_none());
        assert!(f5.compiled_variable_protocol(None).is_none());
        let provider = LogicalCompiledVariableProvider::Tcl84CoreSimulation;
        let logical = f5.compiled_variable_protocol(Some(provider)).unwrap();
        assert_eq!(
            logical.authority(),
            NativeCompiledVariableAuthority::AuthoredSimulation
        );
        assert_eq!(
            logical.recipe(),
            NativeCompiledVariableRecipe::C(TclVersion::V8_4)
        );
        let native = InvocationDialect::for_version(TclVersion::V9_0);
        assert!(native.compiled_variable_protocol(Some(provider)).is_none());
        let mut conflicting = f5;
        conflicting.tcl_version = Some(TclVersion::V9_0);
        assert!(
            conflicting
                .compiled_variable_protocol(Some(provider))
                .is_none()
        );
    }

    #[test]
    fn unknown_and_conflicting_compiler_metadata_never_borrow_compatibility() {
        let mut unknown = InvocationDialect::for_version(TclVersion::V9_0);
        unknown.core_point = None;
        unknown.native_family = None;
        assert!(unknown.native_compiled_variable_protocol().is_none());
        let mut conflicting =
            InvocationDialect::of_point(DialectPoint::for_tcl_version(TclVersion::V8_4));
        conflicting.tcl_version = Some(TclVersion::V9_0);
        assert!(conflicting.native_compiled_variable_protocol().is_none());
        let old_jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_79));
        assert!(old_jim.native_compiled_variable_protocol().is_none());
    }
}
