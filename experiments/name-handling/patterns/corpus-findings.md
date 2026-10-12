# Corpus naming findings

The pinned public corpus supplies useful naming idioms that a shared resolver
should recognise under explicit proofs. The examples also show why computed
bytes, list prefixes, framework receiver handles and caller-frame effects must
remain separate facts. Source popularity does not establish a call target.

The [current name model](../../../docs/design/analysis/name-resolution.md),
[command owner contract](../../../docs/design/contracts/command-resolution.md)
and [canonical proof records](../../../docs/design/analysis/name-resolution-proofs/README.md)
govern Original C/Jim inputs, hosted policy and explicit Logical reporting.
This experiment supplies discovery contexts and exact reduced observations;
it does not create a second proof authority. Consumer obligations are maintained
in [the canonical integration contracts](../../../docs/design/analysis/name-resolution-proofs/consumer-integrations.md).
Original lookup, entered frame, completed world, readonly source advice and
editable geometry remain independently issued facts.

All 35 public checkouts in manifest scope `everything` were fetched and
verified on 2026-10-12. No public checkout was missing or mismatched. The
[provenance](corpus-provenance.json) records the manifest hash, actual revisions
and commands; [fetch](corpus-fetch.txt) and [verification](corpus-verify.txt)
transcripts retain the results. No benchmark pin or workload changed.

## Evidence and coverage bounds

`rg` located candidates, then execution context was manually reviewed in
19 selected source ranges across 12 repositories. Eighteen ranges contain
eligible naming operations; one is metadata data that must not be recognised
as an executed operation. These are selected examples, not a corpus-wide
semantic census or a prevalence percentage. No static resolver accuracy,
missed-target rate or refactoring success rate has been measured.

[sites.json](sites.json) records each exact repository revision, file,
one-based line range, file/range SHA-256, source URL, licence reference,
execution context and associated reduction. Third-party source remains in the
ignored checkouts. Retained probes are authored reductions of semantic
questions, not vendored excerpts or executions of the full applications.

The fetcher reports 2,649 selected `*.tcl`, `*.irule` and `*.tmsh`
files. Research also inspected `tclopt/test/test.test` and
`simple-sideband/simple_sideband.irul`; those suffixes are outside that
selection count. Corpus mining must respect actual source formats, including
test files, while excluding JavaScript grammars, comments and quoted metadata
from executable naming-site counts.

Tcllib build fragments and their generated/amalgamated files are one family.
For example, the HTTP server callback was inspected in
`modules/httpd/build/server.tcl`; its copy in `httpd.tcl` is not an
independent example. The Tk dialog state idiom repeated in pave's bundled
picker similarly supplies a derivative, not independent coverage.

## Concrete patterns and proof requirements

| Family and reviewed source IDs | Context and useful shared inference | Proof bound / counterexample |
| --- | --- | --- |
| Tcltest lifecycle: `corpus-tcltest` | tclopt imports `tcltest::test`, creates optimiser/parameter values in `-setup`, invokes an optimiser handle in `-body`, and reads them in `-cleanup`. Ordered body effects and shared execution scope must feed all name/variable consumers. | Prove that the invocation is the registry definer and its body phases share the frame. A same-spelled user `test` cannot acquire Tcltest semantics. The C reduction establishes payload continuity, not the correctness of the complete Mpfit application or closure of #2286. |
| Computed qualified heads: `corpus-computed-head` | ticklecharts derives `ns` from `namespace qualifiers [self class]` and invokes `${ns}::setdef`. Track class-handle provenance through a shared summary into name construction and call-time binding. | Class origin may be conditional or unknown. Constant bytes alone do not prove the command occupying the slot; intervening rename/redefinition changes the target. |
| Conditional aliases and fixed prefixes: `corpus-current-alias`, `corpus-alias-prefix` | ruff uses `namespace current` to install qualified aliases, selecting CLI or HTTP implementations, and exposes formatter aliases with fixed operation arguments. Record destination identity, target interpreter, target name and exact prefix elements. | Executable/package availability selects a branch. Retain finite alternative implementations unless that selection is proved. An alias's describe token survives rename while its dispatch name changes. |
| Caller-place wrappers: `corpus-state-upvar`, `corpus-frame-forward`, `corpus-mock-frame` | ruff's parameter names a caller array; pave forwards a list-built self call with `uplevel 1`; TesTcl installs values in its own frame before catching a dynamic body. Model exact composed frame reach and name-argument effects. | The `eval` near miss writes the forwarding procedure's frame, leaving the outer cell unchanged. A generic wrapper summary cannot erase that difference. TesTcl mocks do not prove BIG-IP/TMM execution semantics. |
| TclOO callbacks: `corpus-self-callback`, `corpus-captured-my` | pave builds `[list [self] method …]`; Tcllib's server captures `[namespace code [list my connect]]` for sockets and `namespace code {my CheckTimeout}` for cron. Preserve receiver identity, captured namespace, exact prefix elements and callback arguments. | `self` and captured `my` are different receiver mechanisms. Registration frame is not invocation frame. Object destruction/namespace teardown can retire the target before execution. Host-specific callback argument rules require their registry contracts. |
| Keyed variable links: `corpus-keyed-tk-state` | Tk constructs a qualified dialog state array from `winfo name`; its configured `-typevariable` supplies another name consumed by `upvar #0`. Keep array identity and element keys distinct from frame/cell links. | Actual widget-derived name is conditional on Tk host facts. The C reduction proves an explicitly known key containing spaces can link to one array, not that every `winfo` result is statically known. |
| EDA ensemble prefixes: `corpus-eda-ensemble` | OSVVM's tee has an ensemble unknown handler returning a list prefix, an inner ensemble with a parameter, and a `namespace which` prefix passed to a channel transform. Share ensemble selection, mapping and prefix invocation. | A constant subcommand string cannot bypass map/unknown/mutation effects. The reduced fixed map requires Tcl 8.5+; actual channel-transform host semantics were not executed. |
| Finite factories: `corpus-finite-factory` | Tcllib statistics generates eight named procs from a literal list. Bounded evaluation can enumerate definitions, interpolate the loop value into each body and retain the original source spans. | A braced factory body using `$name` reads the eventual procedure's local, not a captured loop value. Finite name enumeration alone does not prove generated-body semantics or editability. |
| Open factories and frameworks: `corpus-open-factory`, `corpus-framework-definer`, `corpus-snit-definition` | Tcllib polynomials generates counter-derived commands; clay creates a namespace/definer dialect from runtime `oo::define` enumeration; tklib diagrams defines multiword Snit methods, aliases and computed procedures. Use framework grammar and shared summaries, with symbolic origins or bounded candidate sets. | Counter-derived families are not finite literal alternatives. Runtime command enumeration requires a complete binding environment. Caller namespace capture and generated bodies must be modelled; framework keyword guesses cannot grant definite targets. |
| Child-interpreter prefixes: `corpus-snit-alias` | Tcllib obtains a caller's `mymethod` prefix, prepends `interp alias` with `linsert`, then evaluates the list. Preserve child interpreter identity plus every prefix element. | A whitespace-containing command name is one list element. String concatenation before `eval` splits it and names a different command. The portable reduction covers quoting; the source's Snit and child-interpreter contracts remain additional requirements. |
| F5 rule calls: `corpus-f5-partition-call`, `corpus-f5-sideband-call` | Independent JSON and sideband sources pass `/partition/rule::procedure` to F5 `call`. Keep the BIG-IP object path and rule-local procedure component in a dialect-owned representation. | Ordinary Tcl namespace splitting cannot identify the target rule. Partition/profile attachment, compilation and runtime availability are appliance facts. Real BIG-IP evidence remains pending. |
| Data near miss: `corpus-keyword-data` | argparse's syntax database assigns metadata with a key containing `namespace code`. | It is a `set` operation, not an executed namespace capture. Lexical keyword search alone must not create a reference, body or command effect. |

The source IDs above resolve to exact pinned upstream links in
[sites.json](sites.json). The initial issue's corresponding source is
`tclopt/test/test.test:188–209`; this is the named Mpfit test reported in
[#2286](https://github.com/bitwisecook/tcl-lsp/issues/2286), checked at the
corpus's recorded revision rather than an assumed current upstream HEAD.

## Real C Tcl reductions

[run_corpus_probes.py](run_corpus_probes.py) captures each authored probe in a
fresh matching C interpreter. Each probe's final line is the observed lookup;
preceding lines are setup. Raw scripts, stdout, stderr, completion codes,
return options where available, binary/source hashes and individual records
live under [results](results/provenance.json). Inputs/results are ASCII; the
capture format does not claim lossless arbitrary Unicode/binary coverage.

```sh
python3 experiments/name-handling/patterns/run_corpus_probes.py
```

The pins are read from the existing shared reference-toolchain owner. Exact
measured patchlevels and binary identities are in the result provenance.
Fifteen probes across C Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 yielded 75 records.
Every record has a captured observation and all processes exited zero; that
means capture succeeded, not that all scripts succeeded. Five setup phases
report unavailable capabilities, and ten lookup phases intentionally expose
near-miss errors.

| Probe | Observed result / completion | Releases |
| --- | --- | --- |
| `computed-qualified` | `first` | All five |
| `computed-binding-mutation` | `replacement` after replacing the command behind unchanged head bytes | All five |
| `list-prefix` | `{a b} {c d}`; whitespace-containing target remains one word | All five |
| `string-prefix-near-miss` | Lookup error: `invalid command name "::nh::two"` | All five |
| `namespace-captured-prefix` | `local {a b} {{c d}}`, despite a global `emit` | All five |
| `alias-prefix-lifetime` | `{::nh::emit {a b}} {{a b} {c d}}`; original alias token describes renamed alias | All five |
| `frame-wrapper` | `changed` | All five |
| `frame-wrapper-near-miss` | `original` | All five |
| `finite-factory` | `red blue` | All five |
| `factory-body-near-miss` | Lookup error: `can't read "name": no such variable` | All five |
| `keyed-variable-link` | `changed` | All five |
| `ensemble-prefix-map` | `{a b} {c d}` | 8.5–9.1; 8.4 setup rejects `namespace ensemble` |
| `oo-self-prefix` | `ok {a b}` | 8.6–9.1; 8.4/8.5 setup lacks `oo::class` |
| `oo-prefix-deleted-near-miss` | `1`: captured prefix raises after object destruction | 8.6–9.1; 8.4/8.5 setup lacks `oo::class` |
| `tcltest-shared-lifecycle` | `{{a b} {a b}} 0`; body and cleanup read setup value, then unset it | All five; retained Tcltest counters show one passed test and no failures |

These observations establish the reduced Tcl semantics. They do not measure
current LSP/compiler/runtime consumer conformance. Random object-command IDs
in setup results are retained raw and are unsuitable as expected values for
regressions. This C capture contains no Jim or BIG-IP answer. The
[maintained catalogue](README.md) links the separately retained
[latest Jim observations](jim-results-5bac7c9/README.md), including original
setup failures and distinct Jim-native controls. Exact-case BIG-IP answers
remain pending in the [appliance bundle](f5/README.md).

## Shared-owner acceptance and consumer tests

Promote measured reductions into deterministic shared conformance fixtures,
keeping positive and adversarial variants together. A static recogniser needs
a separate acceptance test showing which evidence it can prove from the
source and environment.

| Shared owner / fact | Consumer tests that should share one answer |
| --- | --- |
| Syntax/name representation, list-word owner and value provenance | Known `${ns}::name` and list prefixes resolve with original word spans; concatenated near miss declines or reports the actual different head; names containing spaces, empty names and colon edges preserve bytes and structured identity. |
| Registry resolved invocation, command binding realm and lifecycle | Alias fixed-prefix arguments, imported Tcltest definer and ensemble mappings agree in hover/tokens, body analysis, signatures, diagnostics, optimisation and codegen. Mutation changes or invalidates the same binding for each consumer. |
| Frame effects, variable-place/alias owner and ordered body effects | `uplevel` versus `eval` wrapper pair yields distinct target cells; Tcltest setup/body/cleanup defs and uses share the proved frame; computed Tk state stays conditional when widget identity is unknown. |
| Framework definition grammar, OO receiver provenance and callback roles | `self` list and captured-`my` callbacks navigate to the same proved method family while preserving distinct invocation mechanism; destroyed receivers abstain/fail at runtime; Snit multiword names use grammar facts. |
| Source/load-order and bounded definition summaries | Literal statistics factory yields a finite definition family; polynomial counters and dynamic clay command enumeration remain symbolic/conditional unless complete facts permit a bound. Cross-file imports require load-order evidence. |
| F5 environment/rule lookup policy | F5 `call` target is shared by LSP, compiler, tooling and AI surfaces only when the rule/partition/profile context and returned appliance evidence establish it. |

For each accepted pattern, assert the evidence category: proved,
conditional with listed assumptions, bounded candidates, or unknown with a
reason. Completion may present candidates; definition/references cannot invent
a definite target, and rename/optimisation additionally need completeness,
editable provenance and mutation/effect proofs. None of these fixtures
authorises a private name heuristic in an individual provider.

The tklib diagrams factory and independent F5 sideband source are useful
held-out source families for recogniser evaluation. They are reviewed source
checks here, not a measured held-out static test. Keep generated copies and
close derivatives in the same group. Report eligible-site denominators,
wrong targets and missed provable targets only after a shared resolver
evaluation actually runs.

## Appliance questions

[The corpus F5 supplement](corpus-f5-questions.md) records concrete questions
from the two partition-qualified call sources. Follow
[the existing request/answer protocol](../f5/REQUEST.md), retain accepted and
rejected input separately from runtime execution, and return raw, versioned
answers into the branch. Neither C Tcl reductions nor TesTcl mocks replace
those answers.
