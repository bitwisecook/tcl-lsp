use tcl_compiler::analyser::Analyser;
use tcl_compiler::registry_invocation::source_structure::{source_registry_words_at, source_class_declaration_at, OriginalRegistrySource};

fn main() {
    let fixtures = [
        ("alias_coverage", "proc p {} {global shared; puts $shared}"),
        ("class_navigation", concat!(
            "namespace eval left {oo::class create Widget {}}\n",
            "namespace eval right {oo::class create Widget {}}\n",
            "namespace eval left {Widget new}\n",
            "namespace eval right {Widget new}\n",
            "namespace eval other {Widget new}\n",
            "namespace eval left {::right::Widget new}\n",
        )),
        ("factory_header", "oo::class create ::Meta {superclass oo::class; self method create {name body} {next $name $body}}"),
    ];
    for (label, source) in fixtures {
        let analysis = Analyser::new().analyse(source, "tcl").clone();
        let input = analysis.resolved_input.as_ref().expect("actual input");
        let context = input.context_registry();
        println!("fixture={label} classes={} logical={} source_variable_symbols={}", analysis.all_classes.len(), analysis.allows_retained_logical_declaration_advice(), analysis.original_variable_symbols.len());
        if let Some(key) = analysis.original_variable_symbols.iter().find_map(|symbol| symbol.original_name_input().original_word_key()) {
            println!("fixture={label} alias_source_coverage={}", analysis.original_variable_alias_source_is_complete(key.source_image(), analysis.body_lexer_config.unwrap()));
        }
        for (offset, _) in source.match_indices("oo::class create") {
            let offset = u32::try_from(offset).unwrap();
            let words = source_registry_words_at(source, &analysis, offset);
            println!("offset={offset} registry_words={}", words.is_some());
            if let Some(words) = words {
                let kind = match words.source() { OriginalRegistrySource::Selected => "Selected", OriginalRegistrySource::SourceTransitions(_) => "SourceTransitions", OriginalRegistrySource::Conditional(_) => "Conditional", _ => "Other" };
                println!("offset={offset} source_kind={kind} command={:?} argc={}", words.command(), words.arguments().len());
                for body in words.source_script_bodies(&context) {
                    let kind = match body.source_words().source() { OriginalRegistrySource::Selected => "Selected", OriginalRegistrySource::SourceTransitions(_) => "SourceTransitions", OriginalRegistrySource::Conditional(_) => "Conditional", _ => "Other" };
                    println!("offset={offset} body={:?} parent_kind={kind} region={}", body.original_container().span(), body.definition_member_region(&context, body.content_span()).is_some());
                }
            }
            let declaration = source_class_declaration_at(source, &analysis, offset);
            println!("offset={offset} class_declaration={}", declaration.is_some());
            if let Some(declaration) = declaration {
                println!("offset={offset} declaration_slot={:?} obligations={:?} logical_input={}", declaration.source_slot(), declaration.factory().obligations(), declaration.factory().logical_source_input().is_some());
                for word in declaration.factory().original_words() {
                    println!("offset={offset} original_word_span={:?} tokens={:?}", word.span(), word.tokens());
                }
            }
        }
        for class in analysis.all_classes.values() {
            println!("class={:?} source_name={} factory={} provenance={:?} name_span={:?} body_span={:?}", class.qualified_name, class.source_name.is_some(), class.factory.is_some(), class.metaclass_provenance, class.name_span, class.body_span);
            for effect in class.original_relations.effects() {
                println!("class={:?} relation_effect side={:?} operation={:?} values={}", class.qualified_name, effect.side(), effect.operation(), effect.values().map_or(0, |values| values.len()));
                for value in effect.values().unwrap_or(&[]) {
                    println!("class={:?} relation={:?} provider={}", class.qualified_name, value.name_input().bytes(), value.source_provider().is_some());
                    if let Some(provider) = value.source_provider() {
                        println!("class={:?} provider_analysis={} provider_class={} registry_descriptor={:?}", class.qualified_name, provider.matches_analysis(&analysis), provider.matches_class_definition(class), provider.registry_descriptor(&context).map(|spec| spec.name));
                    }
                }
            }
        }
    }
}
