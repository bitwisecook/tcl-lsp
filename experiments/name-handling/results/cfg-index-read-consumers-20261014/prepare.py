from pathlib import Path
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-cfg-index-read-consumers-20261014');changes={}
def edit(path,old,new):
 before=root.joinpath(path).read_text();after=changes.get(path,(before,before))[1]
 assert after.count(old)==1,(path,after.count(old),old[:70]);changes[path]=(before,after.replace(old,new))
p='rust/tcl-compiler/src/var_resolve.rs'
edit(p,'''    // Read-roles + cmd-sub recursion so `a([idx])` and `a($ns::i)` are covered.
    let mut scanner = VarReferenceScanner::for_registry(
        VarScanOptions {
            include_var_read_roles: true,''',
'''    let config = ctx.invocation_dialect.map_or_else(
        || tcl_lexer::LexerConfig::for_profile(registry.profile()),
        |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    );
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: false,''')
# Restrict constructor argument/scan edit to this function.
before=changes[p][1];start=before.index('fn inner_read_places(');end=before.index('/// Turn an array index',start);section=before[start:end]
old='''        registry,
    );
    let names = scanner.scan_word(text, registry);''';assert section.count(old)==1;section=section.replace(old,'''        config,
    );
    let names = scanner.scan_lexical_word(text, registry);''')
old='''    places.sort_by(|a, b| (&a.ns, &a.name).cmp(&(&b.ns, &b.name)));''';assert section.count(old)==1;section=section.replace(old,'''    if !crate::var_refs::command_subst_texts_with_config(text, config).is_empty() {
        // Detached index text cannot select a command's by-name read operands.
        places.push(place::unknown_top());
    }
    places.sort_by(|a, b| (&a.ns, &a.name).cmp(&(&b.ns, &b.name)));''');changes[p]=(changes[p][0],before[:start]+section+before[end:])
edit(p,'    fn dynamic_name_target_is_unknown_but_reads_the_name_var() {',
'''    fn detached_index_command_keeps_an_unknown_read_without_a_named_cell() {
        let bound = resolve_place("a([set named])", &ctx(), false, &registry());
        let reads = places_read_to_form(&bound);
        assert!(reads.iter().any(|read| read.kind == PlaceKind::Unknown));
        assert!(!reads.iter().any(|read| read.name == "named"));
        assert!(reads.iter().filter(|read| read.kind == PlaceKind::Unknown).all(|read| read.cell.is_none()));
    }

    #[test]
    fn dynamic_name_target_is_unknown_but_reads_the_name_var() {''')
p='rust/tcl-compiler/src/cfg_builder/mod.rs'
edit(p,'        let beside = self.read_beside_writes(&embedded_extras, |scanner| {',
'''        let beside = self.read_beside_writes(
            &embedded_extras,
            self.original_tokens_for_statement(stmt),
            |scanner| {''')
edit(p,'''            crate::ir_helpers::statement_substituted_reads(stmt, &embedded, scanner, self.registry)
        });''',
'''                crate::ir_helpers::statement_substituted_reads(stmt, &embedded, scanner, self.registry)
            },
        );''')
edit(p,'        let beside = self.read_beside_writes(&defs, |scanner| {',
'''        let beside = self.read_beside_writes(&defs, self.original_tokens_at(span), |scanner| {''')
edit(p,'''        written: &[String],
        host_reads: impl FnOnce(&mut VarReferenceScanner) -> BTreeSet<String>,''',
'''        written: &[String],
        tokens: Option<&CommandTokens>,
        host_reads: impl FnOnce(&mut VarReferenceScanner) -> BTreeSet<String>,''')
after=changes[p][1];start=after.index('    fn read_beside_writes(');end=after.index('    fn upvar_effects_from_commands(',start);section=after[start:end]
assert section.count('include_var_read_roles: true')==1;section=section.replace('include_var_read_roles: true','include_var_read_roles: false')
old='''        let base = |name: &str| {
            crate::variable_observation_names::module_observation_key(normalise_var_name(name))
                .to_owned()
        };
        let read: BTreeSet<String> = host_reads(&mut scanner)
            .iter()
            .map(|name| base(name))
            .collect();''';assert section.count(old)==1
section=section.replace(old,'''        let base = |name: &str| {
            crate::variable_observation_names::module_observation_key(
                tcl_syntax::naming::normalise_var_name_braced(name, true),
            ).to_owned()
        };
        let mut read: BTreeSet<String> = host_reads(&mut scanner)
            .iter()
            .map(|name| base(name))
            .collect();
        let Some(children) = tokens.and_then(|tokens| {
            self.procedure_summary_owners.children(tokens, self.registry)
        }) else {
            return written.to_vec();
        };
        for child in &children {
            let Some(binding) = child.source_binding.as_ref() else {
                return written.to_vec();
            };
            let metadata = match self.procedure_summary_owners.module(self.registry) {
                Some(module) => binding.original_invocation_metadata_for_module(child, module, self.registry),
                None => binding.original_source_metadata_for_advice(child, self.registry).ok().flatten(),
            };
            let Some(metadata) = metadata else {
                return written.to_vec();
            };
            let Some(roles) = crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                self.registry, Some(metadata), child,
            ) else {
                return written.to_vec();
            };
            if roles.unknown_residual() || roles.unresolved_roles().any(|role| role == tcl_registry::ArgRole::VarRead) {
                return written.to_vec();
            }
            for (role, word) in roles.operands() {
                if role != tcl_registry::ArgRole::VarRead {
                    continue;
                }
                match word {
                    crate::registry_invocation::EffectiveInvocationWord::Literal(name) => { read.insert(base(name)); }
                    crate::registry_invocation::EffectiveInvocationWord::ArrayElementName { root } => { read.insert(base(root)); }
                    _ => return written.to_vec(),
                }
            }
        }''')
changes[p]=(changes[p][0],after[:start]+section+after[end:])
for path,(before,after) in changes.items():
 for side,value in [('before',before),('after',after)]:
  dest=out/side/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(value)
print(list(changes))
