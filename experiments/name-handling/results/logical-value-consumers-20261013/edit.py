from pathlib import Path
r=Path('/workspace/name-implementation-worktrees/lsp_scope/rust/tcl-compiler/src/analyser')
p=r/'scope.rs'; s=p.read_text(); mark='impl Analyser {\n    /// Record a constant string assignment'
s=s.replace(mark,'''#[derive(Debug, Clone)]
pub(super) struct LogicalConstStringProvenance {
    image: tcl_lexer::SourceImage,
    input: super::ResolvedAnalysisInput,
    declaration: Span,
}

impl Analyser {
    /// Record a constant string assignment''',1)
old='''        self.const_strings
            .entry(scope_path.to_vec())'''
new='''        let provenance = self.result.resolved_input.as_ref().and_then(|input| {
            let declaration = scope_at(&self.result.global_scope, scope_path)?
                .variables.get(var_name)?;
            (self.result.allows_lexical_declaration_advice()
                && declaration.link_target.is_none())
                .then(|| LogicalConstStringProvenance {
                    image: tcl_lexer::SourceImage::document(&self.source),
                    input: input.clone(),
                    declaration: declaration.definition_span,
                })
        });
        let records = self.logical_const_string_provenance
            .entry(scope_path.to_vec()).or_default();
        if let Some(provenance) = provenance {
            records.insert(var_name.to_owned(), provenance);
        } else {
            records.remove(var_name);
        }
        self.const_strings
            .entry(scope_path.to_vec())'''
assert old in s;s=s.replace(old,new,1)
old='''    pub fn clear_const_string(&mut self, var_name: &str, scope_path: &[usize]) {
'''
s=s.replace(old,old+'''        if let Some(map) = self.logical_const_string_provenance.get_mut(scope_path) {
            map.remove(var_name);
        }
''',1)
start=s.index('    /// The constant-string value recorded for `base_name`')
end=s.index('\n    }',s.index('    pub fn lookup_const_string_in_namespace',start))+6
# Previous function has nested walk; find end using next function doc marker.
end=s.index('\n    ///',end)
s=s[:start]+'''    /// A dominating value from the exact declaration selected by Logical source advice.
    /// Linked cells and array elements require independent value provenance.
    pub(super) fn lookup_logical_const_string_at(
        &self,
        written: &str,
        span: Span,
    ) -> Option<(&str, Span)> {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let advice = self.result.logical_variable_scope_advice(
            &image, self.file_lexer_config(), span.start(),
        )?;
        let declaration = advice.variable_definition(written)?;
        if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let mut candidates = self.logical_const_string_provenance.iter().flat_map(|(path, records)| {
            records.iter().filter_map(move |(name, provenance)| {
                let scope = scope_at(&self.result.global_scope, path)?;
                let retained = scope.variables.get(name)?;
                if !std::ptr::eq(retained, declaration)
                    || provenance.declaration != declaration.definition_span
                    || &provenance.input != input
                    || !self.result.matches_original_source_image(
                        &provenance.image, self.file_lexer_config(),
                    )
                    || self.nondominating_consts.get(path).is_some_and(|names| names.contains(name))
                {
                    return None;
                }
                let (value, value_span) = self.const_strings.get(path)?.get(name)?;
                (value_span.end() < span.start()).then_some((value.as_str(), *value_span))
            })
        });
        let value = candidates.next()?;
        candidates.next().is_none().then_some(value)
    }

    /// A scalar substitution retains its original braced-name form.
    pub(super) fn lookup_logical_const_substitution(&self, token: Token) -> Option<(&str, Span)> {
        if token.kind != TokenType::Var {
            return None;
        }
        let map = Self::source_map(
            &self.source, &self.cached_line_index, self.cached_line_index_source_len,
        );
        let name = map.token_text(token);
        if token.content_offset != 2 && crate::naming::split_array_name_braced(name, false).1.is_some() {
            return None;
        }
        self.lookup_logical_const_string_at(name, token.span)
    }
''' + s[end:]
p.write_text(s)
p=r/'state.rs';s=p.read_text();s=s.replace('    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,','''    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,
    pub(super) logical_const_string_provenance:
        HashMap<Vec<usize>, HashMap<String, super::scope::LogicalConstStringProvenance>>,''',1);s=s.replace('            const_strings: HashMap::new(),','            const_strings: HashMap::new(),\n            logical_const_string_provenance: HashMap::new(),',1);p.write_text(s)
p=r/'snapshot.rs';s=p.read_text();s=s.replace('    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,','''    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,
    pub(super) logical_const_string_provenance:
        HashMap<Vec<usize>, HashMap<String, super::scope::LogicalConstStringProvenance>>,''',1);s=s.replace('            const_strings: self.const_strings.clone(),','            const_strings: self.const_strings.clone(),\n            logical_const_string_provenance: self.logical_const_string_provenance.clone(),',1);s=s.replace('        self.const_strings = snap.const_strings;','        self.const_strings = snap.const_strings;\n        self.logical_const_string_provenance = snap.logical_const_string_provenance;',1);p.write_text(s)
p=r/'handlers.rs';s=p.read_text();start=s.index('            TokenType::Var => {',s.index('    fn resolve_const_word('));end=s.index('\n            _ =>',start);s=s[:start]+'''            TokenType::Var => self.lookup_logical_const_substitution(tok)
                .map(|(value, _)| value.to_owned()),'''+s[end:]
start=s.index('            let sm = Self::source_map(',s.index('    fn resolve_dynamic_apply_lambda('));end=s.index('            Some((',start);s=s[:start]+'''            let (value, span) = self.lookup_logical_const_substitution(word_tok)?;
'''+s[end:]
s=s.replace('''            .lookup_const_string(&args[1], scope_path)
            .map(str::to_owned)''','''            .lookup_logical_const_string_at(&args[1], arg_tokens[1].span)
            .map(|(value, _)| value.to_owned())''',1)
# Word resolver no longer walks the caller's path independently.
s=s.replace('''        scope_path: &[usize],
    ) -> Option<String> {
        if !is_single''','''        _scope_path: &[usize],
    ) -> Option<String> {
        if !is_single''',1)
s=s.replace('''        scope_path: &[usize],
    ) -> Option<Vec<(Token, String)>> {
        let one_hop''','''        _scope_path: &[usize],
    ) -> Option<Vec<(Token, String)>> {
        let one_hop''',1)
p.write_text(s)
