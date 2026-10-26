// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-written hosted barriers remain separate from native publications.

use crate::signature_scan::vendor_name::VendorSourceNameInput;
use std::collections::BTreeSet;
use tcl_core_types::NameBytes;
use tcl_lexer::NativeWord;
use tcl_registry::model::ContextRegistry;
use tcl_registry::{CommandBindingTransition, StateTransitionDomain, TransitionSubject};
use tcl_syntax::naming::VendorSourceNamePurpose;

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalNominalCommandNameHazard {
    original: Vec<NativeWord>,
    operand: tcl_registry::source_name_ownership::NominalSourceCommandNameHazard,
    coordinate: NameBytes,
}

/// Source operands composed by a possible alias, without Native argv identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NominalAliasSourceOperandOrigin {
    /// Effective post-head operand of the authentic following source command.
    InvocationArgument(usize),
    /// A distinct original word captured by the source alias declaration.
    CapturedSourceWord(NativeWord),
}

pub(super) struct OriginalNominalAliasSourceAlternative {
    pub(super) target: VendorSourceNameInput,
    pub(super) prefix: Vec<NativeWord>,
}

/// Authored root-script transition evidence before one original hosted head.
/// These records can withdraw nominal source advice. They supply no command
/// existence, runtime table, Native name recipe, successful commit or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSourceCatalogueBarriers {
    head: VendorSourceNameInput,
    context: tcl_registry::model::ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    original: Vec<Vec<NativeWord>>,
    schemas: Vec<super::VendorRegistryInvocationShape>,
    unknown_transitions: bool,
    nominal_name_hazards: Vec<OriginalNominalCommandNameHazard>,
    regions: Vec<tcl_lexer::Span>,
}
impl VendorSourceCatalogueBarriers {
    /// Actual original head whose authored applicability was checked.
    #[must_use]
    pub const fn original_head(&self) -> &VendorSourceNameInput {
        &self.head
    }
    /// Complete original root commands preceding the selected head.
    /// Bodies are never treated as executed merely because they were written.
    #[must_use]
    pub fn original_commands(&self) -> &[Vec<NativeWord>] {
        &self.original
    }
    /// Independently selected source descriptors at the retained prior vectors.
    #[must_use]
    pub fn source_schemas(&self) -> &[super::VendorRegistryInvocationShape] {
        &self.schemas
    }
    /// Unknown mutators preserve alternative applicability, rather than making
    /// a definite source barrier or supplying a quiet-world proof.
    #[must_use]
    pub const fn has_unknown_transitions(&self) -> bool {
        self.unknown_transitions
    }
    /// A literal name in a nominal catalogue mutation layout overlaps this
    /// candidate's own source coordinates. This can withdraw object-candidate
    /// advice only; availability, dispatch and any actual table change remain
    /// unproved. Unknown transitions retain their independent residual.
    #[must_use]
    pub fn has_nominal_command_name_hazard(&self) -> bool {
        !self.nominal_name_hazards.is_empty()
    }

    /// Project every named replacement as a separate conditional source schema.
    /// Only finite literal same-interpreter aliases have this purpose. Original
    /// invocation operands and captured words remain distinct; no alias entry,
    /// Native argv, handler, Normal result or discharged uncertainty follows.
    pub fn nominal_alias_source_projections<T>(
        &self,
        context: &ContextRegistry,
        following: &super::source_structure::OriginalRegistryWords,
        mut project: impl FnMut(
            &tcl_registry::ResolvedInvocation<'_, '_>,
            &[NominalAliasSourceOperandOrigin],
        ) -> Option<T>,
    ) -> Option<Vec<T>> {
        if !self.matches_context(context)
            || !following.matches_source(self.head.source_image(), self.head.lexer_config())
            || !matches!(following.source(),
                super::source_structure::OriginalRegistrySource::Vendor(metadata)
                    if metadata.authored_barriers() == self)
        {
            return None;
        }
        let mut projections = Vec::new();
        for hazard in &self.nominal_name_hazards {
            let replacement = hazard.operand.alias_replacement()?;
            let target = VendorSourceNameInput::from_original_word(
                hazard
                    .original
                    .get(replacement.target_argument().checked_add(1)?)?,
                self.head.policy(),
            );
            let coordinate = target.root_command_coordinate()?;
            if coordinate == hazard.coordinate {
                return None;
            }
            // An existing source replacement of the target could compose a
            // different alias or a cycle. Never resolve that from label text.
            for region in &self.regions {
                let target_barriers =
                    capture_region(context, &self.head, &[vec![coordinate.clone()]], *region)?;
                if target_barriers.has_nominal_command_name_hazard() {
                    return None;
                }
            }
            let prefix = replacement
                .prefix_arguments()
                .iter()
                .map(|argument| hazard.original.get(argument.checked_add(1)?).cloned())
                .collect::<Option<Vec<_>>>()?;
            let origins = prefix
                .iter()
                .cloned()
                .map(NominalAliasSourceOperandOrigin::CapturedSourceWord)
                .chain(
                    (0..following.arguments().len())
                        .map(NominalAliasSourceOperandOrigin::InvocationArgument),
                )
                .collect::<Vec<_>>();
            let alternative = OriginalNominalAliasSourceAlternative { target, prefix };
            projections.push(super::vendor_advice::with_vendor_alias_source_schema(
                context,
                &alternative,
                following,
                |schema| project(schema, &origins),
            )?);
        }
        Some(projections)
    }
    /// Same complete source image, channel and semantic lexer configuration.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.head.matches_source(image, config)
            && self
                .original
                .iter()
                .flatten()
                .all(|word| word.image() == image && word.config() == config)
    }
    /// Same retained Registry store and complete availability context.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        self.registry == context.commands().snapshot().semantic_key()
            && &self.context == context.context()
    }
}

/// Readonly authored-source ordering; neither runtime transfer nor a lookup.
pub(super) fn capture(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    selected_prefixes: &[Vec<NameBytes>],
) -> Option<VendorSourceCatalogueBarriers> {
    let region = tcl_lexer::Span::new(0, u32::try_from(head.source_image().bytes().len()).ok()?);
    capture_region(context, head, selected_prefixes, region)
}

/// The same authored barrier classifier over sealed original script ancestry.
/// Parent source bodies supply regions only; no entered frame or table follows.
pub(super) fn capture_in_script_body(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    selected_prefixes: &[Vec<NameBytes>],
    body: &super::OriginalSourceScriptBodyOrigin,
) -> Option<VendorSourceCatalogueBarriers> {
    let mut receipt = capture(context, head, selected_prefixes)?;
    for region in body.source_regions() {
        let local = capture_region(context, head, selected_prefixes, *region)?;
        receipt.original.extend(local.original);
        receipt.schemas.extend(local.schemas);
        receipt
            .nominal_name_hazards
            .extend(local.nominal_name_hazards);
        receipt.unknown_transitions |= local.unknown_transitions;
        receipt.regions.extend(local.regions);
    }
    Some(receipt)
}

fn capture_region(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    selected_prefixes: &[Vec<NameBytes>],
    region: tcl_lexer::Span,
) -> Option<VendorSourceCatalogueBarriers> {
    let image = head.source_image();
    let config = head.lexer_config();
    let plan = tcl_lexer::native_script_words_in(image.clone(), region, config).ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let mut receipt = VendorSourceCatalogueBarriers {
        head: head.clone(),
        context: context.context().clone(),
        registry: context.commands().snapshot().semantic_key(),
        original: Vec::new(),
        schemas: Vec::new(),
        unknown_transitions: false,
        nominal_name_hazards: Vec::new(),
        regions: vec![region],
    };
    let mut barriers = BTreeSet::new();
    for command in plan.commands {
        let Some(first) = command.words.first() else {
            continue;
        };
        let Some(last) = command.words.last() else {
            continue;
        };
        // A body containing this head is not a preceding source operation.
        if last.span().end() > head.span().start() {
            break;
        }
        let input = VendorSourceNameInput::from_original_word(first, head.policy());
        receipt.original.push(command.words.clone());
        let root_paths = input
            .literal_units(VendorSourceNamePurpose::CommandHead)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(|name| {
                tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
                    "::",
                    &[] as &[&str],
                    name,
                )
            });
        if root_paths.as_ref().is_some_and(|paths| {
            paths
                .iter()
                .any(|name| barriers.contains(&NameBytes::from(name.as_bytes())))
        }) {
            // A custom preceding handler may itself mutate arbitrary names.
            receipt.unknown_transitions = true;
            barriers.clear();
            continue;
        }
        receipt.nominal_name_hazards.extend(nominal_name_hazards(
            context,
            &input,
            &command.words,
            selected_prefixes,
        ));
        let Some(shape) = super::vendor_registry_invocation_shape(context, &input, &command.words)
        else {
            receipt.unknown_transitions = true;
            barriers.clear();
            continue;
        };
        let transitions = shape.source_transitions();
        let deferred = shape
            .possible_traits()
            .contains(tcl_registry::Traits::DEFERS_BODY);
        let immediate_body = shape
            .roles()
            .iter()
            .any(|(_, role)| matches!(role, tcl_registry::ArgRole::Body))
            && !deferred;
        if immediate_body
            || transitions.is_none_or(|transitions| {
                transitions.widens(StateTransitionDomain::CommandBindings)
                    || transitions.widens(StateTransitionDomain::CommandResolution)
            })
        {
            receipt.unknown_transitions = true;
            barriers.clear();
        } else if !receipt.unknown_transitions
            && let Some(transitions) = transitions
        {
            for transition in transitions.command_bindings() {
                if apply_source_transition(&mut barriers, &shape, transition).is_none() {
                    receipt.unknown_transitions = true;
                    barriers.clear();
                    break;
                }
            }
        }
        receipt.schemas.push(shape);
    }
    if selected_prefixes
        .iter()
        .any(|path| path.iter().any(|coordinate| barriers.contains(coordinate)))
    {
        return None;
    }
    Some(receipt)
}

fn nominal_name_hazards(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    original: &[NativeWord],
    selected_prefixes: &[Vec<NameBytes>],
) -> Vec<OriginalNominalCommandNameHazard> {
    let Some(coordinate) = head.root_command_coordinate() else {
        return Vec::new();
    };
    let Ok(command) = std::str::from_utf8(coordinate.as_bytes()) else {
        return Vec::new();
    };
    let arguments = original
        .iter()
        .skip(1)
        .map(|word| {
            if word.group().expand {
                return tcl_registry::InvocationWord::Expanded;
            }
            tcl_syntax::naming::vendor_source_literal_units(
                head.policy(),
                word,
                VendorSourceNamePurpose::SourceName,
            )
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map_or(
                tcl_registry::InvocationWord::Dynamic,
                tcl_registry::InvocationWord::Literal,
            )
        })
        .collect::<Vec<_>>();
    context
        .commands()
        .nominal_source_command_name_hazards(tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(command),
            &arguments,
        ))
        .into_iter()
        .filter_map(|operand| {
            let word = original.get(operand.argument().checked_add(1)?)?;
            let input = VendorSourceNameInput::from_original_word(word, head.policy());
            if input.literal_units(VendorSourceNamePurpose::SourceName)? != operand.name() {
                return None;
            }
            let coordinate = input.root_command_coordinate()?;
            selected_prefixes
                .iter()
                .any(|path| path.contains(&coordinate))
                .then_some(())?;
            Some(OriginalNominalCommandNameHazard {
                original: original.to_vec(),
                operand,
                coordinate,
            })
        })
        .collect()
}

fn source_subject<'a>(
    shape: &'a super::VendorRegistryInvocationShape,
    subject: &TransitionSubject,
) -> Option<&'a [u8]> {
    let ordinal = subject.argument_index()?.checked_add(1)?;
    let value = tcl_syntax::naming::vendor_source_literal_units(
        shape.original_head().policy(),
        shape.original_words().get(ordinal)?,
        VendorSourceNamePurpose::SourceName,
    )?;
    let selected = subject
        .literal()
        .map(str::as_bytes)
        .or_else(|| subject.native_bytes())?;
    (selected == value).then_some(value)
}
fn source_coordinate(
    shape: &super::VendorRegistryInvocationShape,
    subject: &TransitionSubject,
) -> Option<NameBytes> {
    Some(NameBytes::from(tcl_syntax::naming::qualify_bytes(
        b"::",
        source_subject(shape, subject)?,
    )))
}
fn apply_source_transition(
    barriers: &mut BTreeSet<NameBytes>,
    shape: &super::VendorRegistryInvocationShape,
    transition: &CommandBindingTransition,
) -> Option<()> {
    match transition {
        CommandBindingTransition::Define { name, .. } => {
            barriers.insert(source_coordinate(shape, name)?);
        }
        CommandBindingTransition::Move { from, to } => {
            barriers.insert(source_coordinate(shape, from)?);
            if !source_subject(shape, to)?.is_empty() {
                barriers.insert(source_coordinate(shape, to)?);
            }
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            if interpreter.as_ref().is_some_and(|interpreter| {
                source_subject(shape, interpreter).is_none_or(|bytes| !bytes.is_empty())
            }) {
                return None;
            }
            barriers.insert(source_coordinate(shape, name)?);
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            ..
        } => {
            if !source_subject(shape, source_interpreter)?.is_empty() {
                return None;
            }
            barriers.insert(source_coordinate(shape, alias)?);
        }
        CommandBindingTransition::Unknown { .. } => return None,
    }
    Some(())
}

#[cfg(test)]
mod tests {
    fn candidate(
        source: &str,
        marker: &str,
    ) -> Option<super::super::OriginalConditionalVendorRegistryMetadata> {
        candidate_in(source, marker, "f5-irules")
    }
    fn candidate_in(
        source: &str,
        marker: &str,
        environment: &str,
    ) -> Option<super::super::OriginalConditionalVendorRegistryMetadata> {
        let analysis = crate::analyser::Analyser::new().analyse(source, environment);
        super::super::source_structure::selected_vendor_registry_words_at(
            source,
            &analysis,
            u32::try_from(source.rfind(marker).unwrap()).unwrap(),
        )
        .map(|(metadata, _)| metadata)
    }
    #[test]
    fn nominal_named_hazards_withdraw_candidates_without_execution_tombstones() {
        for (prelude, hazard) in [
            ("rename pool moved", true),
            ("::rename ::pool moved", true),
            ("rename ::other::pool moved", false),
            ("rename $which moved", false),
            ("interp alias {} helper {} pool", false),
            ("proc rename args {}; rename pool moved", false),
        ] {
            let source = format!("{prelude}\nwhen HTTP_REQUEST {{pool selected}}");
            let metadata = candidate(&source, "pool selected").unwrap();
            let barriers = metadata.authored_barriers();
            assert!(barriers.has_unknown_transitions(), "{prelude}");
            assert_eq!(
                barriers.has_nominal_command_name_hazard(),
                hazard,
                "{prelude}"
            );
            assert!(barriers.matches_source(
                &tcl_lexer::SourceImage::document(&source),
                barriers.original_head().lexer_config()
            ));
        }
    }

    #[test]
    fn original_hosted_source_barriers_keep_declarations_moves_and_unknowns_separate() {
        for (source, marker) in [
            (
                "proc call {args} {}\nwhen HTTP_REQUEST {call Lib::one}",
                "call Lib",
            ),
            (
                "proc pool args {}\nwhen HTTP_REQUEST {pool selected}",
                "pool selected",
            ),
            ("proc when args {}\nwhen HTTP_REQUEST {}", "when HTTP"),
            (
                "proc HTTP::respond args {return custom}\nwhen HTTP_REQUEST {HTTP::respond 200}",
                "HTTP::respond 200",
            ),
        ] {
            assert!(candidate(source, marker).is_none(), "{source}");
        }
        // The iApp context independently admits this transition descriptor.
        assert!(
            candidate_in(
                "rename source moved; source selected.tcl",
                "source selected",
                "f5-iapps",
            )
            .is_none()
        );
        assert!(
            candidate_in(
                "rename source {}; source selected.tcl",
                "source selected",
                "f5-iapps",
            )
            .is_none()
        );
        // TMM excludes rename: its spelling supplies no selected transition.
        let unavailable = candidate(
            "rename pool moved\nwhen HTTP_REQUEST {pool selected}",
            "pool selected",
        )
        .unwrap();
        assert!(unavailable.authored_barriers().has_unknown_transitions());
        let known = candidate("when HTTP_REQUEST {pool selected}", "pool selected").unwrap();
        assert!(!known.authored_barriers().has_unknown_transitions());
        let unknown = candidate(
            "rename $which moved\nwhen HTTP_REQUEST {pool selected}",
            "pool selected",
        )
        .unwrap();
        assert!(unknown.authored_barriers().has_unknown_transitions());
        // A declaration written inside a deferred procedure has not run as a root statement.
        assert!(
            candidate(
                "proc define {} {proc pool args {}}\nwhen HTTP_REQUEST {pool selected}",
                "pool selected"
            )
            .is_some()
        );
        // A possible arbitrary mutation withdraws definite source-written shadowing.
        let uncertain = candidate(
            "proc pool args {}\nunknown_mutator\nwhen HTTP_REQUEST {pool selected}",
            "pool selected",
        )
        .unwrap();
        assert!(uncertain.authored_barriers().has_unknown_transitions());
        let image = tcl_lexer::SourceImage::document("when HTTP_REQUEST {pool selected}");
        let config = known.authored_barriers().original_head().lexer_config();
        assert!(known.authored_barriers().matches_source(&image, config));
        assert!(!known.authored_barriers().matches_source(
            &tcl_lexer::SourceImage::document("when CLIENT_DATA {}"),
            config
        ));
        assert!(known.authored_barriers().matches_context(
            tcl_registry::model::ingress::static_context_for("f5-irules")
        ));
        assert!(
            !known
                .authored_barriers()
                .matches_context(tcl_registry::model::ingress::static_context_for("f5-iapps"))
        );
    }
}
