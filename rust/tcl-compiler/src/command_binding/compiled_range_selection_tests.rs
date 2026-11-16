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

use super::{Arc, BTreeSet, CommandAllocationSite, SourceCommandBindings, SourceOriginId};
use crate::ir::{SourceSite, WordExpr};
use tcl_lexer::Span;

fn words(spans: &[(u32, u32)]) -> Vec<WordExpr> {
    spans
        .iter()
        .map(|&(start, end)| WordExpr::Literal {
            text: String::new(),
            source: SourceSite::source(Span::new(start, end)),
        })
        .collect()
}

fn full_filter(bindings: &SourceCommandBindings, words: &[WordExpr], head: u32) -> Vec<u32> {
    bindings
        .compiler_invocations
        .keys()
        .filter(|site| {
            bindings.root_origin.as_ref() == Some(&site.source)
                && site.offset != head
                && words.iter().any(|word| {
                    word.source().span.start() <= site.offset
                        && site.offset <= word.source().span.end()
                })
        })
        .map(|site| site.offset)
        .collect()
}

fn callback_order(compiled: Vec<u32>, head: u32) -> Vec<u32> {
    let mut retained = BTreeSet::from([1, 3]);
    let nested_offsets = BTreeSet::from([0, 2, 8]);
    let mut queried = Vec::new();
    for nested in nested_offsets.into_iter().chain(compiled) {
        if retained.insert(nested) {
            queried.push(nested);
        }
    }
    queried.push(head);
    queried
}

#[test]
fn original_ranges_match_full_filter_and_binding_order_at_boundaries() {
    let text: Arc<str> = Arc::from("root");
    let roots = [
        Arc::new(SourceOriginId::authored(&text)),
        Arc::new(SourceOriginId::authored(&text)),
        Arc::new(SourceOriginId::loaded(
            Arc::from("a"),
            Arc::from("1"),
            &text,
        )),
        Arc::new(SourceOriginId::loaded(
            Arc::from("b"),
            Arc::from("1"),
            &text,
        )),
        Arc::new(SourceOriginId::authored(&Arc::from("foreign"))),
    ];
    assert_eq!(roots[0], roots[1]);
    assert_ne!(roots[0], roots[2]);
    let mut bindings = SourceCommandBindings::default();
    for source in &roots {
        for offset in [0, 1, 2, 3, 5, 8, u32::MAX - 1, u32::MAX] {
            bindings.compiler_invocations.insert(
                CommandAllocationSite {
                    source: Arc::clone(source),
                    offset,
                },
                Vec::new(),
            );
        }
    }
    let cases: &[&[(u32, u32)]] = &[
        &[],
        &[(0, 0)],
        &[(u32::MAX, u32::MAX)],
        &[(0, u32::MAX)],
        &[(2, 5)],
        &[(8, 8), (0, 3), (2, 8), (2, 8)],
        &[(u32::MAX - 1, u32::MAX), (0, 0)],
    ];
    for root in std::iter::once(None).chain(roots.iter().cloned().map(Some)) {
        bindings.root_origin = root;
        for spans in cases {
            let words = words(spans);
            for head in [0, 2, u32::MAX] {
                let old = full_filter(&bindings, &words, head);
                let new = bindings.compiled_invocation_offsets_in_words(&words, head);
                assert_eq!(new, old);
                assert_eq!(callback_order(new, head), callback_order(old, head));
            }
        }
    }
}

#[test]
fn full_source_identity_breaks_fingerprint_collisions_without_merging_roots() {
    let mut left = SourceOriginId::authored(&Arc::from("left"));
    let mut equal = SourceOriginId::authored(&Arc::from("left"));
    let mut foreign = SourceOriginId::authored(&Arc::from("right"));
    left.fingerprint = 0;
    equal.fingerprint = 0;
    foreign.fingerprint = 0;
    let roots = [Arc::new(left), Arc::new(equal), Arc::new(foreign)];
    for left in &roots {
        for right in &roots {
            assert_eq!(left == right, left.cmp(right).is_eq());
        }
    }
    let mut bindings = SourceCommandBindings {
        root_origin: Some(Arc::clone(&roots[0])),
        ..SourceCommandBindings::default()
    };
    for (source, offset) in [(&roots[0], 1), (&roots[1], 2), (&roots[2], 3)] {
        bindings.compiler_invocations.insert(
            CommandAllocationSite {
                source: Arc::clone(source),
                offset,
            },
            Vec::new(),
        );
    }
    let words = words(&[(0, u32::MAX)]);
    assert_eq!(
        bindings.compiled_invocation_offsets_in_words(&words, 0),
        [1, 2]
    );
    assert_eq!(full_filter(&bindings, &words, 0), [1, 2]);
}
