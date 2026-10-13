from pathlib import Path
p=Path('/workspace/name-implementation-worktrees/lsp_scope/rust/tcl-syntax/src/word_rules.rs');s=p.read_text();start=s.index('pub fn original_static_word_unicode_value(');end=s.index('\n}',start)+2
old=s[start:end]
# Retain the original numeric escape validator once, as a component helper.
a=old.index('            let raw = arena.bytes(component.span)?;');b=old.index('\n        }\n    }\n    std::str::from_utf8',a)
validator=old[a:b]
replacement='''pub fn original_static_word_unicode_value(word: &tcl_lexer::NativeWord) -> Option<Vec<u8>> {
    original_word_unicode_value_with_variables(word, |_, _| None)
}

/// Unicode source value with each substitution resolved from its original component.
/// The callback retains the complete image, grammar and scalar/array form;
/// unavailable commands, indices or values never become replacement text.
#[must_use]
pub fn original_word_unicode_value_with_variables(
    word: &tcl_lexer::NativeWord,
    mut variable: impl FnMut(&tcl_lexer::ExecutablePartArena, &tcl_lexer::SpannedExecutablePart) -> Option<Vec<u8>>,
) -> Option<Vec<u8>> {
    use tcl_lexer::{ExecutablePart, SourceChannel, WordKind};
    if word.image().channel() != SourceChannel::Document || word.group().expand {
        return None;
    }
    word.image().try_text().ok()?;
    let mut value = Vec::new();
    if word.group().kind == WordKind::Braced {
        value = original_static_word_source_bytes(word)?;
    } else {
        let arena = word.executable_parts();
        for component in arena.list(arena.root()) {
            let bytes = match &component.part {
                ExecutablePart::Text(_) => original_unicode_text_component(arena, component)?,
                ExecutablePart::Variable { .. } => variable(arena, component)?,
                _ => return None,
            };
            value.extend_from_slice(&bytes);
        }
    }
    std::str::from_utf8(&value).ok()?;
    Some(value)
}

fn original_unicode_text_component(
    arena: &tcl_lexer::ExecutablePartArena,
    component: &tcl_lexer::SpannedExecutablePart,
) -> Option<Vec<u8>> {
    use tcl_lexer::{ExecutablePart, ExecutableText, SourceChannel};
    let value = match &component.part {
        ExecutablePart::Text(ExecutableText::Original) => crate::backslash::source_literal_bytes(
            arena.bytes(component.span)?, SourceChannel::Document,
        ).into_owned(),
        ExecutablePart::Text(ExecutableText::Decoded(bytes)) => {
'''+validator.replace('word.config().escapes','arena.config().escapes')+'''
            bytes.clone()
        }
        _ => return None,
    };
    std::str::from_utf8(&value).ok()?;
    Some(value)
}'''
s=s[:start]+replacement+s[end:];p.write_text(s)
