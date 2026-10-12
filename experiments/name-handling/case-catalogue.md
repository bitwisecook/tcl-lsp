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

## Dynamic-pattern acceptance

These are required static-analysis acceptance scenarios. They extend the
experiment backlog; they are not claims that probes or consumer implementations
already exist. Keep one positive example and a counterexample for each pattern,
with C/Jim/F5 expectations scoped to actual supported contexts.

Discover representative variants in the maintained corpus and Tcl Wiki using
[pattern-sources.md](pattern-sources.md). Retain repository revision/file/line
or Wiki page/revision/date for each source-backed case; extend examples beyond
the snippets in this matrix and validate on held-out projects.

| Pattern | Useful result when established | Required counterexample / boundary |
| --- | --- | --- |
| Constant command head: `set cmd helper; $cmd` | Flow-sensitive name value plus the correct call-time binding | Reassign the value or rename/shadow/delete the command before invocation. |
| `${ns}::helper`, `format` or list-built names | Preserve constructed spelling, provenance and namespace context | Dynamic prefix, shadowed builder, lone-colon/display collision or unknown later definition. |
| `[list helper $arg]`, `{*}$prefix` callbacks | Command-prefix roles, target candidates and bound arguments | Unknown list head/expansion, value position instead of command-prefix role, or a caller frame different from the callback frame. |
| `namespace code` / `inscope` | Captured namespace and execution scope | Shimmered text versus retained namespace identity, deleted/recreated namespace or opaque callback execution. |
| Alias/import/ensemble wrappers | Shared binding/lifecycle facts and inherited roles | Prefix-argument shifts, import snapshots, intervening rename/delete/redefinition, force conflicts or map mutation. |
| `upvar` / namespace variable wrappers | Target frame, alias cell and read/write effects from a supported summary | Computed level, different caller frame, self/cyclic link, dead target or open-world name argument. |
| Package/autoload/source conventions | Indexed target with declared environment and load-order evidence | Unindexed or conditional source, replaced package implementation, cyclic order or a second possible definition. |
| Finite procedure factories | Definitions derived from bounded literal inputs and known definition semantics | Empty/unknown iteration, opaque eval body, conditional execution or renamed/shadowed definer. |
| OO receiver and framework provenance | Object/class candidates and known method namespace/grammar | Reassignment, dynamic metaclass or factory, mutable mixins/forwards and same-name definition commands. |
| Tcltest setup/body/cleanup | Shared frame and ordered definitions/reads across registry-declared bodies | Shadowed `test`, different wrapper contract or conditional setup/completion that cannot establish the later state. |
| F5 rule `call` and protocol prefixes | Rule-qualified procedure identity and registry/context availability | Unknown rule/partition/profile, duplicate tails across rules or a protocol prefix mistaken for a Tcl namespace. |

For every static case retain source, expected identity/candidate/abstention,
reason, provenance spans, affected surface and real-oracle semantic observations.
An oracle execution establishes that program's behaviour; the consumer test
also establishes whether its available static evidence permits the conclusion.
Do not require static certainty merely because an oracle can execute the code.
Completion, navigation candidates, diagnostics, rename and optimisers need
separate expectations reflecting their evidence requirements.
