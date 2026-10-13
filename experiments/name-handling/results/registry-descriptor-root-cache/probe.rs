fn main() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let dialect = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile(),
    );
    for name in ["set", "::set", "::::set", "tcl::info::exists", "::tcl::info::exists", "::::tcl::info::exists", "oo::define::method", "::oo::define::method", "::::oo::define::method"] {
        println!("{name:?} {:?}", registry.native_compilation_for_registration(name, dialect));
    }
}
