from pathlib import Path
p=Path('/workspace/name-implementation-worktrees/lsp_scope/rust/tcl-compiler/src/const_subst.rs');s=p.read_text();mark='impl ConstSubstCtx<\'_> {'
# Inspect actual impl spelling.
if mark not in s: mark='impl<\'a> ConstSubstCtx<\'a> {'
assert mark in s
struct='''struct OriginalConstSubstSource<'a> {
    image: &'a tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    variable: &'a dyn Fn(&tcl_lexer::ExecutablePartArena, &tcl_lexer::SpannedExecutablePart) -> Option<String>,
}

'''
s=s.replace(mark,struct+mark,1)
# Extend only internal traversal; existing public faces keep their independent callbacks.
s=s.replace('self.fold_at_depth(inner, 0, None)','self.fold_at_depth(inner, 0, None, None)')
s=s.replace('self.fold_at_depth(inner, 0, Some(dialect))','self.fold_at_depth(inner, 0, Some(dialect), None)')
s=s.replace('self.literal_words_at_depth(inner, 0, None)','self.literal_words_at_depth(inner, 0, None, None)')
s=s.replace('''        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<ResolvedConstSubst>''','''        dialect: Option<tcl_registry::InvocationDialect>,
        source: Option<(&OriginalConstSubstSource<'_>, tcl_lexer::Span)>,
    ) -> Option<ResolvedConstSubst>''',1)
s=s.replace('self.literal_words_at_depth(inner, depth, dialect)?','self.literal_words_at_depth(inner, depth, dialect, source)?',1)
s=s.replace('''        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<(
        Vec<String>,''','''        dialect: Option<tcl_registry::InvocationDialect>,
        source: Option<(&OriginalConstSubstSource<'_>, tcl_lexer::Span)>,
    ) -> Option<(
        Vec<String>,''',1)
s=s.replace('''        use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};

        // Re-split''','''        use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};
        if let Some((source, span)) = source {
            return self.original_literal_words_at_depth(source, span, depth, dialect);
        }

        // Re-split''',1)
s=s.replace('self.fold_at_depth(nested, depth + 1, dialect)?','self.fold_at_depth(nested, depth + 1, dialect, None)?',1)
at=s.index('    fn fold_at_depth(')
new='''    /// Fold an authentic original command-substitution operand with typed variable sources.
    /// Complete image/configuration and component spans survive nested folds;
    /// the caller independently establishes command trust and value provenance.
    #[must_use]
    pub fn fold_original_source_substitution(
        &self,
        operand: &tcl_lexer::NativeWord,
        variable: &dyn Fn(&tcl_lexer::ExecutablePartArena, &tcl_lexer::SpannedExecutablePart) -> Option<String>,
    ) -> Option<String> {
        let parts = operand.executable_parts();
        let [component] = parts.list(parts.root()) else { return None; };
        let tcl_lexer::ExecutablePart::Command { body } = component.part else { return None; };
        let source = OriginalConstSubstSource { image: operand.image(), config: operand.config(), variable };
        let inner = source.image.try_text().ok()?.get(body.as_range())?;
        self.fold_at_depth(inner, 0, None, Some((&source, body))).map(|fold| fold.value)
    }

    fn original_literal_words_at_depth(
        &self,
        source: &OriginalConstSubstSource<'_>,
        span: tcl_lexer::Span,
        depth: u32,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<(Vec<String>, Vec<CommandBindingIdentity>, Vec<tcl_runtime_api::SiteClaim>)> {
        let plan = tcl_lexer::native_script_words_in(source.image.clone(), span, source.config).ok()?;
        let [command] = plan.commands.as_slice() else { return None; };
        if plan.fatal_tail.is_some() { return None; }
        let mut words = Vec::new();
        let mut command_bindings = Vec::new();
        let mut site_claims = Vec::new();
        for word in &command.words {
            let parts = word.executable_parts();
            if let [component] = parts.list(parts.root())
                && let tcl_lexer::ExecutablePart::Command { body } = component.part
            {
                let inner = source.image.try_text().ok()?.get(body.as_range())?;
                let folded = self.fold_at_depth(inner, depth + 1, dialect, Some((source, body)))?;
                words.push(folded.value);
                command_bindings.extend(folded.command_bindings);
                site_claims.extend(folded.site_claims);
            } else {
                let value = tcl_syntax::word_rules::original_word_unicode_value_with_variables(word,
                    |arena, component| (source.variable)(arena, component).map(String::into_bytes))?;
                words.push(String::from_utf8(value).ok()?);
            }
        }
        Some((words, command_bindings, site_claims))
    }

'''
s=s[:at]+new+s[at:];p.write_text(s)
