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

use super::*;
use std::cell::Cell;

#[test]
fn shared_fallback_retains_the_same_unavailable_value_without_repreparing() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let ordinary = OriginalSourceTransitionAdviceCache::default();
    let calls = OriginalSourceTransitionAdviceCache::default();
    let prepared = ordinary.get_or_prepare(context, || None);
    let attempts = Cell::new(0);
    let selected = calls.get_or_prepare_with_shared_fallback(
        context,
        || {
            attempts.set(attempts.get() + 1);
            None
        },
        || ordinary.get_or_prepare(context, || panic!("ordinary value already retained")),
    );
    assert!(Arc::ptr_eq(&prepared, &selected));
    assert!(selected.is_none());
    let repeated = calls.get_or_prepare_with_shared_fallback(
        context,
        || panic!("call cache already retained"),
        || panic!("call cache already retained"),
    );
    assert!(Arc::ptr_eq(&selected, &repeated));
    assert_eq!(attempts.get(), 1);
}

#[test]
fn preferred_preparation_never_consults_the_shared_fallback() {
    // Cache mechanics only: the empty test tape asserts no source outcome.
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let calls = OriginalSourceTransitionAdviceCache::default();
    let prepared = calls.get_or_prepare_with_shared_fallback(
        context,
        || Some(OriginalSourceTransitionAdviceTape::default()),
        || panic!("a present preferred tape must not consult the fallback"),
    );
    assert!(prepared.is_some());
    let repeated = calls.get_or_prepare_with_shared_fallback(
        context,
        || panic!("preferred tape already retained"),
        || panic!("preferred tape already retained"),
    );
    assert!(Arc::ptr_eq(&prepared, &repeated));
}

#[test]
fn non_normal_source_reuses_its_actual_ordinary_tape() {
    let analysis =
        crate::analyser::Analyser::new().analyse("error source_tape_cache_control", "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let realm = analysis.retained_command_realm().unwrap();
    assert!(
        realm
            .bindings
            .original_normal_source_procedure_advice_tape(&context)
            .is_none()
    );
    let ordinary = realm.source_advice.get_or_prepare_shared_for(
        &context,
        crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
        || {
            realm
                .bindings
                .original_source_transition_advice_tape_shared(&context)
        },
    );
    assert!(ordinary.is_some());
    let selected = realm.prepared_source_procedure_advice(&context);
    assert!(Arc::ptr_eq(&ordinary, &selected));
    assert!(
        realm
            .bindings
            .original_normal_source_procedure_advice_tape(&context)
            .is_none()
    );
}
