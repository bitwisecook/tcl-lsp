# Tcl Wiki naming patterns and real-oracle observations

The Tcl Wiki provides concrete naming idioms and adversarial variants for the
[central name-handling contract](../../../docs/design/contracts/name-handling.md).
Native answers are registered in the existing
[name-resolution proof index](../../../docs/design/analysis/name-resolution-proofs/README.md):
29 `naming.wiki.*` questions cover these 30 test cases, with the two mutually
exclusive legacy/modern nested-prefix controls joined into one release question.
The [canonical observation join](../../../rust/tcl-registry/tests/data/native_wiki_naming_patterns/observations.json)
references the original input, real C receipts and exact raw PASS/SKIP lines;
it supplies no additional observation or independent proof authority.
This investigation read 17 existing pages on 2026-10-12. The proposed standalone
`forward` page returned HTTP 200 with **Page not found**; forwarding evidence comes
from the actual `oo::define` and `metaclass` pages. No unread page supplies evidence.

## Evidence and reproduction

[wiki-sources.json](wiki-sources.json) retains each URL, title, page's displayed
Updated timestamp, retrieval date, fetched HTML SHA-256 and the SHA-256 of each
decoded `<pre>` example, indexed in document order from zero. The Updated timestamp
is the Wiki's displayed date, not a verified revision ID; HTTP Last-Modified here
reflects request time and is unsuitable as a revision. The selected example IDs
below refer to those hashes. Full downloaded third-party pages are disposable
inputs under `/tmp/name-wiki`, not copied into this repository.

Pages were fetched with authorised network access using
`curl -LsS --max-time 25 https://wiki.tcl-lang.org/page/<title>`, and their HTML
text and code blocks were inspected. A web-connector search returned an empty
error; direct retrieval succeeded. Wiki discussion includes old behaviour,
corrections and proposals: explanations are hypotheses until actually measured.
The authored reductions preserve the semantic question and avoid importing Tk,
XOTcl, Itcl or other unavailable dependencies.

[wiki-probes.tcl](wiki-probes.tcl) contains 30 fixed positive/adversarial cases.
Each case runs in a fresh child interpreter under the real C interpreter; each
child receives normal `init.tcl` startup. Assertions run through the matching
real upstream `tcltest.tcl`, without replacement commands or harness shims.
[run_wiki_probes.py](run_wiki_probes.py) captures stdout, stderr, process status,
test counts, binary identity, manifest/probe/runner hashes and the SHA-256 of the
actual loaded `init.tcl` and `tcltest.tcl` bytes, including ZIPFS resources.

```sh
source /tmp/name-env.sh
python3 experiments/name-handling/patterns/run_wiki_probes.py \
  --bin-dir "$TCL_LSP_TCL_BIN_DIR" --output /tmp/wiki-probes-new
```

The output directory must be empty. Binary versions come from the shared
[reference manifest](../../../rust/tcl-dialect/data/reference-toolchains.tsv).
These are custom reduced cases run with upstream tcltest, not claims that the
Wiki examples themselves or an upstream test-suite file were executed.

| Real C Tcl | Passed | Skipped | Failed |
| --- | ---: | ---: | ---: |
| 8.4.20 | 21 | 9 | 0 |
| 8.5.19 | 22 | 8 | 0 |
| 8.6.18 | 29 | 1 | 0 |
| 9.0.4 | 29 | 1 | 0 |
| 9.1.0 | 29 | 1 | 0 |

[Final transcripts and provenance](wiki-results/provenance.json) retain exact
test IDs and skip constraints. TclOO is unavailable in these stock 8.4/8.5
builds; seven OO cases skip there. The `{*}` case also skips on 8.4. One of the
mutually exclusive legacy/modern `unknown` cases skips on each release. Skips
provide no oracle answer. C results establish expectations only for C.
[Latest Jim captures](jim-results-5bac7c9/README.md) retain each exact inner
question in a fresh Jim process and the unchanged C suite's harness refusal;
these channels are distinct. The [maintained catalogue](README.md) joins those
outcomes to the C cases. Exact-case F5 answers remain pending in the
[appliance bundle](../../oracles/f5/reports.md#report-9eb804dd1e931e16).

The retained [discovery run](wiki-results/discovery/provenance.json) contains the
initial 28-case hypotheses and failures before correction. It established that
the Wiki's digit-leading implicit-`upvar` failure applies to 8.4/8.5 but not these
8.6+ references, and that `auto_index(::late)` must be tested by invoking absolute
`::late`; an unqualified `late` has a different autoload key. Final fixture
expectations were corrected using these actual observations.

## VM/runtime capture boundaries

The [historical supplied-binary capture](wiki-subject-results/provenance.json)
measured 24 passed / 5 failed / 1 skipped in the VM and 25 passed / 4 failed /
1 skipped in the runtime. Both binaries predate the current merged source, and
their build-to-source correspondence is unverified. Those results are historical
observations of the identified binaries, not a current-source conformance result.

Post-merge attempts retain actual rebuilt binaries and their build/source
receipts separately. They stop before any Wiki harness output or case result:

| Actual attempted engine entry | Capture | Startup refusal |
| --- | --- | --- |
| Runtime before native driver selection | [Receipt](wiki-subject-results/post-merge-runtime-baseline/provenance.json) | `native substitution compiler context native handler policy is not selected` |
| Runtime with selected native bootstrap | [Receipt](wiki-subject-results/post-merge-runtime-native/provenance.json) | `original local alias table receipt native handler policy is not selected` |
| VM with selected native bootstrap and release | [Receipt](wiki-subject-results/post-merge-vm-native/provenance.json) | `native compiler preflight provider required` |

No Wiki test case was observed in these runs. Supplied `init.tcl`/`tcltest.tcl`
source hashes identify the intended matching library; they do not establish
that startup completed or Tcltest loaded. These refusals cannot supply per-case
answers or convert old results into current results. Reached comparisons under
both native engines remain pending.

## Pattern families

Counts below describe selected Wiki examples, not corpus prevalence or eligible
call sites. They do not establish static consumer coverage. Cases returning an
error indicator intentionally test an adversarial condition; they are not
unexecuted cases or missing oracle answers.

| Family and actual source | Selected example IDs / section | Cases | Observed semantic boundary |
| --- | --- | --- | --- |
| [command prefix](https://wiki.tcl-lang.org/page/command+prefix) | 2 (`showsort`), 4–7 (modern/classical/discard), 17–18 (canonicalisation); What is a command prefix / Command prefix flavours | `wiki-prefix-1..3` | Canonical list construction preserves embedded semicolons and spaces. `{*}` expansion treats newlines as list whitespace, whereas `eval` treats them as script separators. A namespace-local command named `list` can replace the presumed builder. |
| [namespace code](https://wiki.tcl-lang.org/page/namespace+code), [namespace inscope](https://wiki.tcl-lang.org/page/namespace+inscope), [namespace eval](https://wiki.tcl-lang.org/page/namespace+eval), [callback](https://wiki.tcl-lang.org/page/callback) | code 1–5; inscope Description; eval 2–4; callback Name resolution / Customization tricks | `wiki-code-1..7` | Capture is namespace context, not a local-variable closure or immutable command. Deleted namespaces fail; recreation under the same spelling changes the observed state. Nested unexpanded code prefixes are repaired by default `unknown` in 8.4/8.5 and rejected in 8.6+. |
| [Namespace MegaWidgets](https://wiki.tcl-lang.org/page/Namespace+MegaWidgets), [proc](https://wiki.tcl-lang.org/page/proc) | MegaWidgets 0–1, especially finite `foreach textCmd` factory; proc Description | `wiki-factory-1..2` | Finite literal loops can create a bounded set of procedures with constant bodies. A braced body referring to a factory local does not capture that local. Wiki megawidget dispatch also depends on ordered class lists and procedures being loaded before introspection. Tk lifecycle itself was not executed. |
| [interp alias](https://wiki.tcl-lang.org/page/interp+alias), [rename](https://wiki.tcl-lang.org/page/rename) | alias 0–1, 13, 28–30; rename 3–4; alias Synopsis | `wiki-alias-1..2`, `wiki-rename-1` | Alias targets retain name and interpreter context; imported commands retain identity across a rename. Replacing the old target slot affects an alias but not that import. Renaming a procedure into another namespace changes its `variable` context. |
| [upvar](https://wiki.tcl-lang.org/page/upvar), [global](https://wiki.tcl-lang.org/page/global), [variable](https://wiki.tcl-lang.org/page/variable) | upvar 1, 10–15, 21–22; global 6,8; variable 3–5,13–17 | `wiki-upvar-1..4`, `wiki-global-1`, `wiki-variable-1` | Wrapper frame forwarding determines which cell is linked. An ordinary helper's `upvar` links in the helper frame. Explicit level accepts `0a`; implicit syntax rejects it in 8.4/8.5, accepts it in 8.6+. `variable a b` assigns `b` to `a`, unlike `global a b` or TclOO's definition-body `variable a b`. |
| [auto_load](https://wiki.tcl-lang.org/page/auto_load), [unknown](https://wiki.tcl-lang.org/page/unknown) | auto_load Description; unknown 0–1,3,5 | `wiki-autoload-1`, `wiki-unknown-1` | A currently missing command can load from a known index or be handled by an overridden `unknown`. Missing table entries alone do not prove an invalid invocation. Unknown implementations may also interpret variable names and expressions. |
| [TclOO](https://wiki.tcl-lang.org/page/TclOO), [metaclass](https://wiki.tcl-lang.org/page/metaclass), [oo::define](https://wiki.tcl-lang.org/page/oo%3A%3Adefine), [callback](https://wiki.tcl-lang.org/page/callback) | TclOO 1,4–7; metaclass 0 and final paragraph; define 0–1; callback Good practices | `wiki-oo-1..7` | Captured `my` can invoke an unexported method; direct object dispatch cannot. Object deletion invalidates its callback namespace. Forward command targets rebind; a `my` forward dispatches internally. Metaclasses can define methods and override `new` to return a non-object. Method replacement resets an explicit export. |

## Shared recogniser requirements

Each row describes a recogniser/summary contract to implement or verify, not a
claim that the analyser already supports it. Runtime execution of a positive
case does not prove an individual static call. The contract's evidence categories
and edit/optimisation proof requirements apply across every consumer.

| Pattern | Preconditions for useful analysis | Mutation / abstention boundary | Shared owner and consumer expectation |
| --- | --- | --- | --- |
| List-built and stored prefixes | Proved builder binding, canonical list provenance, bounded constant head or finite alternatives; registry-declared prefix sink and appended-argument shape | Shadowed builder, unknown writes, noncanonical script/list conversion, unknown invocation context; widen only affected facts | Registry prefix roles/builders plus common value/binding provenance. LSP can offer explicit candidates; compiler direct dispatch needs binding/effect proof. Keep source spans through composition for safe edits. |
| Namespace capture | Proved `namespace code`/`inscope` binding, captured namespace context, identified script/prefix semantics and invocation phase | Namespace deletion/recreation, command replacement, variable writes, object destruction; a namespace name alone is not durable identity | Shared namespace lookup and binding lifecycle. Track frame as well as namespace; do not promote proc locals into closures. Version-gate old `unknown` repair. |
| Factories and namespace/widget wrappers | Finite literal inputs, body-construction provenance, actual execution namespace, known substitution stages and ordered dispatch tables | Dynamic templates, arbitrary `regsub`/`subst`, unknown load order, changing class lists, Tk hull destruction; never infer arbitrary factories from a `proc` spelling | Registry definition-body grammars and declared summaries, common source/load graph and binding facts. Generated definitions may have finite identities but lack editable definition spans. |
| Alias/import/rename | Known source/target interpreters, prefix values, creation order and imported command identity | Retarget, slot replacement, import/forget, rename, interpreter deletion and proc namespace changes | Shared command identity/binding owners. Keep aliases' name-based target semantics distinct from import identity; use runtime fallback when lifecycle cannot be proved. |
| Variable/frame wrappers | Proved helper body or typed effect summary, explicit level/relative frame reach, declaration grammar and variable-name provenance | Unknown `uplevel` level, reentrant traces, escaping frame lifetime, qualified/array names, wrapper mutation | Shared frame/place and registry roles. Definitions/navigation identify cells and links; rewrites require alias completeness. Do not equate core `variable` with TclOO definition-body declarations. |
| Unknown/autoload | Actual `unknown` binding/effects; finite index key, loader body, known source path and load order | Index replacement, arbitrary loader side effects, custom namespace unknown, absent external package context | Shared source graph, registry and command-binding facts. An unresolved head needs justified abstention rather than a definite missing-command diagnostic. |
| OO handles, forwards and metaclasses | Proved class/metaclass identity, method resolution/export facts, receiver provenance and factory return summary | Overridden `new`/`create`, mixins/filters, dynamic method changes, destruction/rename, method export reset | Registry definer grammars/member effects and shared receiver/binding facts. Do not assign an object type to every `$class new` result; distinguish internal `my` access from public dispatch. |

## Existing consumer contracts

These patterns extend the existing source/binding owners. They do not justify
a parallel resolver or reissuing execution authority from Wiki examples.

| Wiki family | Existing canonical contract | Independent obligation |
| --- | --- | --- |
| List construction, stored callbacks, builder shadowing | [Produced command-prefix source](../../../docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md) and [structured timing](../../../docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md) | Authentic whole builder/parent inputs and selected registry purpose precede source advice. That advice does not establish list evaluation or later callback entry. |
| Alias prefix and lifecycle | [Captured prefix allocation](../../../docs/design/analysis/name-resolution-proofs/alias-captured-prefix-allocation-contract.md) | Retained alias allocation and original prefix lineage remain separate from current name lookup, imported command identity and frame execution. |
| Caller-place wrappers | [Variable alias source schema](../../../docs/design/analysis/name-resolution-proofs/original-variable-alias-source-schema.md) and [caller-frame navigation](../../../docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md) | Source declaration/navigation candidates do not manufacture a reached variable cell, current value or complete rename alias inventory. |
| Unknown/autoload | [Unknown-handler source context](../../../docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md) | Secondary source assistance retains the actual source/context and residual alternatives; an unobserved handler or loader cannot manufacture a target. |
| TclOO forwards | [Forward registration prefix](../../../docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md) and [native forward boundaries](../../../docs/design/analysis/name-resolution-proofs/tcloo-native-forward-registration-prefix-boundaries.md) | Registration and occupied method identity do not prove future target dispatch. |
| TclOO factory results | [Stock factory Normal transfer](../../../docs/design/analysis/name-resolution-proofs/tcloo-original-stock-class-factory-normal-transfer.md) | Intrinsic stock constructor, current workers, original whole argv and actual completed allocation are independent premises. An overridden metaclass `new` does not satisfy them merely by spelling. |

The [consumer integration map](../../../docs/design/analysis/name-resolution-proofs/consumer-integrations.md)
is the shared index for current source consumers. New positive/adversarial
coverage should bind to those existing owners and preserve their source/native,
binding/cell, frame, completion and edit distinctions.

Bounded backward slicing at declared sinks should come before whole-program
template evaluation. A useful first set is canonical prefix composition,
namespace capture, explicit frame wrappers, finite factories and OO factory
return summaries. Adversarial fixtures must cover builder shadowing, intervening
mutation, namespace/object destruction, different interpreter/frame contexts
and alternative methods/factory returns alongside ordinary positive examples.

The Wiki broadens variants; the maintained pinned corpus supplies independent
project breadth and denominators. Held-out project coverage, false target
selection, missed provable targets and candidate usefulness remain unmeasured
for these recognisers. Their implementation must retain those measures before
claiming conformance or completing a consumer migration. BIG-IP adaptations
must first identify the actual appliance execution context and capability;
these desktop child-interpreter probes are not ready-to-load iRules.
