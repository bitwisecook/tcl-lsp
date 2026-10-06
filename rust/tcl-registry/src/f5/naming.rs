// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact build and event admission for independently measured name projections.

use super::{BigIpExecutionContext, evidence::BigIpBuild};
use tcl_syntax::naming::{MeasuredBigIpNameScope, ObservedBigIpNamePolicy};

/// Actual event retained independently of the feature's interpreter context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BigIpNameEvent {
    /// The reached HTTP_REQUEST event, independent of source dialect labels.
    HttpRequest,
    /// Other or unavailable event evidence cannot inherit HTTP name observations.
    Unmeasured,
}

/// Select finite naming facts only for their exact observed execution boundary.
/// Software compatibility, reported Tcl patchlevel and source dialect do not select it.
#[must_use]
pub fn observed_name_policy(
    build: BigIpBuild,
    context: BigIpExecutionContext,
    event: BigIpNameEvent,
) -> Option<ObservedBigIpNamePolicy> {
    (build == BigIpBuild::MEASURED_21_1_0_1
        && context == BigIpExecutionContext::TmmIRule
        && event == BigIpNameEvent::HttpRequest)
        .then(|| {
            ObservedBigIpNamePolicy::for_measured_scope(
                MeasuredBigIpNameScope::BigIp21_1_0_1Build0_0_26TmmHttpRequest,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_names_do_not_cross_build_feature_or_event_contexts() {
        let build = BigIpBuild::MEASURED_21_1_0_1;
        assert!(
            observed_name_policy(
                build,
                BigIpExecutionContext::TmmIRule,
                BigIpNameEvent::HttpRequest
            )
            .is_some()
        );
        for context in BigIpExecutionContext::ALL {
            if context != BigIpExecutionContext::TmmIRule {
                assert_eq!(
                    observed_name_policy(build, context, BigIpNameEvent::HttpRequest),
                    None
                );
            }
        }
        assert_eq!(
            observed_name_policy(
                build,
                BigIpExecutionContext::TmmIRule,
                BigIpNameEvent::Unmeasured
            ),
            None
        );
        for other in [
            BigIpBuild {
                release: "21.1.0.2",
                build: "0.0.26",
            },
            BigIpBuild {
                release: "21.1.0.1",
                build: "0.0.27",
            },
        ] {
            assert_eq!(
                observed_name_policy(
                    other,
                    BigIpExecutionContext::TmmIRule,
                    BigIpNameEvent::HttpRequest
                ),
                None
            );
        }
    }
}
