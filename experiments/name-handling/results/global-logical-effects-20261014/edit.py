from pathlib import Path
root=Path('/tmp/name-global-logical-effects-20261014/after/rust/tcl-compiler/src')
p=root/'registry_invocation.rs'; s=p.read_text(); marker='/// Purity of the selected original substitution implementation.'
assert marker in s
addition='''/// Conditional variable destinations and aliases of the exact Logical source.
/// This is a readonly MAY footprint, without Native entry or completed stores.
pub(crate) fn original_logical_variable_footprint_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<(
    tcl_registry::VariableWriteProjection,
    Vec<tcl_registry::VariableCellAliasTransition>,
)> {
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    let selected = original_logical_operation_invocation_with_metadata_context(
        registry, context, tokens,
    )?;
    let realm = binding.original_source_realm_for_input(
        tokens, context.source_analysis_input()?, registry,
    )?;
    selected.with_metadata_schema(registry, context, realm, |schema| {
        let aliases = schema.state_transitions().facts().iter().filter_map(|fact| {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                return None;
            };
            Some(alias.clone())
        }).collect();
        Some((
            CommandRegistry::variable_write_projection_for_selected_source(schema),
            aliases,
        ))
    })
}

'''
s=s.replace(marker,addition+marker,1); p.write_text(s)
p=root/'cfg_builder/global_write_info.rs'; s=p.read_text()
marker='''    fn standalone(&self) -> bool {'''
addition='''    fn logical_source(&self) -> bool {
        self.input.is_some_and(|input| input.has_logical_source_name_context())
    }

    fn logical_footprint(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<(
        tcl_registry::VariableWriteProjection,
        Vec<tcl_registry::VariableCellAliasTransition>,
    )> {
        crate::registry_invocation::original_logical_variable_footprint_with_metadata_context(
            self.registry, self.metadata?, tokens,
        )
    }

'''
assert marker in s; s=s.replace(marker,addition+marker,1)
s=s.replace('''        statement: &Statement,
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        if !self.standalone() {''','''        statement: &Statement,
        tokens: Option<&crate::ir::CommandTokens>,
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        if self.logical_source() {
            return tokens.and_then(|tokens| self.logical_footprint(tokens))
                .map(|(writes, _)| writes)
                .unwrap_or(tcl_registry::VariableWriteProjection {
                    opaque_variable_frame: true,
                    ..Default::default()
                });
        }
        if !self.standalone() {''',1)
s=s.replace('''    if aliases.script_has_opaque_binding_effect(body, registry, namespace)
        || script_invokes_unknown_binding(body, aliases, namespace)
    {''','''    if !semantics.logical_source()
        && (aliases.script_has_opaque_binding_effect(body, registry, namespace)
            || script_invokes_unknown_binding(body, aliases, namespace))
    {''',1)
s=s.replace('''            collect_renamed_outer_alias(
                stmt,
                renamed_aliases,''','''            collect_renamed_outer_alias(
                stmt,
                script.retained_source_tokens_for_statement(stmt),
                renamed_aliases,''',1)
s=s.replace('''fn collect_renamed_outer_alias(
    stmt: &Statement,
    renamed_aliases:''','''fn collect_renamed_outer_alias(
    stmt: &Statement,
    tokens: Option<&crate::ir::CommandTokens>,
    renamed_aliases:''',1)
needle='''    for invocation in semantics.resolve_statement(stmt, namespace) {
        collect_renamed_outer_alias_facts(&invocation.facts, renamed_aliases, semantics);
    }
}'''
replacement='''    if semantics.logical_source() {
        if let Some((_, aliases)) = tokens.and_then(|tokens| semantics.logical_footprint(tokens)) {
            for alias in &aliases {
                collect_renamed_outer_alias_transition(alias, renamed_aliases, semantics);
            }
        }
        return;
    }
    for invocation in semantics.resolve_statement(stmt, namespace) {
        collect_renamed_outer_alias_facts(&invocation.facts, renamed_aliases, semantics);
    }
}'''
assert needle in s;s=s.replace(needle,replacement,1)
# Extract transition application, preserving each existing MAY operand case.
start=s.index('''        let Some(outer) = selected_outer_alias_target(&alias.target, semantics)''',s.index('fn collect_renamed_outer_alias_facts'))
end=s.index('''\n}\n\nfn collect_write_targets''',start)
block=s[start:end]
assert block.endswith('    }'); block=block[:-5]
s=s[:start]+'''        collect_renamed_outer_alias_transition(alias, renamed_aliases, semantics);
    }
}

fn collect_renamed_outer_alias_transition(
    alias: &tcl_registry::VariableCellAliasTransition,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
) {
'''+block.replace('            continue;','            return;',1)+s[end:]
# Thread actual script-retained carriers, including consumed typed statements.
s=s.replace('''fn own_write_targets(
    stmt: &Statement,''','''fn own_write_targets(
    script: &Script,
    stmt: &Statement,''',1)
s=s.replace('''let direct = semantics.statement_writes(stmt, command_namespace.as_ref());''','''let direct = semantics.statement_writes(
        stmt, script.retained_source_tokens_for_statement(stmt), command_namespace.as_ref(),
    );''',1)
s=s.replace('own_write_targets(stmt, semantics, namespace)','own_write_targets(script, stmt, semantics, namespace)')
s=s.replace('own_write_targets(stmt, semantics, &ExecutionNamespace::exact(namespace))','own_write_targets(script, stmt, semantics, &ExecutionNamespace::exact(namespace))')
# The original assertions remain; the source now actually declares its holders.
s=s.replace('''            "proc ::b::run {} { global bbb; set bbb 1 }\\n\\
             proc ::a::run {} { global aaa; set aaa 1 }\\n",''','''            "namespace eval ::b {}; namespace eval ::a {}\\n\\
             proc ::b::run {} { global bbb; set bbb 1 }\\n\\
             proc ::a::run {} { global aaa; set aaa 1 }\\n",''',1)
p.write_text(s)
