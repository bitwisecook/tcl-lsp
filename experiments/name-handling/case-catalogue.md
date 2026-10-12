# Portable case catalogue

`cases.tcl` records observations; it does not assert a prewritten answer.
Every case has a unique namespace and runs from the global script frame in
a fresh interpreter. Namespace/import/path/alias capabilities differ by oracle;
the starter catch combines setup and lookup, so interpret setup failures using
retained errorInfo and rerun in isolation. Dialect/context adaptations must
record explicit setup/lookup phases. All current outcomes are ASCII; extend
the byte/text capture contract before adding Unicode/binary variants.

| ID | Question | Consumers |
| --- | --- | --- |
| NH01 | Does current-namespace bare command win over global? | binding, analyser, LSP, runtime dispatch |
| NH02 | Does a relative-qualified command fall back when the current qualifier namespace exists but the command does not? | candidates, registry, optimiser, runtime |
| NH03 | Does `namespace path` apply to relative-qualified names? | static candidates, token/tree policy adapters |
| NH04 | Does a child namespace implicitly search ancestors? | same lookup consumers, unresolved-call diagnostics |
| NH05 | Is `n::x` a namespace variable inside a proc? | place resolver, SSA, load/store codegen, both runtimes |
| NH06 | Does an undefined unqualified namespace-scope variable read fall back globally? | release-specific read/introspection policy |
| NH07 | Do `x` and `::x` share a top-level place? | SSA/value transfers/summaries/optimisation |
| NH08 | Are `{}`, `{$n}` and `{{a}}` exact distinct variable names? | ingress, variable roles, stack forms, rename/diagnostics |
| NH09 | Do colon runs collapse in written command names while trailing separators name the empty command? | parser/candidates/spans, codegen and runtimes |
| NH10 | Can distinct `a:`/`p` and `a`/`:p` slots render identically while both retain their behaviour? | structured identity, workspace/indexes/caches/rename |
| NH11 | Does an import keep working across source rename, with updated origin? | binding lifecycle, command tokens, LSP links |
| NH12 | Does the imported name observe source redefinition? | import identity/lifetime; record the live result and lifecycle identity |
| NH13 | Does a late-bound interp alias observe a target redefinition? | alias identity versus target name, invalidation |
| NH14 | Does a same-frame `upvar 0` write address the original cell? | variable aliases, place facts, optimisation |
| NH15 | Does `upvar 1` see a namespace-eval caller frame? | frame reach, VM/WASM, call summaries |
| NH16 | Does `namespace forget` remove an import from the global namespace? | import pattern owner, global-root handling |

Extend with retained cases for exact #2286 `tcltest` setup/body/cleanup
sequencing, namespace missing-parent definitions, qualified alias destination
creation and description, source deletion and import chains, aliases in child
interpreters, namespace activation-delayed deletion/recreation, compiled cached
descendant tokens, trace mutation windows, OO forwards/method namespace,
mathfunc lookup, command/variable error options, binary names and complex array
indices. Those need their own oracle-specific setup and are not claimed covered
by these sixteen starter observations.
