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

//! Conservative caller-source effects for local storage and SSA escape analysis.

use super::{CallerAliasSelection, caller_alias_selection};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use tcl_core_types::BodyFrameId;
use tcl_registry::{CallerFrameSelection, FrameLevel, VariableCellAliasTransition};

pub(crate) enum CallerSource<'a> {
    None,
    DynamicFrame,
    DynamicName,
    Literal(&'a str),
}

pub(crate) struct AliasEscape<'a> {
    pub local: Option<&'a str>,
    pub source: CallerSource<'a>,
}

impl<'a> AliasEscape<'a> {
    pub(crate) fn from_transition(
        alias: &'a VariableCellAliasTransition,
        parse_level: impl FnOnce(&CallerFrameSelection, &str) -> Option<FrameLevel>,
    ) -> Self {
        let local = alias.local.literal();
        let source = match caller_alias_selection(&alias.target, parse_level) {
            CallerAliasSelection::Other
            | CallerAliasSelection::Selected(
                FrameLevel::Relative(0) | FrameLevel::Absolute(0),
                _,
            ) => CallerSource::None,
            CallerAliasSelection::Dynamic => CallerSource::DynamicFrame,
            // The escape projection retains invalid or unplaceable levels
            // conservatively, independently of the CFG's selected writes.
            CallerAliasSelection::Selected(_, name) | CallerAliasSelection::Invalid(name) => {
                name.map_or(CallerSource::DynamicName, CallerSource::Literal)
            }
        };
        Self { local, source }
    }
}

#[derive(Clone)]
pub(crate) struct EscapeFrameSources {
    pub names: BTreeSet<String>,
    pub unbounded: bool,
}

/// Flow caller-source escapes to every transitive caller. Even when an
/// ordinary call shifts the exact target frame, local storage keeps the
/// conservative set of names that may have to be spilled across the chain.
pub(crate) fn transitive_escape_frame_sources(
    mut effects: HashMap<BodyFrameId, EscapeFrameSources>,
    callees: &HashMap<BodyFrameId, HashSet<BodyFrameId>>,
) -> HashMap<BodyFrameId, EscapeFrameSources> {
    let mut reverse_edges: HashMap<_, Vec<_>> = HashMap::new();
    for (&caller, targets) in callees {
        for &callee in targets {
            reverse_edges.entry(callee).or_default().push(caller);
        }
    }
    let mut worklist: VecDeque<_> = effects.keys().copied().collect();
    let mut queued: HashSet<_> = effects.keys().copied().collect();
    while let Some(id) = worklist.pop_front() {
        queued.remove(&id);
        let current = effects[&id].clone();
        for caller in reverse_edges.get(&id).into_iter().flatten() {
            let next = effects.get_mut(caller).expect("caller in summaries");
            let prior = (next.names.len(), next.unbounded);
            next.names.extend(current.names.iter().cloned());
            next.unbounded |= current.unbounded;
            if prior != (next.names.len(), next.unbounded) && queued.insert(*caller) {
                worklist.push_back(*caller);
            }
        }
    }
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::{
        AliasWords, InvocationWordKind, TransitionSubject, VariableAliasDestination,
        VariableAliasTarget,
    };

    #[test]
    fn variable_escape_preserves_selected_alias_source_boundaries() {
        use CallerFrameSelection::{DefaultCaller, Explicit};
        use TransitionSubject::{Literal, Unknown};

        fn project(alias: &VariableCellAliasTransition) -> AliasEscape<'_> {
            AliasEscape::from_transition(alias, |_, level| {
                FrameLevel::parse_in(level, tcl_registry::default_registry())
            })
        }

        let unknown = Unknown {
            argument_index: 0,
            word_kind: InvocationWordKind::Dynamic,
        };
        let source = Literal("caller".to_owned());
        let mut alias = VariableCellAliasTransition {
            destination: VariableAliasDestination::CurrentNamespaceOrLocal,
            local: Literal("local".to_owned()),
            target: VariableAliasTarget::CallerSelectedFrame {
                frame: DefaultCaller,
                variable: source.clone(),
            },
            writes_value: false,
            words: AliasWords {
                local: 1,
                target: 0,
            },
        };
        let default = project(&alias);
        assert_eq!(default.local, Some("local"));
        assert!(matches!(default.source, CallerSource::Literal("caller")));

        for level in ["0", "#0"] {
            alias.target = VariableAliasTarget::CallerSelectedFrame {
                frame: Explicit(Literal(level.to_owned())),
                variable: source.clone(),
            };
            assert!(matches!(project(&alias).source, CallerSource::None));
        }

        alias.target = VariableAliasTarget::CallerSelectedFrame {
            frame: Explicit(unknown.clone()),
            variable: source,
        };
        assert!(matches!(project(&alias).source, CallerSource::DynamicFrame));

        alias.target = VariableAliasTarget::CallerSelectedFrame {
            frame: DefaultCaller,
            variable: unknown,
        };
        assert!(matches!(project(&alias).source, CallerSource::DynamicName));
    }
}
