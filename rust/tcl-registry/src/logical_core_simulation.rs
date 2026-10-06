// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit logical core simulation context, independent of native authority.

/// Consistent F5 Tcl 8.4 logical context in which an authored provider may run.
/// The capability supplies no actual C interpreter, object or compiler proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredF5Tcl84Core {
    _private: (),
}

impl crate::InvocationDialect {
    /// Validate the logical context for separately authored F5 core providers.
    /// A provider still has to be requested and selected for each semantic axis.
    #[must_use]
    pub fn authored_f5_tcl84_core(self) -> Option<AuthoredF5Tcl84Core> {
        use tcl_dialect::model::Family;
        let family = self.native_family.or(self
            .core_point
            .map(tcl_dialect::model::DialectPoint::family))?;
        if !matches!(family, Family::F5Tcl | Family::F5Irules)
            || self.tcl_version != Some(tcl_dialect::TclVersion::V8_4)
            || self.core_point.is_some_and(|point| {
                point.family() != family
                    || point.tcl_version() != Some(tcl_dialect::TclVersion::V8_4)
            })
        {
            return None;
        }
        Some(AuthoredF5Tcl84Core { _private: () })
    }
}
