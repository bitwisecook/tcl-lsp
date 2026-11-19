use std::path::Path;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::model::ingress::resolve_environment;
fn main() {
    let args=std::env::args().collect::<Vec<_>>();
    let source=std::fs::read_to_string(Path::new(&args[2])).expect("UTF-8 script");
    let selected=args[1].as_str();
    let regions=match selected {
        "query-dsl" | "declarative-sslic" => Vec::new(),
        "scf-regions"=>tcl_bigip::rule_extract::find_embedded_rules(&source).into_iter().map(|r|("f5-irules",r.body)).collect(),
        // APL callback Tcl lacks a measured appliance runtime. Select only core
        // unversioned Tcl grammar, never the iRule or iApp implementation world.
        "apl-regions"=>tcl_bigip::apl::embedded_tcl_regions(&source).into_iter().map(|(a,b)|("tcl",source[a as usize..b as usize].to_owned())).collect(),
        _=>vec![(selected,source)],
    };
    if regions.is_empty(){println!("no compiled Tcl region");}
    for (dialect,source) in regions {
    let env=resolve_environment(dialect);
    let context=env.default_context_registry();
    let unit=CompilationUnit::build_for_dialect(&source,context.commands(),false,dialect);
    let mut functions=vec![("top",0,&unit.top_level)];
    for (kind, bodies) in [("procedure",&unit.procedures),("method",&unit.methods),("other",&unit.body_units)] {
        functions.extend(bodies.iter().map(|(id,f)|(kind,id.ordinal(),f)));
    }
    functions.sort_unstable_by_key(|(kind,ordinal,_)|(*kind,*ordinal));
    for (kind,ordinal,f) in functions {
        let _=std::hint::black_box(f.semantic_values());
        let _=std::hint::black_box(f.semantic_values());
        let cloned=f.clone();
        let _=std::hint::black_box(cloned.semantic_values());
        if args.get(3).is_some_and(|arg|arg=="invalidate") {
            let mut cloned=cloned;
            cloned.invalidate_semantic_values();
            let _=std::hint::black_box(cloned.semantic_values());
        }
        println!("{kind}\t{ordinal}\t{:?}\t{}",f.name,f.sccp.values.len());
    }
    }
}
