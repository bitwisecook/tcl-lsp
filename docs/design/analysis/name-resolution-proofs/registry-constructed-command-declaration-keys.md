# naming.registry.constructed-command-declaration-keys

Kind: `implementation-contract`

## Problem statement

A source declaration query can strip every leading root pair from both a constructed reporting key and the selected descriptor label, conflating literal colon bytes with the one root supplied by the key producer. Its package catalogue is also broader than an installed interpreter table.

## Question

How does the shared Registry declaration query preserve exactly one constructed root and the selected descriptor bytes while keeping operator/method declaration conventions, source import guards and package loading separate?

## Conclusion

The rooted constructed-key query delegates exactly one root removal to Syntax and declines an unrooted argument. The existing rooted/unrooted compatibility query reaches the same unrooted worker. Exact Registry spelling selection and byte-exact CanonicalCommandDescriptorKey label equality precede the existing operator and method-context exclusions; a shared constructed holder projection supplies only that reporting distinction. No descriptor punctuation is stripped or treated as a written-name recipe. Package-required catalogue rows remain declared with their original requirement metadata, independently of actual loading. The two production import conflict consumers already supply rooted constructed keys; their caller Registry selection, source/import/export/force/site and current link gates remain separate. Two fixed software controls cover one-root versus literal-leading-colon selection, trailing/Unicode labels, stock declaration conventions and a genuine package-required row. Their source definitions have no attached assertion or provider execution. A separate direct Syntax helper probe passed six text/byte pairs and two opaque-byte cases covering exact one-root removal and relative refusal; it does not execute either Registry control.

## Scope

Finite CommandRegistry catalogue declaration metadata, its existing compatibility entry, new rooted constructed-key entry and two actual WorkspaceIndex import conflict consumers. This is neither generic written command lookup nor a NameKey/Native namespace resolver. No source reporting key, selected descriptor or catalogue membership establishes current Native command identity, an installed table, package loading or import/export permission. Existing old operator/global-name test bodies are unchanged; their separately documented observations receive no new execution here. This question does not certify the caller Registry selection or its document-store provenance.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Exact Registry descriptor labels and rooted constructed reporting keys. Version: tcl8.4 No external provider execution is attached to this source-catalogue query.

This source-only question records no external tcl8.4 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl8.5

Status: `not-tested`. Channel: Exact Registry descriptor labels and rooted constructed reporting keys. Version: tcl8.5 No external provider execution is attached to this source-catalogue query.

This source-only question records no external tcl8.5 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl8.6

Status: `not-tested`. Channel: Exact Registry descriptor labels and rooted constructed reporting keys. Version: tcl8.6 No external provider execution is attached to this source-catalogue query.

This source-only question records no external tcl8.6 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl9.0

Status: `not-tested`. Channel: Exact Registry descriptor labels and rooted constructed reporting keys. Version: tcl9.0 No external provider execution is attached to this source-catalogue query.

This source-only question records no external tcl9.0 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl9.1

Status: `not-tested`. Channel: Exact Registry descriptor labels and rooted constructed reporting keys. Version: tcl9.1 No external provider execution is attached to this source-catalogue query.

This source-only question records no external tcl9.1 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### jim

Status: `not-tested`. Channel: Exact Registry descriptor labels and rooted constructed reporting keys. Version: jim No external provider execution is attached to this source-catalogue query.

This source-only question records no external jim result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### bigip

Status: `not-tested`. Channel: Exact Registry descriptor labels and rooted constructed reporting keys. Version: bigip No external provider execution is attached to this source-catalogue query.

This source-only question records no external bigip result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

## Consumer bindings

- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `declares_command_at_rooted_key`: One-root constructed-key entry; no written lookup or Native authority.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `declares_command_at_unrooted_key`: One exact selected descriptor label with existing declaration traits, independent of loaded table.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `unroot_rooted_key`: Remove exactly the root supplied by the constructed-key producer.
- [rust/tcl-registry/src/command_descriptor_key.rs](../../../../rust/tcl-registry/src/command_descriptor_key.rs), `CanonicalCommandDescriptorKey`: Byte-exact selected descriptor label; punctuation supplies no root encoding.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `declares_builtin_at`: Guarded source import conflict metadata; caller Registry selection and actual source/import/export/site remain independent.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `registry::declaration_key_tests::rooted_declaration_queries_preserve_one_root_and_exact_descriptor_bytes` (linked): The stock set declaration accepts one constructed root and refuses a repeated-root key that would erase literal colons. A genuinely inserted ::literal descriptor is declared only at its one-root constructed key; selected colon-run, trailing-empty and Unicode labels retain their exact bytes. An unrooted argument to the rooted query declines. These are catalogue metadata results, not written Tcl namespace or installed-command outcomes.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `registry::declaration_key_tests::catalogue_declarations_keep_operator_context_and_package_metadata_separate` (linked): Existing global set, qualified math operator and qualified helper declarations remain available, while bare operator imports and method-only global forms decline. A genuine inserted package-required declaration remains catalogue metadata with its original required_package value; its unrelated global tail is absent. No package load or Native table state is asserted.

## Evidence

- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs)
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs)
- [rust/tcl-registry/src/command_descriptor_key.rs](../../../../rust/tcl-registry/src/command_descriptor_key.rs)
- [rust/tcl-registry/src/spec.rs](../../../../rust/tcl-registry/src/spec.rs)
- [rust/tcl-registry/src/lib.rs](../../../../rust/tcl-registry/src/lib.rs)
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs)
