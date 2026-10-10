// SPDX-License-Identifier: AGPL-3.0-or-later
//! Version reporting has independent C-global and Jim-build purposes.

/// Selected version-reporting operand source, independently of variable lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeInfoVersionSource {
    /// C reads this live global with `TCL_GLOBAL_ONLY`.
    Global(&'static str),
    /// Jim without an independently supplied build description reports its core release.
    /// This is not a source checkout, executable hash or git build-version claim.
    CoreRelease(&'static str),
}

impl crate::InvocationDialect {
    /// Select the actual info patchlevel or core-version reporting purpose.
    /// C's core-version operation is tclversion; Jim's is version. Supplying
    /// the wrong member for that actual engine does not select a report source.
    #[must_use]
    pub fn native_info_version_source(self, member: &str) -> Option<NativeInfoVersionSource> {
        use tcl_dialect::model::{Family, Release};
        match (self.family()?, member) {
            (Family::Tcl, "patchlevel") if self.tcl_version.is_some() => {
                Some(NativeInfoVersionSource::Global("tcl_patchLevel"))
            }
            (Family::Tcl, "tclversion") if self.tcl_version.is_some() => {
                Some(NativeInfoVersionSource::Global("tcl_version"))
            }
            (Family::Jim, "patchlevel" | "version")
                if self.core_point?.release() == Release::JIM_0_84 =>
            {
                Some(NativeInfoVersionSource::CoreRelease("0.84"))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_version_report_keeps_jim_independent_of_tcl_globals() {
        // Source proof: naming.info.version-source-owner
        // docs/design/analysis/name-resolution-proofs/info-version-source-owner.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = crate::model::ingress::resolve_environment(engine).unit_profile();
            let dialect = crate::InvocationDialect::of_profile(profile);
            if engine == "jim" {
                assert_eq!(
                    dialect.native_info_version_source("patchlevel"),
                    Some(NativeInfoVersionSource::CoreRelease("0.84"))
                );
                assert_eq!(
                    dialect.native_info_version_source("version"),
                    Some(NativeInfoVersionSource::CoreRelease("0.84"))
                );
                assert_eq!(dialect.native_info_version_source("tclversion"), None);
            } else {
                assert_eq!(
                    dialect.native_info_version_source("patchlevel"),
                    Some(NativeInfoVersionSource::Global("tcl_patchLevel"))
                );
                assert_eq!(
                    dialect.native_info_version_source("tclversion"),
                    Some(NativeInfoVersionSource::Global("tcl_version"))
                );
                assert_eq!(dialect.native_info_version_source("version"), None);
            }
        }
    }
}
