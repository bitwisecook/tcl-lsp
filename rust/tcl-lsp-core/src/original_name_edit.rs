// SPDX-License-Identifier: AGPL-3.0-or-later
//! Validated whole-source-container edits from retained original naming inputs.

use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
use tcl_core_types::NameBytes;
use tcl_lexer::{NativeWord, SourceImage, Span};
use tcl_syntax::naming::NamePolicyProtocol;

/// Check a naming input against the complete consumer source and grammar,
/// retaining its authentic whole word or child ordinal lineage. This readonly
/// correspondence issues no lookup, source execution or edit capability.
pub(crate) fn original_input_matches_source(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    recorded: Span,
) -> bool {
    use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
    let image = tcl_lexer::SourceImage::document(source);
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    if !analysis.matches_original_source_image(&image, config) {
        return false;
    }
    if let SignatureSourceNameInput::OriginalVariableRoot(root) = input {
        return root.source_image() == &image && root.lexer_config() == config;
    }
    let Some(realm) = analysis.retained_command_realm() else {
        return false;
    };
    if realm
        .original_written_name_input_at_span_in_source(&image, recorded, config)
        .as_ref()
        == Some(input)
    {
        return true;
    }
    let Some(container) = input.original_static_list_container() else {
        return false;
    };
    if container.parent_word().image() != &image || container.parent_word().config() != config {
        return false;
    }
    let Some(first) = container.parent_word().tokens().first() else {
        return false;
    };
    for span in [container.parent_word().span(), first.span] {
        let Some(mut current) =
            realm.original_written_name_input_at_span_in_source(&image, span, config)
        else {
            continue;
        };
        let mut valid = true;
        for ordinal in container.ordinals() {
            let Some(child) = current.original_list_element(*ordinal) else {
                valid = false;
                break;
            };
            current = child;
        }
        if valid && current == *input {
            return true;
        }
    }
    false
}

/// Exact original source extent for a static native value component. The
/// complete word and selected decoder must reproduce its retained bytes;
/// computed values and list children cannot donate a whole-word extent. This
/// is geometry only, independent of lookup, source currency and edit coverage.
#[must_use]
pub fn original_static_name_value_span(
    input: &SignatureSourceNameInput,
    component: std::ops::Range<usize>,
) -> Option<Span> {
    let SignatureSourceNameInput::OriginalWord(key) = input else {
        return None;
    };
    if component.start > component.end || component.end > input.bytes().len() {
        return None;
    }
    let word = key.original_word();
    let content = word.content_span().ok()?;
    let raw = word.image().bytes().get(content.as_range())?;
    let protocol = input.policy().string_protocol();
    let extent = if word.group().kind == tcl_lexer::WordKind::Braced {
        let literal = tcl_syntax::backslash::native_source_literal_bytes(
            raw,
            word.image().channel(),
            protocol,
        )
        .ok()?;
        if literal.as_ref() != input.bytes() {
            return None;
        }
        tcl_syntax::backslash::native_source_literal_extent(
            raw,
            word.image().channel(),
            protocol,
            component,
        )?
    } else {
        tcl_syntax::backslash::native_source_string_extent(
            raw,
            word.image().channel(),
            word.config().escapes,
            protocol,
            component,
        )?
    };
    Some(Span::new(
        content
            .start()
            .checked_add(u32::try_from(extent.start).ok()?)?,
        content
            .start()
            .checked_add(u32::try_from(extent.end).ok()?)?,
    ))
}

/// One atomic source replacement, independently of LSP coordinate rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalNameSourceEdit {
    span: Span,
    text: String,
}

impl OriginalNameSourceEdit {
    /// Original word/container or lexical name span selected by the edit proof.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Replacement source preserving the selected grammar and untouched values.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

struct ContainerEdits<'a> {
    word: &'a NativeWord,
    policy: NamePolicyProtocol,
    requests: Vec<(Vec<usize>, Vec<u8>)>,
}

fn literal(word: &NativeWord, policy: NamePolicyProtocol) -> Option<Vec<u8>> {
    if word.config().escapes != policy.string_protocol().escape_syntax() {
        return None;
    }
    Some(
        tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            std::slice::from_ref(word),
            policy.string_protocol(),
        )
        .ok()?
        .literal(0)?
        .to_vec(),
    )
}

fn selected(root: &[u8], path: &[usize], policy: NamePolicyProtocol) -> Option<Vec<u8>> {
    let mut value = root.to_vec();
    for ordinal in path {
        value = tcl_syntax::list::split_native_list_bytes(&value, policy.string_protocol())
            .ok()?
            .get(*ordinal)?
            .to_vec();
    }
    Some(value)
}

fn replace_list(
    old: &[u8],
    requests: &[(Vec<usize>, Vec<u8>)],
    policy: NamePolicyProtocol,
) -> Option<Vec<u8>> {
    if let Some((_, wanted)) = requests.iter().find(|(path, _)| path.is_empty()) {
        return requests
            .iter()
            .all(|(path, value)| path.is_empty() && value == wanted)
            .then(|| wanted.clone());
    }
    let original = tcl_syntax::list::split_native_list_bytes(old, policy.string_protocol()).ok()?;
    let mut values = original
        .iter()
        .map(|value| value.to_vec())
        .collect::<Vec<_>>();
    let mut groups = std::collections::BTreeMap::<usize, Vec<(Vec<usize>, Vec<u8>)>>::new();
    for (path, wanted) in requests {
        let ordinal = *path.first()?;
        if ordinal >= values.len() {
            return None;
        }
        groups
            .entry(ordinal)
            .or_default()
            .push((path[1..].to_vec(), wanted.clone()));
    }
    for (&ordinal, children) in &groups {
        values[ordinal] = replace_list(&values[ordinal], children, policy)?;
    }
    let result = tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
        policy.string_protocol(),
    )
    .render(&values);
    let reparsed =
        tcl_syntax::list::split_native_list_bytes(&result, policy.string_protocol()).ok()?;
    if reparsed.len() != original.len()
        || reparsed
            .iter()
            .zip(&values)
            .any(|(got, wanted)| got.as_ref() != wanted)
    {
        return None;
    }
    // Unrequested children must remain byte-identical, even if serialization
    // chooses a different list presentation for another element.
    if original.iter().enumerate().any(|(ordinal, value)| {
        !groups.contains_key(&ordinal) && value.as_ref() != reparsed[ordinal].as_ref()
    }) {
        return None;
    }
    Some(result)
}

fn render_container_literal(
    source: &SourceImage,
    word: &NativeWord,
    wanted: &[u8],
    policy: NamePolicyProtocol,
) -> Option<String> {
    // Retain a plain or braced presentation only after the exact same grammar
    // parses one static word and the channel produces all requested bytes.
    // Quoting and opaque-unit rendering remain the shared decoder's purpose.
    if let Ok(text) = std::str::from_utf8(wanted) {
        let original = source.bytes().get(word.word_span().as_range())?;
        let candidate = if original.starts_with(b"{") {
            format!("{{{text}}}")
        } else {
            text.to_owned()
        };
        let image = match source.channel() {
            tcl_lexer::SourceChannel::Document => SourceImage::document(&candidate),
            tcl_lexer::SourceChannel::NativeValue => SourceImage::native(candidate.as_bytes()),
        };
        if let Ok(plan) = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(candidate.len()).ok()?),
            word.config(),
        ) {
            if let [command] = plan.commands.as_slice() {
                if let [selected] = command.words.as_slice() {
                    if !selected.group().expand
                        && literal(selected, policy).as_deref() == Some(wanted)
                    {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    tcl_syntax::backslash::native_literal_source_word(
        wanted,
        source.channel(),
        word.config(),
        policy.string_protocol(),
    )
}

/// Plan all replacements in one source image. Each requested input must retain
/// a genuine complete static container. Readonly evaluated/fragment producers
/// remain unavailable. Multiple children of one parent produce one edit.
///
/// Full image/channel/configuration, original values, expansion markers, list
/// cardinality and every unrelated element are checked before edits are issued.
#[must_use]
pub fn original_name_input_edits(
    source: &SourceImage,
    requests: &[(SignatureSourceNameInput, NameBytes)],
) -> Option<Vec<OriginalNameSourceEdit>> {
    let mut containers: Vec<ContainerEdits<'_>> = Vec::new();
    for (input, wanted) in requests {
        let container = input.original_static_list_container()?;
        let word = container.parent_word();
        let policy = input.policy();
        if word.image() != source {
            return None;
        }
        let old = literal(word, policy)?;
        if selected(&old, container.ordinals(), policy)?.as_slice() != input.bytes() {
            return None;
        }
        let request = (container.ordinals().to_vec(), wanted.as_bytes().to_vec());
        if let Some(group) = containers
            .iter_mut()
            .find(|group| group.word == word && group.policy == policy)
        {
            if !group.requests.contains(&request) {
                group.requests.push(request);
            }
        } else {
            containers.push(ContainerEdits {
                word,
                policy,
                requests: vec![request],
            });
        }
    }
    let mut edits = Vec::with_capacity(containers.len());
    for group in containers {
        let old = literal(group.word, group.policy)?;
        let wanted = replace_list(&old, &group.requests, group.policy)?;
        let rendered = render_container_literal(source, group.word, &wanted, group.policy)?;
        let marker_extent = Span::new(group.word.span().start(), group.word.word_span().start());
        let markers = source.bytes().get(marker_extent.as_range())?;
        if group.word.group().expand != !markers.is_empty()
            || (!markers.is_empty()
                && (!group.word.config().expand_syntax
                    || markers.chunks(3).any(|marker| marker != b"{*}")))
        {
            return None;
        }
        let text = format!("{}{rendered}", std::str::from_utf8(markers).ok()?);
        let image = match source.channel() {
            tcl_lexer::SourceChannel::Document => SourceImage::document(&text),
            tcl_lexer::SourceChannel::NativeValue => SourceImage::native(text.as_bytes()),
        };
        let plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(text.len()).ok()?),
            group.word.config(),
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        if word.group().expand != group.word.group().expand
            || literal(word, group.policy)? != wanted
        {
            return None;
        }
        for (path, desired) in &group.requests {
            if selected(&wanted, path, group.policy)?.as_slice() != desired {
                return None;
            }
        }
        if old != wanted {
            edits.push(OriginalNameSourceEdit {
                span: group.word.span(),
                text,
            });
        }
    }
    edits.sort_by_key(|edit| (edit.span.start(), edit.span.end()));
    if edits
        .windows(2)
        .any(|pair| pair[0].span.end() > pair[1].span.start())
    {
        return None;
    }
    Some(edits)
}

/// Replace the authentic name extent of one executable variable component.
/// The original image/configuration and literal native units are checked, then
/// its complete substitution is reparsed. Delimiters, the separate index and
/// every byte after the name remain unchanged. This does not turn a lexical
/// root into a complete-word key or establish variable-cell identity.
#[must_use]
pub fn original_variable_root_name_edit(
    source: &SourceImage,
    root: &tcl_compiler::signature_scan::variable_name::SignatureSourceVariableRoot,
    new_name: &str,
) -> Option<OriginalNameSourceEdit> {
    use tcl_compiler::signature_scan::variable_name::SignatureSourceVariableRoot;
    if root.source_image() != source {
        return None;
    }
    let name = root.name_span()?;
    let raw = source.bytes().get(name.as_range())?;
    let original = tcl_syntax::backslash::native_source_literal_bytes(
        raw,
        source.channel(),
        root.policy().string_protocol(),
    )
    .ok()?;
    if original.as_ref() != root.bytes() {
        return None;
    }
    let wanted = tcl_syntax::backslash::native_source_literal_bytes(
        new_name.as_bytes(),
        source.channel(),
        root.policy().string_protocol(),
    )
    .ok()?;
    let part = source.bytes().get(root.part_span().as_range())?;
    let mut replacement = std::str::from_utf8(part).ok()?.to_owned();
    let local = usize::try_from(name.start().checked_sub(root.part_span().start())?).ok()?;
    replacement.replace_range(local..local.checked_add(raw.len())?, new_name);
    let check = match source.channel() {
        tcl_lexer::SourceChannel::Document => SourceImage::document(&replacement),
        tcl_lexer::SourceChannel::NativeValue => SourceImage::native(replacement.as_bytes()),
    };
    let span = Span::new(0, u32::try_from(replacement.len()).ok()?);
    let arena = tcl_lexer::ExecutablePartArena::decompose(
        check.clone(),
        span,
        tcl_lexer::SubstFlags::default(),
        root.lexer_config(),
    )
    .ok()?;
    let [component] = arena.list(arena.root()) else {
        return None;
    };
    let selected = SignatureSourceVariableRoot::from_original_executable(
        &arena,
        &check,
        root.lexer_config(),
        component.span,
        tcl_syntax::word_rules::WordValueRules::from_config(&root.lexer_config()),
        root.policy(),
    )?;
    let selected_name = selected.name_span()?;
    if component.span != span
        || selected_name.start() as usize != local
        || replacement.as_bytes().get(selected_name.end() as usize..)
            != part.get(local + raw.len()..)
        || selected.bytes() != wanted.as_ref()
        || selected.is_separate_array_root() != root.is_separate_array_root()
    {
        return None;
    }
    Some(OriginalNameSourceEdit {
        span: name,
        text: new_name.to_owned(),
    })
}

/// One Unicode editor request encoded through the input's actual source string
/// protocol, then handled by the same complete-container planner.
#[must_use]
pub fn original_name_input_edit(
    source: &str,
    input: &SignatureSourceNameInput,
    desired: &str,
) -> Option<crate::rename::TextEdit> {
    let image = SourceImage::document(source);
    let units = tcl_syntax::backslash::native_source_literal_bytes(
        desired.as_bytes(),
        image.channel(),
        input.policy().string_protocol(),
    )
    .ok()?;
    let edits = original_name_input_edits(&image, &[(input.clone(), units.as_ref().into())])?;
    let [edit] = edits.as_slice() else {
        return None;
    };
    Some(crate::rename::TextEdit {
        range: crate::definition::span_to_range(
            source,
            &tcl_lexer::LineIndex::new(source),
            edit.span,
        ),
        new_text: edit.text.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::signature_scan::scope::SignatureSourceNameKey;
    use tcl_syntax::word_rules::WordValueRules;

    fn input(source: &SourceImage) -> SignatureSourceNameInput {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let config = tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(
            Some("tcl8.6"),
        ));
        let parsed = tcl_lexer::native_script_words_in(
            source.clone(),
            Span::new(0, u32::try_from(source.bytes().len()).unwrap()),
            config,
        )
        .unwrap();
        SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                &parsed.commands[0].words[1],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap(),
        )
    }

    #[test]
    fn original_list_children_batch_one_parent_and_preserve_unrelated_opaque_units() {
        let source = SourceImage::document(r"list {first {nested keep\uD800} last}");
        let parent = input(&source);
        let first = parent.original_list_element(0).unwrap();
        let nested = parent
            .original_list_element(1)
            .unwrap()
            .original_list_element(0)
            .unwrap();
        let edits = original_name_input_edits(
            &source,
            &[
                (first, b"changed".as_slice().into()),
                (nested, b"inside".as_slice().into()),
            ],
        )
        .unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].span(), parent.original_word_key().unwrap().span());
        let mut after = source.try_text().unwrap().to_owned();
        after.replace_range(edits[0].span().as_range(), edits[0].text());
        let parent = input(&SourceImage::document(&after));
        assert_eq!(parent.original_list_element(0).unwrap().bytes(), b"changed");
        let inner = parent.original_list_element(1).unwrap();
        assert_eq!(inner.original_list_element(0).unwrap().bytes(), b"inside");
        assert_eq!(
            inner.original_list_element(1).unwrap().bytes(),
            b"keep\xed\xa0\x80",
            "unrelated decoded native surrogate remains byte-identical"
        );
        assert_eq!(parent.original_list_element(2).unwrap().bytes(), b"last");
    }

    #[test]
    fn static_expanded_container_preserves_marker_and_baked_cardinality() {
        let source = "list {*}{first second third}";
        let image = SourceImage::document(source);
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings = tcl_compiler::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            tcl_compiler::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
        );
        let original = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let parent = bindings
            .original_written_name_input_at_span_in_source(
                &image,
                original.commands[0].words[1].span(),
                config,
            )
            .unwrap();
        assert!(
            parent.original_word_key().is_none(),
            "expanded parent is not an ordinary word key"
        );
        let child = parent.original_list_element(1).unwrap();
        let edits =
            original_name_input_edits(&image, &[(child, b"middle value".as_slice().into())])
                .unwrap();
        assert_eq!(edits.len(), 1);
        assert!(edits[0].text().starts_with("{*}"));
        let mut after = source.to_owned();
        after.replace_range(edits[0].span().as_range(), edits[0].text());
        let parsed = tcl_lexer::native_script_words_in(
            SourceImage::document(&after),
            Span::new(0, u32::try_from(after.len()).unwrap()),
            config,
        )
        .unwrap();
        assert_eq!(parsed.commands[0].words.len(), 2);
        assert!(parsed.commands[0].words[1].group().expand);
        let value = literal(
            &parsed.commands[0].words[1],
            dialect.authored_name_policy().unwrap(),
        )
        .unwrap();
        let values = tcl_syntax::list::split_native_list_bytes(
            &value,
            dialect.authored_name_policy().unwrap().string_protocol(),
        )
        .unwrap();
        assert_eq!(
            values
                .iter()
                .map(|value| value.as_ref())
                .collect::<Vec<_>>(),
            [b"first".as_slice(), b"middle value", b"third"]
        );
    }

    #[test]
    fn computed_frozen_values_remain_readonly_without_static_container() {
        let source = "set target first; list $target";
        let image = SourceImage::document(source);
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings = tcl_compiler::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            tcl_compiler::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
        );
        let start = u32::try_from(source.find("$target").unwrap()).unwrap();
        let input = bindings
            .original_written_name_input_at_span_in_source(
                &image,
                Span::new(start, start + 7),
                config,
            )
            .unwrap();
        assert_eq!(input.bytes(), b"first");
        assert!(input.original_static_list_container().is_none());
        assert!(
            original_name_input_edits(&image, &[(input, b"other".as_slice().into())]).is_none()
        );
    }

    #[test]
    fn original_edit_refuses_stale_channel_conflicting_paths_and_unrenderable_units() {
        let source = SourceImage::document("list {first last}");
        let child = input(&source).original_list_element(0).unwrap();
        let request = [(child.clone(), b"one".as_slice().into())];
        assert!(
            original_name_input_edits(&SourceImage::native(source.bytes()), &request).is_none()
        );
        assert!(
            original_name_input_edits(&SourceImage::document("list {other last}"), &request)
                .is_none()
        );
        assert!(
            original_name_input_edits(
                &source,
                &[
                    (child.clone(), b"one".as_slice().into()),
                    (child.clone(), b"two".as_slice().into())
                ]
            )
            .is_none()
        );
        assert!(
            original_name_input_edits(&source, &[(child, b"raw\0tail".as_slice().into())])
                .is_none()
        );
    }
}

#[cfg(test)]
mod original_edit_presentation_tests {
    use super::*;
    use tcl_compiler::signature_scan::scope::SignatureSourceNameKey;
    use tcl_syntax::word_rules::WordValueRules;

    #[test]
    fn original_edit_preserves_verified_plain_and_braced_container_presentations() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let mut config = tcl_lexer::LexerConfig::default();
        config.escapes = policy.string_protocol().escape_syntax();
        for (source, wanted, expected) in [
            ("list old", b"new".as_slice(), "new"),
            ("list {old keep}", b"new keep".as_slice(), "{new keep}"),
            ("list old", b"new value".as_slice(), "\"new value\""),
        ] {
            let image = SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[1],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            let edits = original_name_input_edits(
                &image,
                &[(SignatureSourceNameInput::OriginalWord(key), wanted.into())],
            )
            .unwrap();
            assert_eq!(edits.len(), 1);
            assert_eq!(edits[0].text(), expected);
        }
    }
}

#[cfg(test)]
mod original_variable_root_edit_tests {
    use super::*;
    use tcl_compiler::signature_scan::variable_name::SignatureSourceVariableRoot;
    fn root(source: &SourceImage) -> SignatureSourceVariableRoot {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let config = tcl_lexer::LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
                .lexer_grammar,
        );
        let arena = tcl_lexer::ExecutablePartArena::decompose(
            source.clone(),
            Span::new(0, u32::try_from(source.bytes().len()).unwrap()),
            tcl_lexer::SubstFlags::default(),
            config,
        )
        .unwrap();
        let [component] = arena.list(arena.root()) else {
            panic!("one executable variable")
        };
        SignatureSourceVariableRoot::from_original_executable(
            &arena,
            source,
            config,
            component.span,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .unwrap()
    }
    #[test]
    fn original_variable_name_edits_keep_wrappers_and_array_index_unchanged() {
        for (before, name, after) in [
            ("${::old::v}", "::new::v", "${::new::v}"),
            ("$::old::a($k)", "::new::a", "$::new::a($k)"),
            ("${::old::a(k)}", "::new::a(k)", "${::new::a(k)}"),
        ] {
            let image = SourceImage::document(before);
            let selected = root(&image);
            let edit = original_variable_root_name_edit(&image, &selected, name).unwrap();
            let mut actual = before.to_owned();
            actual.replace_range(edit.span().as_range(), edit.text());
            assert_eq!(actual, after);
            let result = root(&SourceImage::document(&actual));
            assert_eq!(
                result.is_separate_array_root(),
                selected.is_separate_array_root()
            );
        }
    }
    #[test]
    fn original_c_empty_array_root_edit_preserves_the_independent_index() {
        // Native proof: naming.variable.empty-array-root-lexical-reference
        // docs/design/analysis/name-resolution-proofs/empty-array-root-lexical-reference.md
        // The native observation establishes the C lexical spelling; this
        // assertion separately verifies this editor's exact geometry.
        let image = SourceImage::document("$::old::a(k)");
        let selected = root(&image);
        let edit = original_variable_root_name_edit(&image, &selected, "").unwrap();
        let mut after = std::str::from_utf8(image.bytes()).unwrap().to_owned();
        after.replace_range(edit.span().as_range(), edit.text());
        assert_eq!(after, "$(k)");
        let empty = root(&SourceImage::document(&after));
        assert_eq!(empty.bytes(), b"");
        assert!(empty.is_separate_array_root());
    }
    #[test]
    fn original_variable_name_edits_refuse_stale_or_changed_root_index_boundaries() {
        let image = SourceImage::document("$::old::a(k)");
        let selected = root(&image);
        assert!(
            original_variable_root_name_edit(
                &SourceImage::document("$::other::a(k)"),
                &selected,
                "::new::a"
            )
            .is_none()
        );
        for name in ["::new::a(extra)", "::new::a $other"] {
            assert!(
                original_variable_root_name_edit(&image, &selected, name).is_none(),
                "{name}"
            );
        }
    }
}
