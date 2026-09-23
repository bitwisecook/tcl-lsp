# KCS: Where does tcl-lsp decide whether a diagnostic is shown?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp CLI, MCP, diagnostic

## Question

Where is the decision made that a diagnostic is shown, hidden, or
relabelled, and where does a new rule go?

## Answer

In one place: the `diagnostic_policy` module of the `tcl-lsp-core` crate.
Every producer converts its own finding into one shared shape and filters
nothing. That covers the analyser, the compiler checks, the optimiser, the
style pass, the byte-integrity pass, the SslicTcl loader, the XC translator,
and the BIG-IP validators. The policy step then pairs every finding with an
outcome: shown, with the severity and tag to display, or suppressed, with
the reason. Nothing is deleted, so "why is this code not firing" has one
answer.

The reasons apply in a fixed order, and the first one that fires is the one
recorded:

1. The document-wide gates: diagnostics turned off for the folder, or an
   excluded file.
2. Encoding abstention, when the bytes are not UTF-8 text. Only the
   integrity codes survive it.
3. An inline `# noqa` on the command, then a top-of-file
   `# tcl-lsp: disable=`.
4. The per-code decision from the configuration layers: the global file,
   the editor settings or the command-line flags, then the project file.
   This is the order the [suppression guide](kcs-howto-suppress-diagnostics.md)
   promises, and the layer that decided is recorded with the reason.
5. The optimiser switch and profile, then the shimmer switch.
6. The overlap table: W110 owns O120 at the same span, and in a SslicTcl
   document the loader owns W123.

Three whole-file codes ignore an inline `# noqa`, because they have no line
to attach it to: W107, W109, and W118. The file directive and the
configuration layers still reach them.

Every surface reads one report built this way: the editor's published and
pulled diagnostics, `tcl diag`, `tcl lint`, `tcl validate` and `tcl opt`,
the MCP tools, the editor's *Optimise document* command, and the
code-action lightbulb. Each one runs its producers, hands the findings to
the policy step, and renders what the report shows. A quick-fix or a
rewrite is offered only for a finding the report shows. A new rule goes
into the policy step, never into a producer or a surface; the
[design page](../design/compiler/diagnostic-policy.md) tracks the slices
still to come.

## Related

- [KCS index](README.md)
- [How do I turn a diagnostic off?](kcs-howto-suppress-diagnostics.md)
- [Diagnostic policy design](../design/compiler/diagnostic-policy.md)
- [Glossary](../GLOSSARY.md)
