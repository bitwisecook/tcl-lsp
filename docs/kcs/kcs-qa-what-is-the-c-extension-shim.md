# KCS: What is the C extension shim and when should I use it?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

tcl-lsp CLI, mcp

## Question

What is the C extension shim, and when should I use it instead of a Tcl hook body or a native Rust hook?

## Answer

The [C extension shim](../GLOSSARY.md#c-extension-shim) (`rust/tcl-cshim`) lets a command written against the C
Tcl API run on the project's own engines. You compile the extension's C
source against the shim's header, `include/tclshim.h`, instead of `tcl.h`
(usually just an include swap), and load its `<Pkg>_Init` entry point into a
shim interpreter from Rust. The commands it registers with
`Tcl_CreateObjCommand` then work like any other command on the engine: the
engine's words become `objv`, and `Tcl_SetObjResult` becomes the result.
Integers and lists cross without turning into text, and error messages such
as `wrong # args` and `bad subcommand` match C Tcl byte for byte.

Use it when you already have working C code for a Tcl command and want that
exact behaviour available to the project's bytecode virtual machine (the
`tclvm` engine in [spec-packs.md](../design/spec-packs.md#what-exists-today))
or, later, the WASM engine, without rewriting it. Do not reach for it to add
behaviour to a [SpecTcl](../design/spec-packs.md) pack: a pack's hooks are
small Tcl bodies that
run in a sandbox with a budget, and a native hook in a shipped pack is a
`-native` reference to Rust code the server already contains. The shim is
neither. It is trusted native code, loaded only by the host process's own
configuration, and no `.tclspec` can name it, `load` it, or call a command it
registered. Loading one is an `unsafe` call in Rust for exactly that reason:
the shim contains Rust panics at the boundary, but it cannot limit or contain
what the C code itself does.

The first leg covers the argument-handling core: registration, the object and
list API, the integer, double, boolean, and index conversions, and the result
and error-code API. Channels, the event loop, threads, `Tcl_Eval`, and binary
compatibility with a real `libtcl` are out of scope. The full subset, the
value-marshalling rules, and the trust model are in the
[design doc](../design/c-extension-shim.md).

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [The C Tcl extension shim (design)](../design/c-extension-shim.md)
- [SpecTcl packs (design)](../design/spec-packs.md)
