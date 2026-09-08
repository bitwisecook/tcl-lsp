# KCS: Auditing the analyser against C Tcl

> **Audience:** Contributor
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I check whether an analyser finding on real-world Tcl is right, with a
C Tcl interpreter as the oracle?

## Before you start

- A reference `tclsh`. The source trees under `tmp/` (the `fetch-tcl-source`
  skill fetches them) build one with `./configure && make` in `unix/`; the
  distribution's `tclsh8.6` covers version-sensitive cases.
- The server under test: `make rust-server`.
- The `lsp-client` skill, which drives the server over JSON-RPC without an
  editor.

## Answer

1. Start from a real pattern in a real project (tcllib, Tk, a TclOO-heavy
   application), never an invented one. Note the file and line.
2. Reduce it to the smallest `.tcl` file that still shows the behaviour,
   without changing what the code does.
3. Run the file under the reference interpreter to establish ground truth:
   `tclsh9.0` by default, `tclsh8.6` when the behaviour is version-sensitive.
   Verify Tcl semantics; never assume them.
4. Run the same file through the server:
   `python3 .claude/skills/lsp-client/lsp_client.py diagnostics <file>`, or
   `hover` / `definition` / `references` with a position for a
   position-dependent feature.
5. Compare the two and classify: **confirmed** (the server provably diverges
   from `tclsh`), **refuted** (the server is right, or the pattern does not
   reproduce), **plausible** (a divergence that needs a stronger repro), or
   **inconclusive**.
6. File a confirmed finding as an issue carrying the repro, the `tclsh`
   output, and the server output. The fix is registry-driven — `CommandSpec`
   / `SubCommand` data, hooks, traits, or analysis state an earlier pass
   recorded — never a string branch on the command name in the analyser or
   compiler.

## How to tell it worked

The issue holds both outputs, the classification follows from the difference
between them, and the repro file shows the same divergence on a fresh
checkout.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-howto-work-on-fuzz-findings.md](kcs-howto-work-on-fuzz-findings.md)
  — the same discipline for differential-fuzzer findings.
- [Differential fuzzing contracts](../design/contracts/differential-fuzzing.md)
  — the VM-versus-`tclsh` differential that hunts miscompiles.
