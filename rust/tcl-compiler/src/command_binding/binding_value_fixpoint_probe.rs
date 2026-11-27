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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use tcl_lexer::{SourceImage, Span};

use super::{
    ModuleCommandBindings, SourceOriginId, SourceOriginKind, native_result::EvaluatedSourceValue,
};
use crate::analyser::{Analyser, ResolvedAnalysisInput};
use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
use crate::realm::CommandBindingRealm;
use crate::ssa::original_source_read_at_unit;
use tcl_source::segment::SegmentedCommand;
use tcl_source::segment::segment_commands_incremental;
use tcl_source::segment::segment_commands_with_offset_and_config;
use tcl_source::words::WordExpr;

const LIMIT: usize = 4;
const DIALECT: &str = "tcl8.6";

type Values = BTreeMap<(u32, u32), String>;

struct OperandRound {
    image: SourceImage,
    values: Values,
    missing: BTreeSet<(u32, u32)>,
}

thread_local! {
    static ROUND: RefCell<Option<OperandRound>> = const { RefCell::new(None) };
}

pub(super) fn active() -> bool {
    ROUND.with_borrow(Option::is_some)
}

/// Follow a single original variable substitution while preserving the walk's
/// actual normal and abrupt routes. The suggestion supplies text only.
pub(super) fn operand_value(
    word: &WordExpr,
    state: &ModuleCommandBindings,
    value: Option<Arc<EvaluatedSourceValue>>,
    has_name_value: bool,
) -> Option<Arc<EvaluatedSourceValue>> {
    if value.is_some() || has_name_value {
        return value;
    }
    let WordExpr::Variable { source, spelling } = word else {
        return None;
    };
    ROUND.with_borrow_mut(|round| {
        let round = round.as_mut()?;
        let origin = state.current_source_origin.as_ref()?;
        if !matches!(origin.kind(), SourceOriginKind::Authored(image) if *image == round.image)
            || round.image.try_text().ok()?.get(source.span.as_range()) != Some(spelling)
        {
            return None;
        }
        let key = (source.span.start(), source.span.end());
        round.missing.insert(key);
        Some(Arc::new(EvaluatedSourceValue {
            text: round.values.get(&key)?.clone(),
            representation: tcl_syntax::value::ValueRepresentation::Unknown,
            numeric: None,
        }))
    })
}

#[derive(Debug, PartialEq, Eq)]
struct Site {
    command: Option<String>,
    targets: Vec<(String, u32, bool)>,
    unknown: bool,
    absent: bool,
    arguments: Vec<Option<String>>,
    frame: String,
}

type Sites = BTreeMap<(Arc<SourceOriginId>, u32), Site>;

fn origin_description(origin: &SourceOriginId) -> String {
    match origin.kind() {
        SourceOriginKind::Authored(_) => String::from("document"),
        SourceOriginKind::Derived {
            parent,
            arguments,
            source,
            kind,
        } => {
            let parent_origin = origin_description(&parent.source);
            let bytes = source.bytes();
            let offset = parent.offset;
            format!("{parent_origin}/{offset}:{arguments:?}:{kind:?}:{bytes:?}")
        }
        other => panic!("unexpected probe source instance {other:?}"),
    }
}

fn sites(realm: &CommandBindingRealm) -> Sites {
    let bindings = realm.source_bindings_ref();
    let root = bindings.source_origin().expect("source origin");
    let authored = bindings
        .dispatch_points
        .keys()
        .map(|&offset| (Arc::clone(root), offset));
    let materialised = bindings.origin_points.iter().flat_map(|(origin, points)| {
        points
            .iter()
            .filter(|point| point.dispatch)
            .map(|point| (Arc::clone(origin), point.offset))
    });
    authored
        .chain(materialised)
        .map(|(origin, offset)| {
            let invocation = bindings.invocation_at_origin(&origin, offset);
            let mut targets: Vec<_> = invocation
                .targets
                .iter()
                .map(|target| {
                    (
                        target.command.clone(),
                        target.implementation_generation,
                        target.registry_backed,
                    )
                })
                .collect();
            targets.sort_unstable();
            (
                (origin, offset),
                Site {
                    command: invocation.evaluated_command_word().map(str::to_owned),
                    targets,
                    unknown: invocation.unknown,
                    absent: invocation.may_be_absent,
                    arguments: invocation.evaluated_argument_values.to_vec(),
                    frame: format!("{:?}", invocation.variable_frame),
                },
            )
        })
        .collect()
}

fn discovered_values(
    unit: &CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    missing: BTreeSet<(u32, u32)>,
) -> Values {
    missing
        .into_iter()
        .filter_map(|(start, end)| {
            let read = original_source_read_at_unit(unit, Span::new(start, end), registry)?;
            let key = (read.symbol(), read.version()?);
            let LatticeValue::Const(ConstValue::String(value)) =
                read.function().semantic_values()?.contents(key)?
            else {
                return None;
            };
            Some(((start, end), value.as_str().to_owned()))
        })
        .collect()
}

struct Previous<'a> {
    source: &'a str,
    commands: &'a [SegmentedCommand],
}

fn run_round(
    source: &str,
    input: &ResolvedAnalysisInput,
    previous: Option<&Previous<'_>>,
    values: Values,
) -> (CommandBindingRealm, Values) {
    let config = input.lexer_config();
    let context = input.context_registry();
    let registry = context.commands();
    ROUND.with_borrow_mut(|round| {
        assert!(
            round
                .replace(OperandRound {
                    image: SourceImage::document(source),
                    values,
                    missing: BTreeSet::new(),
                })
                .is_none()
        );
    });
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    );
    let round = ROUND.with_borrow_mut(|round| round.take().expect("active probe round"));
    let values = discovered_values(&unit, registry, round.missing);
    let mut analyser = Analyser::new().with_resolved_input(input.clone());
    ROUND.with_borrow_mut(|slot| {
        *slot = Some(OperandRound {
            missing: BTreeSet::new(),
            ..round
        });
    });
    let analysis = previous.map_or_else(
        || {
            Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, DIALECT)
        },
        |previous| {
            let commands = segment_commands_incremental(
                previous.source,
                previous.commands,
                source,
                input.lexer_config(),
            );
            analyser.analyse_commands(source, &commands, DIALECT, false)
        },
    );
    ROUND.with_borrow_mut(|round| {
        round.take();
    });
    let realm = analysis
        .retained_command_realm()
        .expect("analysed source realm")
        .clone();
    (realm, values)
}

struct Measurement {
    realm: CommandBindingRealm,
    bytes: Vec<u8>,
    rebound: Vec<usize>,
}

fn measure(
    source: &str,
    input: &ResolvedAnalysisInput,
    previous: Option<&Previous<'_>>,
) -> Measurement {
    let mut values = Values::new();
    let mut earlier: Option<Sites> = None;
    let mut rebound = Vec::new();
    for _ in 0..LIMIT {
        let (realm, next) = run_round(source, input, previous, values.clone());
        let sites = sites(&realm);
        let changed = earlier.as_ref().map_or(0, |earlier| {
            sites
                .iter()
                .filter(|(key, site)| earlier.get(*key) != Some(*site))
                .count()
                + earlier
                    .keys()
                    .filter(|key| !sites.contains_key(*key))
                    .count()
        });
        rebound.push(changed);
        if next == values {
            assert!(realm.matches_resolved_analysis_input(input));
            assert!(realm.matches_original_source_image(
                &SourceImage::document(source),
                input.lexer_config()
            ));
            let rows: BTreeMap<_, _> = sites
                .iter()
                .map(|((origin, offset), site)| ((origin_description(origin), offset), site))
                .collect();
            assert_eq!(rows.len(), sites.len());
            let bytes = format!("{source:?}\n{DIALECT}\n{values:?}\n{rows:?}").into_bytes();
            return Measurement {
                realm,
                bytes,
                rebound,
            };
        }
        earlier = Some(sites);
        values = next;
    }
    panic!("did not converge in {LIMIT} rounds ({rebound:?})");
}

#[test]
fn binding_value_fixpoint_probe() {
    let input = Analyser::new()
        .prepare_analysis_input(DIALECT)
        .expect("Tcl profile");
    let fixtures = [
        (
            "e01-eval-var",
            include_str!(
                "../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e01-eval-var.tcl"
            ),
        ),
        (
            "e03-uplevel",
            include_str!(
                "../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e03-uplevel.tcl"
            ),
        ),
        (
            "e04-upvar",
            include_str!(
                "../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e04-upvar.tcl"
            ),
        ),
        (
            "e18-uplevel-proc-def",
            include_str!(
                "../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e18-uplevel-proc-def.tcl"
            ),
        ),
        (
            "e21-upvar-array-elem",
            include_str!(
                "../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e21-upvar-array-elem.tcl"
            ),
        ),
    ];
    let variants = [
        (
            "e01-computed-head",
            "set head [lreverse {set}]\n$head value 1\nputs $value\n",
        ),
        (
            "e03-computed-level",
            "set lvl [format %s #0]\nuplevel $lvl {set made 1}\nputs $made\n",
        ),
        (
            "e21-computed-array-name",
            "set a(1) x\nset name [lreverse {a(1)}]\nupvar #0 $name e\nset e y\nputs $a(1)\n",
        ),
    ];
    let mut saw_rebinding = false;
    for (name, source) in fixtures.into_iter().chain(variants) {
        // Replace the final command with the fixture's actual final command,
        // exercising a changed command stream with an unchanged prefix.
        let start = source
            .trim_end()
            .rfind('\n')
            .expect("fixture command prefix");
        let before = format!("{}\nset probe_predecessor 0\n", &source[..start]);
        let commands = segment_commands_with_offset_and_config(&before, 0, input.lexer_config());
        let previous = Previous {
            source: &before,
            commands: &commands,
        };
        let fresh = measure(source, &input, None);
        let incremental = measure(source, &input, Some(&previous));
        println!(
            "{name}: fresh {:?}, incremental {:?}",
            fresh.rebound, incremental.rebound
        );
        saw_rebinding |= fresh.rebound.iter().any(|&count| count > 0);
        assert_eq!(fresh.rebound, incremental.rebound, "{name}: round counts");
        assert!(
            fresh.realm == incremental.realm,
            "{name}: retained realms differ"
        );
        assert_eq!(fresh.bytes, incremental.bytes, "{name}: sealed site bytes");
    }
    assert!(
        saw_rebinding,
        "the fixtures must exercise the feedback edge"
    );
}
