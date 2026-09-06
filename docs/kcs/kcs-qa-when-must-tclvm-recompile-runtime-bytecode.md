# KCS: When must TclVM recompile runtime bytecode?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

tcl-vm and TclVM embedders

## Question

If an embedder replaces TclVM's compiler or dialect profile, which compiled
bodies can be reused?

## Answer

Treat a compiler or dialect-profile replacement as a new compile target, even
when the Tcl release did not change. TclVM clears cached eval modules.
Procedures, TclOO methods, and function handles keep their source and compile
again on first use.

Do not copy only the bytecode and attach the interpreter's current generation
later. Carry the whole [compiled artefact](../GLOSSARY.md#compiled-artefact)
into its activation. A live frame or suspended coroutine has already advanced
its program counter, so TclVM cannot safely start it again from newly compiled
source. It reports a fail-closed error instead.

When adding a reusable bytecode consumer, store `CompiledUnit`, retain source
when lazy recompilation is valid, and test a same-profile default-to-custom
compiler swap. Public `ModuleAsm` values are embedder-owned: TclVM admits their
top-level bytecode without claiming its compiler produced it, and recompiles
their source-bearing procedures through the current service on first entry. A
self-contained module can still define and call its supplied procedures when
the VM has no compiler; those procedures remain foreign so installing a
compiler later invalidates them.
The complete ownership and invalidation rules are in the
[TclVM compiled-artifact contract](../design/contracts/vm-compiled-artifact-provenance.md).

## Related

- [Is the command registry fixed at compile time?](compiler/kcs-qa-is-the-command-registry-fixed-at-compile-time.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
