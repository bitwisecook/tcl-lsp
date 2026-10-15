use tcl_compiler::analyser::{Analyser, MemberSide};
use tcl_compiler::analyser::types::OriginalSourceClassRelationKind;
fn main() {
    let source = "oo::class create ::Meta {superclass oo::class; self method create {name body} {next $name $body}}";
    let analysis = Analyser::new().analyse(source, "tcl").clone();
    for class in analysis.all_classes.values() {
        println!("class={:?} superclasses={:?} class_methods={:?} instance_methods={:?} original_methods={} relations_debug={:?}", class.qualified_name, class.superclasses, class.class_methods.keys().collect::<Vec<_>>(), class.methods.keys().collect::<Vec<_>>(), class.original_members.declarations().count(), class.original_relations);
        println!("class={:?} original_class_methods={} original_instance_methods={} closed_relations={}", class.qualified_name, class.original_members.methods(MemberSide::ClassObject).is_some(), class.original_members.methods(MemberSide::Instance).is_some(), class.original_relations.resolve(MemberSide::Instance, OriginalSourceClassRelationKind::Superclass, |_| Some(())).is_some());
    }
    for original in analysis.original_class_declarations() {
        println!("canonical site={:?} name={:?} methods={} relation_count={}", original.declaration_site(), original.metadata().qualified_name, original.metadata().original_members.declarations().count(), original.metadata().original_relations.effects().count());
    }
}
