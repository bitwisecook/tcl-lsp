# naming.tcloo.original-stock-metaclass-no-hook-selection

Kind: `implementation-contract`

## Problem statement

A registered stock metaclass without a command compiler must preserve ordinary argv evaluation under an unresolved compilation mode. Omitting its no-hook descriptor turns a genuine selected command into an unknown compiler candidate before independently validated factory transfer can run.

## Question

Do the selected stock TclOO metaclass descriptors retain their no-hook compiler role across supported releases without granting factory, body or object authority?

## Conclusion

The ordinary class and supported derived stock metaclasses share the existing Registry no-hook compilation descriptor. Selection returns Generic argv evaluation under the supported actual Tcl recipe even when the compilation mode is Unknown. Unsupported release surfaces remain absent; successful manufacture, definition execution and native class-role capabilities require independent owners.

## Scope

Rust Registry metadata selection for oo::class under C8.6 and later and oo::configurable, oo::abstract and oo::singleton under C9.0 and later. Tests cover all five Tcl release selectors and unavailable earlier surfaces. This is a compiler-role descriptor, not an observation of native guest execution, a current command token, factory Normal completion or installed object state.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No guest execution of this Rust implementation question is claimed.

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `OO_NATIVE_COMPILATION`: Shared intrinsic stock object-command no-hook descriptor, without factory completion or body authority.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::select`: Generic argv settlement for independently selected no-hook descriptors; compiler-bearing unresolved modes remain unknown.
- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `commands::tcl::oo_class::tests::original_stock_metaclass_compilation_has_no_hook_without_factory_authority` (linked): Available stock metaclasses select NoHook and Generic under Unknown compilation mode; earlier unsupported release surfaces remain unavailable.
