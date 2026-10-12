# naming.registry.constructed-command-declaration-keys

Kind: `implementation-contract`

## Problem statement

A source declaration query can strip every leading root pair from both a constructed reporting key and the selected descriptor label, conflating literal colon bytes with the one root supplied by the key producer. Its package catalogue is also broader than an installed interpreter table.

## Question

How does the shared Registry declaration query preserve exactly one constructed root and the selected descriptor bytes while keeping operator/method declaration conventions, source import guards and package loading separate?

## Conclusion

The rooted constructed-key query delegates exactly one root removal to Syntax and declines an unrooted argument. The existing rooted/unrooted compatibility query reaches the same unrooted worker. Exact Registry spelling selection and byte-exact CanonicalCommandDescriptorKey label equality precede the existing operator and method-context exclusions; a shared constructed holder projection supplies only that reporting distinction. No descriptor punctuation is stripped or treated as a written-name recipe. Package-required catalogue rows remain declared with their original requirement metadata, independently of actual loading. The two production import conflict consumers already supply rooted constructed keys; their caller Registry selection, source/import/export/force/site and current link gates remain separate. Two fixed software controls cover one-root versus literal-leading-colon selection, trailing/Unicode labels, stock declaration conventions and a genuine package-required row. Their source definitions have no attached assertion or provider execution.

## Scope

Finite CommandRegistry catalogue declaration metadata, its existing compatibility entry, new rooted constructed-key entry and two actual WorkspaceIndex import conflict consumers. This is neither generic written command lookup nor a NameKey/Native namespace resolver. No source reporting key, selected descriptor or catalogue membership establishes current Native command identity, an installed table, package loading or import/export permission. Existing old operator/global-name test bodies are unchanged; their separately documented observations receive no new execution here. This question does not certify the caller Registry selection or its document-store provenance.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: No external provider execution is attached to this source-catalogue query.. Build: Current Registry/Syntax source and two fixed software control definitions only.. Channel: Exact Registry descriptor labels and rooted constructed reporting keys.. Dialect: tcl8.4.

This source-only question records no external tcl8.4 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl8.5

Status: `not-tested`. Version: No external provider execution is attached to this source-catalogue query.. Build: Current Registry/Syntax source and two fixed software control definitions only.. Channel: Exact Registry descriptor labels and rooted constructed reporting keys.. Dialect: tcl8.5.

This source-only question records no external tcl8.5 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl8.6

Status: `not-tested`. Version: No external provider execution is attached to this source-catalogue query.. Build: Current Registry/Syntax source and two fixed software control definitions only.. Channel: Exact Registry descriptor labels and rooted constructed reporting keys.. Dialect: tcl8.6.

This source-only question records no external tcl8.6 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl9.0

Status: `not-tested`. Version: No external provider execution is attached to this source-catalogue query.. Build: Current Registry/Syntax source and two fixed software control definitions only.. Channel: Exact Registry descriptor labels and rooted constructed reporting keys.. Dialect: tcl9.0.

This source-only question records no external tcl9.0 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### tcl9.1

Status: `not-tested`. Version: No external provider execution is attached to this source-catalogue query.. Build: Current Registry/Syntax source and two fixed software control definitions only.. Channel: Exact Registry descriptor labels and rooted constructed reporting keys.. Dialect: tcl9.1.

This source-only question records no external tcl9.1 result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### jim

Status: `not-tested`. Version: No external provider execution is attached to this source-catalogue query.. Build: Current Registry/Syntax source and two fixed software control definitions only.. Channel: Exact Registry descriptor labels and rooted constructed reporting keys.. Dialect: jim.

This source-only question records no external jim result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

### bigip

Status: `not-tested`. Version: No external provider execution is attached to this source-catalogue query.. Build: Current Registry/Syntax source and two fixed software control definitions only.. Channel: Exact Registry descriptor labels and rooted constructed reporting keys.. Dialect: bigip.

This source-only question records no external bigip result. Descriptor equality and constructed-key reporting do not authenticate a current installed command, package load, Native namespace, import or execution.

## Exact evidence

- `registry-declaration-source-0` (implementation): [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs). SHA-256 `1de8aafe803a4ca72bdd3374c6348938f1e75ec4f9c7b2501127e6e27de17024`. Current shared constructed-key/descriptor/declaration owner or actual guarded import consumer source. Fixed controls remain unexecuted; no external provider or installed-table outcome.
- `registry-declaration-source-1` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `66784f60107d941e748c803a64f0ca052d6dd601cc95b4e24ca5233552c2a87d`. Current shared constructed-key/descriptor/declaration owner or actual guarded import consumer source. Fixed controls remain unexecuted; no external provider or installed-table outcome.
- `registry-declaration-source-2` (implementation): [rust/tcl-registry/src/command_descriptor_key.rs](../../../../rust/tcl-registry/src/command_descriptor_key.rs). SHA-256 `a813cd6be686d087e807d9468b256c5dd16d94bfef83d7093c3537b57adb2379`. Current shared constructed-key/descriptor/declaration owner or actual guarded import consumer source. Fixed controls remain unexecuted; no external provider or installed-table outcome.
- `registry-declaration-source-3` (implementation): [rust/tcl-registry/src/spec.rs](../../../../rust/tcl-registry/src/spec.rs). SHA-256 `553a01b03ae5834bf1650fbc573c9a38562906492ebbe726d45da9e920b11381`. Current shared constructed-key/descriptor/declaration owner or actual guarded import consumer source. Fixed controls remain unexecuted; no external provider or installed-table outcome.
- `registry-declaration-source-4` (implementation): [rust/tcl-registry/src/lib.rs](../../../../rust/tcl-registry/src/lib.rs). SHA-256 `51e694f4609dd94828b00d3cac9d9aff73d723d97677e9cb537aa573f5b2e593`. Current shared constructed-key/descriptor/declaration owner or actual guarded import consumer source. Fixed controls remain unexecuted; no external provider or installed-table outcome.
- `registry-declaration-source-5` (implementation): [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs). SHA-256 `feefb4b2da2ae821f3ebec02282bd7a92da59461cfbccfd7f09bf21aea6c13dc`. Current shared constructed-key/descriptor/declaration owner or actual guarded import consumer source. Fixed controls remain unexecuted; no external provider or installed-table outcome.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `declares_command_at_rooted_key`: One-root constructed-key entry; no written lookup or Native authority.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `declares_command_at_unrooted_key`: One exact selected descriptor label with existing declaration traits, independent of loaded table.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `unroot_rooted_key`: Remove exactly the root supplied by the constructed-key producer.
- [rust/tcl-registry/src/command_descriptor_key.rs](../../../../rust/tcl-registry/src/command_descriptor_key.rs), `CanonicalCommandDescriptorKey`: Byte-exact selected descriptor label; punctuation supplies no root encoding.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `declares_builtin_at`: Guarded source import conflict metadata; caller Registry selection and actual source/import/export/site remain independent.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `registry::declaration_key_tests::rooted_declaration_queries_preserve_one_root_and_exact_descriptor_bytes` (linked): The stock set declaration accepts one constructed root and refuses a repeated-root key that would erase literal colons. A genuinely inserted ::literal descriptor is declared only at its one-root constructed key; selected colon-run, trailing-empty and Unicode labels retain their exact bytes. An unrooted argument to the rooted query declines. These are catalogue metadata results, not written Tcl namespace or installed-command outcomes.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `registry::declaration_key_tests::catalogue_declarations_keep_operator_context_and_package_metadata_separate` (linked): Existing global set, qualified math operator and qualified helper declarations remain available, while bare operator imports and method-only global forms decline. A genuine inserted package-required declaration remains catalogue metadata with its original required_package value; its unrelated global tail is absent. No package load or Native table state is asserted.

A named test is a coverage binding, not a claim that it executed.

## Replay

Two fixed source definitions support later owned-software reconfirmation. No new test assertions, C Tcl/Jim/BIG-IP observations, command installation, package load or private/native chronology are attached.
