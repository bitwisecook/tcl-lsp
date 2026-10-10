// SPDX-License-Identifier: AGPL-3.0-or-later
//! Flat executable substitution components from the shared word scanner.

use std::borrow::Cow;

use crate::word_parts::{
    SpannedPart, SubstFlags, TemplateVariableSyntax, WordPart, shallow_spanned_parts,
};
use crate::{LexerConfig, SourceImage, Span};

/// An ordered component list owned by one [`ExecutablePartArena`].
/// IDs are meaningful only in the arena that returned them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartListId(usize);

/// Literal bytes of an executable component.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutableText {
    /// The component's original source span already contains its value bytes.
    Original,
    /// The shared backslash decoder produced these bytes from the source span.
    Decoded(Vec<u8>),
}

/// An executable component with index children retained as flat list IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutablePart {
    /// Literal text, with an explicit original/decoded storage distinction.
    Text(ExecutableText),
    /// Original variable name and its separately substituted index, if any.
    Variable {
        /// Exact name bytes in the arena's original image, without wrappers.
        name: Span,
        /// Ordered index components; `None` denotes a scalar reference.
        index: Option<PartListId>,
    },
    /// Command substitution whose body is an original byte source span.
    Command {
        /// Inner script bytes, excluding the surrounding brackets.
        body: Span,
    },
    /// Jim expression substitution whose original parentheses are retained.
    Expression {
        /// Original expression bytes, including its parentheses.
        expression: Span,
    },
    /// The shared scanner's native syntax error, after preceding components.
    ParseError(&'static str),
}

/// One executable component and its original image byte extent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpannedExecutablePart {
    /// The retained component.
    pub part: ExecutablePart,
    /// Raw source extent, including substitution delimiters and undecoded text.
    pub span: Span,
    source_span: Span,
    error_term: Option<u32>,
}

impl SpannedExecutablePart {
    /// Actual failed delimiter retained by the shared scanner, independently of the component span.
    #[must_use]
    pub const fn parse_error_term(&self) -> Option<u32> {
        self.error_term
    }
}

/// A source geometry or ownership failure in executable decomposition.
/// Native syntax errors remain explicit [`ExecutablePart::ParseError`] parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutablePartsUnavailable {
    /// An extent is outside the exact original image or cannot be represented.
    SourceBounds,
    /// A scanner component does not borrow the captured original source.
    SourceGeometry,
}

/// Full-depth executable components with one immutable original source owner.
///
/// Construction uses the shared substitution scanner without recursive index
/// decomposition, then queues each raw index for that same scanner. All index
/// edges are list IDs; construction, traversal and destruction require no
/// recursive component tree. Array-index syntax, bracket boundaries and literal
/// escapes are supplied by their existing lexical owners.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutablePartArena {
    image: SourceImage,
    config: LexerConfig,
    variables: TemplateVariableSyntax,
    lists: Vec<Vec<SpannedExecutablePart>>,
}

/// Original source regions selected by an existing word/token owner. This
/// chooses literal data versus substitution; it grants no compiler admission.
pub enum ExecutableInput {
    /// Exact literal source data, with channel-selected value translation later.
    Literal(Span),
    /// An original region passed to the common substitution scanners.
    Substitute {
        /// Original whole-image byte extent.
        span: Span,
        /// Independently selected substitution grammar switches.
        flags: SubstFlags,
    },
}

impl ExecutablePartArena {
    /// Decompose an original byte span as an executable substitution template.
    /// Component and name spans address `image`, irrespective of its channel.
    ///
    /// # Errors
    /// Returns unavailable source geometry rather than guessing text or names.
    pub fn decompose(
        image: SourceImage,
        content: Span,
        flags: SubstFlags,
        config: LexerConfig,
    ) -> Result<Self, ExecutablePartsUnavailable> {
        Self::from_inputs(
            image,
            &[ExecutableInput::Substitute {
                span: content,
                flags,
            }],
            config,
        )
    }

    /// Decompose a reached substitution template under its own variable grammar.
    /// Written words use `decompose`; Jim's `subst` braced-variable acceptance
    /// remains independently selected rather than borrowed from word parsing.
    ///
    /// # Errors
    /// Returns unavailable original source geometry, never replacement text.
    pub fn decompose_template(
        image: SourceImage,
        content: Span,
        flags: SubstFlags,
        config: LexerConfig,
        variables: TemplateVariableSyntax,
    ) -> Result<Self, ExecutablePartsUnavailable> {
        Self::from_inputs_with_variables(
            image,
            &[ExecutableInput::Substitute {
                span: content,
                flags,
            }],
            config,
            variables,
        )
    }

    /// The sole exact original source, including its independently retained channel.
    #[must_use]
    pub fn image(&self) -> &SourceImage {
        &self.image
    }

    /// Exact lexical configuration used by the shared component scanner.
    #[must_use]
    pub const fn config(&self) -> LexerConfig {
        self.config
    }

    /// The independently selected runtime-template variable grammar.
    #[must_use]
    pub const fn variable_syntax(&self) -> TemplateVariableSyntax {
        self.variables
    }

    /// The component list for the original template or grouped word.
    #[must_use]
    pub const fn root(&self) -> PartListId {
        PartListId(0)
    }

    /// Components in evaluation order. `id` must belong to this arena.
    #[must_use]
    pub fn list(&self, id: PartListId) -> &[SpannedExecutablePart] {
        &self.lists[id.0]
    }

    /// Every retained component, including index children, without recursion.
    pub fn all_parts(&self) -> impl Iterator<Item = &SpannedExecutablePart> {
        self.lists.iter().flatten()
    }

    /// Smallest original variable component containing an image offset.
    /// Index-child references take precedence over their enclosing array
    /// reference. Conflicting equal extents decline; no text is reparsed and
    /// this lexical component supplies no variable cell or read admission.
    #[must_use]
    pub fn variable_part_at(&self, offset: u32) -> Option<&SpannedExecutablePart> {
        let mut selected: Option<&SpannedExecutablePart> = None;
        let mut conflicting = false;
        for part in self.all_parts().filter(|part| {
            matches!(part.part, ExecutablePart::Variable { .. })
                && part.span.start() <= offset
                && offset < part.span.end()
        }) {
            match selected {
                Some(previous) if previous.span.len() < part.span.len() => {}
                Some(previous) if previous.span.len() == part.span.len() => {
                    conflicting |= previous != part;
                }
                _ => {
                    selected = Some(part);
                    conflicting = false;
                }
            }
        }
        (!conflicting).then_some(selected).flatten()
    }

    /// Exact original bytes for an image-coordinate extent.
    #[must_use]
    pub fn bytes(&self, span: Span) -> Option<&[u8]> {
        self.image.bytes().get(span.as_range())
    }

    /// The shared token extent, including its native inner-closer convention.
    /// The component must belong to this arena. Whole raw bytes remain in `span`.
    #[must_use]
    pub fn source_span(&self, component: &SpannedExecutablePart) -> Option<Span> {
        self.bytes(component.span)?;
        self.bytes(component.source_span)?;
        Some(component.source_span)
    }

    /// Value bytes of a text component; other component kinds return `None`.
    /// An original text component uses its original span, while a decoded text
    /// component uses only the shared decoder's retained output.
    #[must_use]
    pub fn text<'a>(&'a self, component: &'a SpannedExecutablePart) -> Option<&'a [u8]> {
        match &component.part {
            ExecutablePart::Text(ExecutableText::Original) => self.bytes(component.span),
            ExecutablePart::Text(ExecutableText::Decoded(bytes)) => Some(bytes),
            _ => None,
        }
    }

    pub(crate) fn empty(image: SourceImage, config: LexerConfig) -> Self {
        Self {
            image,
            config,
            variables: TemplateVariableSyntax::WrittenWord,
            lists: vec![vec![]],
        }
    }

    /// Retain ordered regions chosen by a lexical owner, without assembling a
    /// rewritten source string. Every region addresses the same original image.
    ///
    /// # Errors
    /// Returns unavailable original geometry. Authentic syntax errors remain
    /// explicit components, as with [`Self::decompose`].
    pub fn from_inputs(
        image: SourceImage,
        inputs: &[ExecutableInput],
        config: LexerConfig,
    ) -> Result<Self, ExecutablePartsUnavailable> {
        Self::from_inputs_with_variables(image, inputs, config, TemplateVariableSyntax::WrittenWord)
    }

    fn from_inputs_with_variables(
        image: SourceImage,
        inputs: &[ExecutableInput],
        config: LexerConfig,
        variables: TemplateVariableSyntax,
    ) -> Result<Self, ExecutablePartsUnavailable> {
        let mut arena = Self::empty(image, config);
        arena.variables = variables;
        let mut pending = Vec::new();
        let mut root = Vec::new();
        for input in inputs {
            match *input {
                ExecutableInput::Literal(span) => {
                    arena
                        .bytes(span)
                        .ok_or(ExecutablePartsUnavailable::SourceBounds)?;
                    root.push(SpannedExecutablePart {
                        part: ExecutablePart::Text(ExecutableText::Original),
                        span,
                        source_span: span,
                        error_term: None,
                    });
                }
                ExecutableInput::Substitute { span, flags } => {
                    root.extend(scan_list(
                        &arena.image,
                        span,
                        flags,
                        config,
                        variables,
                        &mut arena.lists,
                        &mut pending,
                    )?);
                }
            }
        }
        arena.lists[0] = root;
        while let Some((id, span, flags)) = pending.pop() {
            let parts = scan_list(
                &arena.image,
                span,
                flags,
                config,
                if variables == TemplateVariableSyntax::CTcl {
                    // A C template's variable index uses full word-token
                    // evaluation. Its text/BS tokens are joined by the native
                    // index compiler, independently of root template tokens.
                    TemplateVariableSyntax::WrittenWord
                } else {
                    variables
                },
                &mut arena.lists,
                &mut pending,
            )?;
            arena.lists[id.0] = parts;
        }
        Ok(arena)
    }
}

fn scan_list(
    image: &SourceImage,
    span: Span,
    flags: SubstFlags,
    config: LexerConfig,
    variables: TemplateVariableSyntax,
    lists: &mut Vec<Vec<SpannedExecutablePart>>,
    pending: &mut Vec<(PartListId, Span, SubstFlags)>,
) -> Result<Vec<SpannedExecutablePart>, ExecutablePartsUnavailable> {
    let bytes = image
        .bytes()
        .get(span.as_range())
        .ok_or(ExecutablePartsUnavailable::SourceBounds)?;
    shallow_spanned_parts(bytes, flags, config, variables)
        .into_iter()
        .map(|component| {
            let source_span = component
                .template_source_span(bytes, span.start(), variables)
                .ok_or(ExecutablePartsUnavailable::SourceGeometry)?;
            retain_component(image, span.start(), component, source_span, lists, pending)
        })
        .collect()
}

fn retain_component(
    image: &SourceImage,
    base: u32,
    component: SpannedPart<'_>,
    source_span: Span,
    lists: &mut Vec<Vec<SpannedExecutablePart>>,
    pending: &mut Vec<(PartListId, Span, SubstFlags)>,
) -> Result<SpannedExecutablePart, ExecutablePartsUnavailable> {
    let start = base
        .checked_add(
            u32::try_from(component.start).map_err(|_| ExecutablePartsUnavailable::SourceBounds)?,
        )
        .ok_or(ExecutablePartsUnavailable::SourceBounds)?;
    let end = base
        .checked_add(
            u32::try_from(component.end).map_err(|_| ExecutablePartsUnavailable::SourceBounds)?,
        )
        .ok_or(ExecutablePartsUnavailable::SourceBounds)?;
    let span = Span::new(start, end);
    image
        .bytes()
        .get(span.as_range())
        .ok_or(ExecutablePartsUnavailable::SourceBounds)?;
    let error_term = match component.error_term {
        Some(term) => Some(
            base.checked_add(
                u32::try_from(term).map_err(|_| ExecutablePartsUnavailable::SourceBounds)?,
            )
            .filter(|term| image.bytes().get(*term as usize).is_some())
            .ok_or(ExecutablePartsUnavailable::SourceBounds)?,
        ),
        None => None,
    };
    let part = match component.part {
        WordPart::Text(Cow::Borrowed(bytes)) => {
            if borrowed_span(image, bytes)? != span {
                return Err(ExecutablePartsUnavailable::SourceGeometry);
            }
            ExecutablePart::Text(ExecutableText::Original)
        }
        WordPart::Text(Cow::Owned(bytes)) => ExecutablePart::Text(ExecutableText::Decoded(bytes)),
        WordPart::Variable(reference) => {
            let name = borrowed_span(image, reference.name)?;
            let index = match reference.index {
                None => None,
                Some(mut raw) => {
                    if raw.len() != 1 {
                        return Err(ExecutablePartsUnavailable::SourceGeometry);
                    }
                    let WordPart::Text(Cow::Borrowed(bytes)) = raw.remove(0) else {
                        return Err(ExecutablePartsUnavailable::SourceGeometry);
                    };
                    let index_span = borrowed_span(image, bytes)?;
                    let id = PartListId(lists.len());
                    lists.push(vec![]);
                    // Tcl_ParseVarName uses TCL_SUBST_ALL for its index;
                    // JimExpandDictSugar selects Jim_SubstObj(...JIM_NONE).
                    pending.push((id, index_span, SubstFlags::default()));
                    Some(id)
                }
            };
            ExecutablePart::Variable { name, index }
        }
        WordPart::Command(bytes) => ExecutablePart::Command {
            body: borrowed_span(image, bytes)?,
        },
        WordPart::Expression(bytes) => ExecutablePart::Expression {
            expression: borrowed_span(image, bytes)?,
        },
        WordPart::ParseError(message) => ExecutablePart::ParseError(message),
    };
    Ok(SpannedExecutablePart {
        part,
        span,
        source_span,
        error_term,
    })
}

fn borrowed_span(image: &SourceImage, bytes: &[u8]) -> Result<Span, ExecutablePartsUnavailable> {
    let source = image.bytes();
    let start = (bytes.as_ptr() as usize)
        .checked_sub(source.as_ptr() as usize)
        .ok_or(ExecutablePartsUnavailable::SourceGeometry)?;
    let end = start
        .checked_add(bytes.len())
        .filter(|end| *end <= source.len())
        .ok_or(ExecutablePartsUnavailable::SourceGeometry)?;
    Ok(Span::new(
        u32::try_from(start).map_err(|_| ExecutablePartsUnavailable::SourceBounds)?,
        u32::try_from(end).map_err(|_| ExecutablePartsUnavailable::SourceBounds)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_variable_component_cursor_keeps_index_children_and_literal_dollars() {
        // Implementation contract: naming.core.selected-variable-cursor-syntax
        // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
        let source = "$arr($idx) ${cash$name}";
        let arena = ExecutablePartArena::decompose(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            SubstFlags::default(),
            LexerConfig::default(),
        )
        .unwrap();
        let outer = arena.variable_part_at(2).unwrap();
        assert_eq!(arena.bytes(outer.span), Some(b"$arr($idx)".as_slice()));
        let child = arena.variable_part_at(7).unwrap();
        assert_eq!(arena.bytes(child.span), Some(b"$idx".as_slice()));
        let literal = arena
            .variable_part_at(u32::try_from(source.find("$name").unwrap()).unwrap())
            .unwrap();
        assert_eq!(arena.bytes(literal.span), Some(b"${cash$name}".as_slice()));
        assert!(arena.variable_part_at(10).is_none());
        assert!(
            arena
                .variable_part_at(u32::try_from(source.len()).unwrap())
                .is_none()
        );
    }

    #[test]
    fn full_depth_index_edges_keep_original_bytes_and_drop_without_recursion() {
        std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(|| {
                let mut source = Vec::new();
                for _ in 0..2000 {
                    source.extend_from_slice(b"$a(");
                }
                source.extend_from_slice(b"x\xff[child]");
                source.extend(std::iter::repeat_n(b')', 2000));
                let image = SourceImage::native(source);
                let span = Span::new(
                    0,
                    u32::try_from(image.len()).expect("fixture source extent"),
                );
                let arena = ExecutablePartArena::decompose(
                    image,
                    span,
                    SubstFlags::default(),
                    LexerConfig::default(),
                )
                .unwrap();
                let mut list = arena.root();
                for _ in 0..2000 {
                    let [component] = arena.list(list) else {
                        panic!("one original index reference");
                    };
                    let ExecutablePart::Variable {
                        name,
                        index: Some(child),
                    } = component.part
                    else {
                        panic!("retained index edge");
                    };
                    assert_eq!(arena.bytes(name), Some(b"a".as_slice()));
                    list = child;
                }
                let [literal, script] = arena.list(list) else {
                    panic!("literal then original child");
                };
                assert_eq!(arena.text(literal), Some(b"x\xff".as_slice()));
                let ExecutablePart::Command { body } = script.part else {
                    panic!("original bracket child");
                };
                assert_eq!(arena.bytes(body), Some(b"child".as_slice()));
                assert_eq!(arena.all_parts().count(), 2002);
                drop(arena);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn flat_parts_keep_decoded_text_separate_from_raw_component_extents() {
        let image = SourceImage::native(b"pre\\t${\xff}-$a(k\\n)[child]".as_slice());
        let span = Span::new(
            0,
            u32::try_from(image.len()).expect("fixture source extent"),
        );
        let arena = ExecutablePartArena::decompose(
            image,
            span,
            SubstFlags::default(),
            LexerConfig::default(),
        )
        .unwrap();
        let [text, scalar, separator, array, command] = arena.list(arena.root()) else {
            panic!("original five components");
        };
        assert_eq!(arena.text(text), Some(b"pre\t".as_slice()));
        assert_eq!(arena.bytes(text.span), Some(b"pre\\t".as_slice()));
        assert!(matches!(
            text.part,
            ExecutablePart::Text(ExecutableText::Decoded(_))
        ));
        let ExecutablePart::Variable { name, index: None } = scalar.part else {
            panic!("opaque scalar name");
        };
        assert_eq!(arena.bytes(name), Some(b"\xff".as_slice()));
        assert_eq!(arena.bytes(scalar.span), Some(b"${\xff}".as_slice()));
        assert_eq!(
            arena.bytes(arena.source_span(scalar).unwrap()),
            Some(b"${\xff".as_slice())
        );
        assert_eq!(arena.text(separator), Some(b"-".as_slice()));
        let ExecutablePart::Variable {
            index: Some(index), ..
        } = array.part
        else {
            panic!("retained array index");
        };
        assert_eq!(arena.text(&arena.list(index)[0]), Some(b"k\n".as_slice()));
        let ExecutablePart::Command { body } = command.part else {
            panic!("original command");
        };
        assert_eq!(arena.bytes(body), Some(b"child".as_slice()));
    }

    #[test]
    fn template_variable_grammar_retains_jim_missing_braced_closer_independently() {
        let image = SourceImage::native(b"${name".as_slice());
        let span = Span::new(
            0,
            u32::try_from(image.len()).expect("fixture source extent"),
        );
        let config = LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("jim")));
        let template = ExecutablePartArena::decompose_template(
            image.clone(),
            span,
            SubstFlags::default(),
            config,
            TemplateVariableSyntax::Jim084,
        )
        .unwrap();
        let [component] = template.list(template.root()) else {
            panic!("one actual template variable");
        };
        let ExecutablePart::Variable { name, index: None } = component.part else {
            panic!("accepted template variable");
        };
        assert_eq!(template.bytes(name), Some(b"name".as_slice()));
        assert_eq!(template.source_span(component), Some(span));
        let written =
            ExecutablePartArena::decompose(image, span, SubstFlags::default(), config).unwrap();
        assert!(matches!(
            written.list(written.root())[0].part,
            ExecutablePart::ParseError(_)
        ));
    }

    #[test]
    fn template_masks_leave_array_index_substitutions_enabled() {
        // Source proof: naming.substitution.template-token-and-index-source
        // docs/design/analysis/name-resolution-proofs/substitution-template-token-and-index-source.md
        // Native proof: naming.substitution.counted-template-completions
        // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
        // C v6 template2/flags6 evaluates [set k k] in the index. Jim's
        // independent JimExpandDictSugar source selects JIM_NONE there.
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            let variables = if dialect == "jim" {
                TemplateVariableSyntax::Jim084
            } else {
                TemplateVariableSyntax::CTcl
            };
            let source = br"$a($k\x21[side])-L";
            let arena = ExecutablePartArena::decompose_template(
                SourceImage::native(source.as_slice()),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                SubstFlags {
                    cmds: false,
                    backslashes: false,
                    ..SubstFlags::default()
                },
                config,
                variables,
            )
            .unwrap();
            let [reference, trailing] = arena.list(arena.root()) else {
                panic!("one original variable and trailing literal");
            };
            assert_eq!(arena.text(trailing), Some(b"-L".as_slice()));
            let ExecutablePart::Variable {
                index: Some(index), ..
            } = reference.part
            else {
                panic!("retained array index");
            };
            let [variable, decoded, command] = arena.list(index) else {
                panic!("all index substitutions enabled for {dialect}");
            };
            assert!(matches!(variable.part, ExecutablePart::Variable { .. }));
            assert_eq!(arena.text(decoded), Some(b"!".as_slice()));
            let ExecutablePart::Command { body } = command.part else {
                panic!("independently enabled index command");
            };
            assert_eq!(arena.bytes(body), Some(b"side".as_slice()));
        }
    }

    #[test]
    fn syntax_errors_and_unavailable_geometry_remain_distinct() {
        let image = SourceImage::native(b"[side]$a(".as_slice());
        let span = Span::new(
            0,
            u32::try_from(image.len()).expect("fixture source extent"),
        );
        let arena = ExecutablePartArena::decompose(
            image.clone(),
            span,
            SubstFlags::default(),
            LexerConfig::default(),
        )
        .unwrap();
        let [earlier, error] = arena.list(arena.root()) else {
            panic!("earlier side effect then native syntax error");
        };
        assert!(matches!(earlier.part, ExecutablePart::Command { .. }));
        assert!(matches!(error.part, ExecutablePart::ParseError(_)));
        assert_eq!(
            ExecutablePartArena::decompose(
                image,
                Span::new(0, 999),
                SubstFlags::default(),
                LexerConfig::default()
            ),
            Err(ExecutablePartsUnavailable::SourceBounds)
        );
    }
}
