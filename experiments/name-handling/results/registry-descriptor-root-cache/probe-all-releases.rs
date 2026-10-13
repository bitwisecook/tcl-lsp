fn main() {
    let registry = tcl_registry::CommandRegistry::build_default();
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
    let dialect = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment(environment).unit_profile(),
    );
    println!("environment {environment}");
    for name in ["set", "::set", "::::set", "tcl::info::exists", "::tcl::info::exists", "::::tcl::info::exists", "oo::define::method", "::oo::define::method", "::::oo::define::method"] {
        println!("{name:?} {:?}", registry.native_compilation_for_registration(name, dialect));
    }
    }
}
