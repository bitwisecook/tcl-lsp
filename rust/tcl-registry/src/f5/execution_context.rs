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

//! BIG-IP execution contexts key runtime facts, command surfaces and storage.
//!
//! TMM, tmsh, iApp implementation and iCall agree on the measured F5 grammar cases,
//! while each retains its own environment and evidence. Host Tcl is an
//! independent control. APL and its Tcl callbacks are unmeasured and cannot
//! inherit facts from implementation scripts. A measured context can still
//! have an unknown build profile or no registered environment.

pub use tcl_dialect::model::bigip_execution_context::{BigIpExecutionContext, ContextMeasurement};

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::Family;
    use tcl_dialect::model::family::{BuildProfileId, CapabilityAnswer};

    #[test]
    fn execution_contexts_are_distinct_and_named_as_the_transcript_names_them() {
        let mut seen = std::collections::HashSet::new();
        for context in BigIpExecutionContext::ALL {
            assert!(seen.insert(context.as_str()), "{context}: duplicate label");
            assert_eq!(context.to_string(), context.as_str());
        }
        assert_eq!(seen.len(), 7);
    }

    /// §11: four contexts measured, two `Unknown` — and the two unmeasured
    /// ones stay unknown all the way down. No family, no environment, no
    /// core profile, and a build profile whose capability answers are
    /// `Unknown` rather than the canonical column.
    #[test]
    fn unmeasured_contexts_stay_unknown_all_the_way_down() {
        let measured: Vec<_> = BigIpExecutionContext::ALL
            .into_iter()
            .filter(|c| c.measurement().is_measured())
            .collect();
        assert_eq!(
            measured,
            vec![
                BigIpExecutionContext::TmmIRule,
                BigIpExecutionContext::TmshCliScript,
                BigIpExecutionContext::IAppImplementation,
                BigIpExecutionContext::ICallScript,
                BigIpExecutionContext::HostShellTcl,
            ]
        );

        for context in [
            BigIpExecutionContext::IAppPresentationApl,
            BigIpExecutionContext::IAppPresentationTclCallback,
        ] {
            assert!(!context.measurement().is_measured(), "{context}");
            assert_eq!(context.family(), None, "{context}");
            assert_eq!(context.environment_name(), None, "{context}");
            assert_eq!(context.core_profile(), None, "{context}");
            assert_eq!(
                context.build_profile(),
                BuildProfileId::Unknown,
                "{context}"
            );
        }

        // The APL contexts sit inside the same template as
        // `IAppImplementation`; that proximity must not leak its row.
        let implementation = BigIpExecutionContext::IAppImplementation;
        assert_eq!(implementation.family(), Some(Family::F5Tcl));
        assert_eq!(implementation.environment_name(), Some("f5-iapps"));
        for apl in [
            BigIpExecutionContext::IAppPresentationApl,
            BigIpExecutionContext::IAppPresentationTclCallback,
        ] {
            assert!(!implementation.promotes_facts_to(apl));
            assert!(!apl.promotes_facts_to(implementation));
        }
    }

    /// §4a's three consequences, as type facts: the three F5 contexts
    /// share one parser but not one environment, the host `tclsh` is not
    /// an F5 context at all, and no fact crosses a context boundary.
    #[test]
    fn contexts_key_surface_and_environment_not_grammar() {
        let irule = BigIpExecutionContext::TmmIRule;
        let tmsh = BigIpExecutionContext::TmshCliScript;
        let iapp = BigIpExecutionContext::IAppImplementation;
        let icall = BigIpExecutionContext::ICallScript;

        // One parser: the iRules offshoot inherits the trunk grammar
        // whole, so every lexical axis agrees across the three.
        let grammar_of = |context: BigIpExecutionContext| {
            let id = context.core_profile().expect("measured context");
            tcl_dialect::model::family::grammar(id.family(), id.release)
        };
        assert_eq!(grammar_of(irule), grammar_of(tmsh));
        assert_eq!(grammar_of(tmsh), grammar_of(iapp));
        assert_eq!(grammar_of(iapp), grammar_of(icall));
        assert_eq!(icall.environment_name(), None);
        assert_eq!(icall.build_profile(), BuildProfileId::Unknown);
        // …and every one of them differs from the host build.
        assert_ne!(
            grammar_of(irule),
            grammar_of(BigIpExecutionContext::HostShellTcl)
        );

        // Not one environment: distinct environments, and the iApp host
        // is a 32-bit build of the same trunk (`wordSize 4`, §4).
        assert_eq!(irule.environment_name(), Some("f5-irules"));
        assert_eq!(tmsh.environment_name(), Some("f5-tmsh"));
        assert_eq!(iapp.environment_name(), Some("f5-iapps"));
        assert_eq!(iapp.build_profile(), BuildProfileId::F5Scriptd32);
        assert_eq!(
            iapp.core_profile()
                .expect("measured")
                .resolve()
                .capabilities
                .word_size_64,
            CapabilityAnswer::No
        );
        assert_eq!(
            tmsh.core_profile()
                .expect("measured")
                .resolve()
                .capabilities
                .word_size_64,
            CapabilityAnswer::Unknown
        );

        // The host `tclsh` is provenance only.
        let host = BigIpExecutionContext::HostShellTcl;
        assert!(!host.is_appliance_hosted());
        assert_eq!(host.environment_name(), None);
        assert_eq!(host.family(), Some(Family::Tcl));

        // No fact ever crosses a context boundary (F1/F4).
        for a in BigIpExecutionContext::ALL {
            for b in BigIpExecutionContext::ALL {
                assert_eq!(a.promotes_facts_to(b), a == b, "{a} -> {b}");
            }
        }
    }

    /// APL is a presentation DSL, not a Tcl dialect (F1's required
    /// change): its keywords must never become Tcl commands. The Tcl
    /// callback nested *inside* it is Tcl — but an unmeasured one.
    #[test]
    fn apl_is_not_tcl_but_its_callback_is() {
        assert!(!BigIpExecutionContext::IAppPresentationApl.is_tcl());
        assert!(BigIpExecutionContext::IAppPresentationTclCallback.is_tcl());
        for context in BigIpExecutionContext::ALL {
            if context != BigIpExecutionContext::IAppPresentationApl {
                assert!(context.is_tcl(), "{context}");
            }
        }
    }
}
