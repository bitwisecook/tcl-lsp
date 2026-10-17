# Parent schema generic projection repair

The actual integrated build reported recursion-limit monomorphisation at the DefinitionMember parent source-schema projection inside generic with_source_arguments. Its local closure recursively changes the generic projection type at each instantiation. That one projection now calls the module-level nongeneric source_definition_body_grammar function item. The original parent/source/config/context/Registry/descriptor checks and all existing assertions remain unchanged. No recursion limit is raised, no owner discarded, no parser or semantics added.

Source-only validation: current beforeimage exact; scoped rustfmt; Rust AST parse; entire test module byte exact; fresh fuzz-zero forward/inverse exact. No Cargo/build/tests/traces/provider execution and no claim the actual build has passed.
