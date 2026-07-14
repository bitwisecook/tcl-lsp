# KCS: How are Tcl command names resolved across namespaces?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, analyser

## Question

When a script calls a command by a bare name (`helper`), a relative
qualified name (`inner::p`), or an absolute name (`::inner::p`), which
definition does tcl-lsp decide it dispatches to?

## Answer

The same way C Tcl does. The rule, in priority order for a call made from
namespace `ns`:

1. An absolute name (starting with `::`) names exactly one command.
2. Any relative name tries `ns` first, then each `namespace path` entry
   in order, then the global namespace — and dispatches the **first
   candidate that exists**. A namespace merely existing does not count;
   the command must exist.
3. There is no ancestor walk: `helper` inside `::a::b` never reaches
   `::a::helper` unless `::a` is on the `namespace path`.
4. Resolution is at call time, so a definition later in the file still
   wins for calls made from procedure bodies.

One shared implementation (`resolve_command_with` in the `tcl-syntax`
crate) backs the analyser, the optimiser, the bytecode virtual machine,
and the WASM runtime. A conformance table of scenarios, executed against
real tclsh 8.6 and 9.0, keeps every implementation in agreement — see the
[command-resolution contract](../design/contracts/command-resolution.md)
for the algorithm, the consumer list, and how to add a scenario.

One static limitation: the analyser does not yet track `namespace path`
declarations, so editor features assume an empty path. The virtual
machine and the WASM runtime honour the real path at run time.
