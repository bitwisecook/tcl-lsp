# KCS: What is the C extension shim and when should I use it?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

tcl-lsp CLI, mcp

## Question

What is the C extension shim, and when should I use it instead of a Tcl hook body or a native Rust hook?

## Answer

The [C extension shim](../GLOSSARY.md#c-extension-shim) (`rust/tcl-cshim`)
lets a command written against the C Tcl API run on the project's own
engines. You compile the extension's C source against the project's own
`tcl.h` (`runtime/rust/include/tcl.h`; the shim implements its native leg and
the WASM runtime the other) and load its `<Pkg>_Init` entry point into a shim
interpreter from Rust, or let the host's scripts `load` it: a host that links
the extension in registers a `load` command over a table of entry points
(`StaticExtensions`), which answers Tcl's `load` by prefix (`load {} Pkga`, or a file name such as
`libpkga.so` that stands for it) and loads each prefix once. The
commands it registers with `Tcl_CreateObjCommand` then work like any other
command on the engine: the engine's words become `objv`, and
`Tcl_SetObjResult` becomes the result. Constructed integer and list payloads,
plus full scalar caches, can cross structurally. Existing string bytes travel independently, preserving
raw NUL, modified NUL and non-Unicode bytes; pure byte-array backing is not
substituted for its native string. Guest message, error-code and return-options
bytes retain their original spelling. Unicode-only metadata handlers explicitly
refuse a value without a checked Unicode view.

Use it when you already have working C code for a Tcl command and want that
exact behaviour available to the project's bytecode virtual machine (the
`tclvm` engine in [spec-packs.md](../design/registry/spec-packs.md#what-exists-today))
without rewriting it. Arguments are reconstructed shim objects: their cache
mutations do not write back to the original VM arguments or preserve their
sharing relationships. Commands that depend on those effects exceed this
interface. Do not reach for it to add
behaviour to a [SpecTcl](../design/registry/spec-packs.md) pack: a pack's hooks are
small Tcl bodies that run in a sandbox with a budget, and a native hook in a
shipped pack is a `-native` reference to Rust code the server already
contains. The shim is neither. It is trusted native code, loaded only by the
host process's own configuration, and no `.tclspec` can name it, `load` it,
or call a command it registered. Loading one is an `unsafe` call in Rust for
exactly that reason, and so is building the table a host's `load` reads: the
shim contains Rust panics at the boundary, but it cannot limit or contain what
the C code itself does.

The shim covers the argument-handling core: registration, the object and list
API, the C9 primitive integer, double, boolean and index conversions, and the
result and error-code API. A command can read, write and unset variables in
its caller's frame and evaluate a script there (`Tcl_GetVar2Ex`,
`Tcl_ObjSetVar2`, `Tcl_UnsetVar2`, `Tcl_EvalObjEx`). Native command publication
requires an engine-owned preparation service that captures the actual
interpreter and namespace incarnation before callbacks. Engines without that
capability, or without support for a reached synchronous script deletion trace,
return a typed host refusal. Channels, the event loop, threads, the rest of the
`Tcl_Eval` family, and binary compatibility with a real `libtcl` are out of scope.
The full subset, the
value-marshalling rules, and the trust model are in the
[design doc](../design/runtime/c-extension-shim.md).

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [The C Tcl extension shim (design)](../design/runtime/c-extension-shim.md)
- [SpecTcl packs (design)](../design/registry/spec-packs.md)
