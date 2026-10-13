# Tcl naming patterns found in the wild

This catalogue joins reviewed public source contexts to retained experiments,
original experiment source and answers from C Tcl 8.4–9.1, latest Jim and
BIG-IP/F5. A source example motivates a question; only a reached experiment
supplies an oracle answer. Each table keeps narrower reductions and unanswered
application-specific questions explicit.

The [canonical proof index](../../../docs/design/analysis/name-resolution-proofs/README.md)
is the authority for the 44 existing questions: 15 `naming.corpus-pattern.*`
and 29 `naming.wiki.*`. Thirty Wiki cases include mutually exclusive legacy and
modern controls for one question. This catalogue navigates those records and
captures; it does not issue proofs or grant execution, edit or optimisation
authority. The [consumer integration map](../../../docs/design/analysis/name-resolution-proofs/consumer-integrations.md)
records implementation coverage independently of native semantics.

The supplemental [composed callback-prefix pair](composed-callback-prefix/README.md)
retains original aliased-builder/captured-`my`/stored-callback inputs, C5 and pinned
Jim outcomes, failed driver attempts, a separate consumer plan and unanswered F5
contexts; it changes no existing canonical question or answer.

The separate [frame-capture prefix pair](frame-capture-prefix/README.md) retains
C5 observations of an aliased caller-relative link with/without namespace
capture, original Jim setup refusals and unanswered native F5 contexts.

## Source evidence

The [maintained corpus manifest](../../../scripts/perf/MANIFEST.toml) pins all
35 public checkouts in scope `everything`.
[Corpus provenance](corpus-provenance.json), [fetch](corpus-fetch.txt) and
[verification](corpus-verify.txt) retain exact revisions and manifest identity.
Manual review selected 19 contexts across 12 repositories: 18 naming contexts
and one metadata near miss. The fetcher's 2,649 selected files do not include
all source suffixes: the reviewed `.test` and `.irul` inputs are additional
formats. These are selected contexts, with no measured prevalence or static
recogniser accuracy. Generated copies and close derivatives share one family.

The zero-based index below selects the corresponding object in
[sites.json](sites.json)'s `sites` array. That object retains the full revision,
file SHA-256, selected-range SHA-256, physical source lines and licence evidence.
Upstream links are pinned to those revisions. A named reduction measures only
the stated core operation, even when the source requires a broader framework.

| Source ID / hash index | Pinned source context | Measured reduction and remaining context |
| --- | --- | --- |
| `corpus-tcltest` / 0 | [tclopt/test/test.test:188–209](https://github.com/georgtree/tclopt/blob/ff9a56074272327f3d5f7a3d589e50d9c2da24a0/test/test.test#L188) — Imported test executes setup/body/cleanup with variable continuity; optimiser handle is used as command head. | `tcltest-shared-lifecycle`: phase variable continuity; optimiser handle/type and full application unmeasured. |
| `corpus-current-alias` / 1 | [ruff/src/diagram.tcl:75–101](https://github.com/georgtree/ruff/blob/b63da69eed625bc544d1f4da76371e8bd6b02d29/src/diagram.tcl#L75) — Alternative initialisation branches install qualified alias from namespace current; external executable/packages select branch. | `alias-prefix-lifetime`: prefix and alias-token lifetime; CLI/HTTP branch selection unmeasured. |
| `corpus-alias-prefix` / 2 | [ruff/src/doctools_formatter.tcl:510–520](https://github.com/georgtree/ruff/blob/b63da69eed625bc544d1f4da76371e8bd6b02d29/src/doctools_formatter.tcl#L510) — Literal aliases share implementation but retain distinct fixed prefix arguments. | `alias-prefix-lifetime`: fixed argument elements; formatter application unmeasured. |
| `corpus-state-upvar` / 3 | [ruff/src/ruff.tcl:1137–1150](https://github.com/georgtree/ruff/blob/b63da69eed625bc544d1f4da76371e8bd6b02d29/src/ruff.tcl#L1137) — Caller-supplied name parameter aliases caller array through upvar 1. | `frame-wrapper` / near miss: composed frame reach; source array wrapper needs its own accepted summary. |
| `corpus-self-callback` / 4 | [pave/apavebase.tcl:1552–1566](https://github.com/aplsimple/pave/blob/875de1f1539d62e46e48e206f34f82848bca1736/apavebase.tcl#L1552) — TclOO self fills command-prefix receiver; Tk bind, after, execution trace have separate invocation contexts. | `oo-self-prefix` / deleted near miss: receiver and lifetime; Tk bind/after/trace argument injection unmeasured. |
| `corpus-frame-forward` / 5 | [pave/apavebase.tcl:3673–3678](https://github.com/aplsimple/pave/blob/875de1f1539d62e46e48e206f34f82848bca1736/apavebase.tcl#L3673) — Method forwards through uplevel 1 with list-built self receiver and expansion. | `frame-wrapper` / near miss: uplevel/eval distinction; OO forwarding context is additional. |
| `corpus-keyed-tk-state` / 6 | [tk/library/xmfbox.tcl:144–165](https://github.com/tcltk/tk/blob/7005f92520a03c7bec78ea521f96258d3365e4aa/library/xmfbox.tcl#L144) — Widget-derived qualified state array and caller-configured typevariable name; requires Tk widget identity. | `keyed-variable-link`: known key containing spaces; widget identity and `-typevariable` host behaviour unmeasured. |
| `corpus-eda-ensemble` / 7 | [OSVVM-Scripts/tee.tcl:22–36](https://github.com/OSVVM/OSVVM-Scripts/blob/60a2751c1d2087e6e58671585d3649cd5eb0b93d/tee.tcl#L22) — Ensemble unknown returns a prefix; nested ensemble has fd parameters and namespace-which prefix for channel transform. | `ensemble-prefix-map`: fixed map only; unknown handler, parameters and channel transform unmeasured. |
| `corpus-finite-factory` / 8 | [tcllib/modules/math/statistics.tcl:101–108](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/math/statistics.tcl#L101) — Eight literal procedure names generated by finite foreach; body interpolates type now and values later. | `finite-factory` / body near miss: literal generated names and substitution stage; actual eight-function family unexecuted. |
| `corpus-open-factory` / 9 | [tcllib/modules/math/polynomials.tcl:58–67](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/math/polynomials.tcl#L58) — Runtime counter creates unbounded command family with generated expression body; not finite literal enumeration. | Source-only open counter family; `finite-factory` supplies a contrasting finite control, not an answer to this factory. |
| `corpus-framework-definer` / 10 | [tcllib/modules/clay/build/dialect.tcl:67–90](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/clay/build/dialect.tcl#L67) — Caller namespace capture, dynamic namespace creation, aliases from runtime oo::define enumeration and prefixed definer commands. | Source-only dynamic Clay definer; `computed-qualified` covers constant construction only, not runtime enumeration/grammar. |
| `corpus-snit-alias` / 11 | [tcllib/modules/interp/interp.tcl:66–81](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/interp/interp.tcl#L66) — Caller-frame mymethod resolves prefix; linsert constructs child-interpreter alias and eval executes list; preserve interpreter identity. | [Real Snit composition](corpus-snit-alias-original/README.md): fixed supported names with captured/tail argument spaces pass C8.5–9.1; changed missing-method prefix errors. C8.4/Jim refuse real package setup. Authored caller over unchanged original helpers; earlier spaced-name refusals retained; native F5 pending. |
| `corpus-captured-my` / 12 | [tcllib/modules/httpd/build/server.tcl:361–385](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/httpd/build/server.tcl#L361) — socket server callback and cron task capture object namespace for my; plugin and generated amalgamation copies deduplicated. | `namespace-captured-prefix` covers namespace capture; `wiki-oo-1..2` cover private `my` and destruction. Socket/cron host context unmeasured. |
| `corpus-computed-head` / 13 | [ticklecharts/dataset.tcl:25–39](https://github.com/nico-robert/ticklecharts/blob/b49f014cb6c2412a7e661e805a37eaebcd625c59/dataset.tcl#L25) — self class -> namespace qualifiers -> ns interpolation drives setdef; class identity needed, not just bareword recognition. | `computed-qualified` / mutation: known bytes and rebinding; `self class` provenance in this project unmeasured. |
| `corpus-snit-definition` / 14 | [tklib/modules/diagrams/core.tcl:37–68](https://github.com/tcltk/tklib/blob/627bfe1e8dcc17b718dfafba26478c3e08501b2f/modules/diagrams/core.tcl#L37) — snit type declares multiword methods, command-prefix aliases, and computed namespace procedure factory; needs grammar/summary. | Source-only multiword Snit grammar and factory; finite core control does not execute Snit or prove editability. |
| `corpus-f5-partition-call` / 15 | [f5-irules-json/test-json.irule:16–23](https://github.com/JuergenMang/f5-irules-json/blob/a8d9e0ffa7f4227f8696523f1551e304025b467a/test-json.irule#L16) — call argument contains BIG-IP partition/object path and procedure suffix; not ordinary namespace FQN. | F5-only `corpus-f5-*` bounded native controls and remaining variants · [review](f5/answers/native-appliance-question-review.md). |
| `corpus-f5-sideband-call` / 16 | [simple-sideband/simple_sideband.irul:177–184](https://github.com/pwhitef5/simple-sideband/blob/95b13e0e323919330cabe1dcbba7bb966173b9cd/simple_sideband.irul#L177) — Same partition-qualified call idiom in independent source; .irul not selected by benchmark source suffix count. | Independent F5 source, same bounded native questions · [review](f5/answers/native-appliance-question-review.md). |
| `corpus-mock-frame` / 17 | [TesTcl/src/onirule.tcl:59–80](https://github.com/landro/TesTcl/blob/e31743809b976ba39da39138aeebf36f8d264fd9/src/onirule.tcl#L59) — Mock when injects variables through uplevel 0 and catches dynamic event body; Tcl harness is not evidence of TMM runtime semantics. | `frame-wrapper` / near miss: frame distinction; TesTcl mock does not execute TMM. |
| `corpus-keyword-data` / 18 | [argparse/test/syntaxdb.tcl:860–865](https://github.com/georgtree/argparse/blob/139d695c9012ebd79e21798b5c8041fef64e8358/test/syntaxdb.tcl#L860) — set stores namespace-code text as array metadata; name operation is not executed here. | Source-only negative control: metadata key contains `namespace code`; no executed capture at this site. |

Seventeen existing Tcl Wiki pages were read. One additional retrieval,
`forward`, returned HTTP 200 with **Page not found** and supplies no example.
[wiki-sources.json](wiki-sources.json) retains page HTML SHA-256, retrieval date,
displayed Updated date and every decoded `<pre>` SHA-256. The first number
below is the zero-based page object; `pre` numbers select its `examples[].index`.
Displayed dates are not verified revisions. A prose-only section has the page
hash as its evidence anchor and no invented example hash.

| Page / hash index | Selected source context | Retained case IDs / unanswered context |
| --- | --- | --- |
| [oo::define](https://wiki.tcl-lang.org/page/oo%3A%3Adefine) / 0 | pre 0–1 | `wiki-oo-3`, `wiki-oo-6..7`: forwards and export reset. |
| [command prefix](https://wiki.tcl-lang.org/page/command+prefix) / 1 | pre 2, 4–7, 17–18 | `wiki-prefix-1..3`: canonical words, expansion/script distinction, shadowed builder. |
| [namespace code](https://wiki.tcl-lang.org/page/namespace+code) / 2 | pre 1–5 | `wiki-code-1..7`: capture, lifetime, replacement and release-specific nested repair. |
| [namespace inscope](https://wiki.tcl-lang.org/page/namespace+inscope) / 3 | Description (prose) | `wiki-code-1`, `wiki-code-4..5`: live namespace and recreation. |
| [namespace eval](https://wiki.tcl-lang.org/page/namespace+eval) / 4 | pre 2–4 | `wiki-code-2`: namespace variable versus expired local; broad eval transformations unmeasured. |
| [Namespace MegaWidgets](https://wiki.tcl-lang.org/page/Namespace+MegaWidgets) / 5 | pre 0–1 | `wiki-factory-1..2`: finite factory reduction; Tk lifecycle and ordered class dispatch unmeasured. |
| [proc](https://wiki.tcl-lang.org/page/proc) / 6 | Description (prose) | `wiki-factory-1..2`: definition/substitution stages; arbitrary generated templates unmeasured. |
| [interp alias](https://wiki.tcl-lang.org/page/interp+alias) / 7 | pre 0–1, 13, 28–30; Synopsis | `wiki-alias-1..2`: target slot/interpreter versus imported identity. |
| [rename](https://wiki.tcl-lang.org/page/rename) / 8 | pre 3–4 | `wiki-alias-1`, `wiki-rename-1`: rebinding/import identity and proc namespace. |
| [unknown](https://wiki.tcl-lang.org/page/unknown) / 9 | pre 0–1, 3, 5 | `wiki-unknown-1`, `wiki-code-6..7`: custom fallback and nested-prefix release controls. |
| [auto_load](https://wiki.tcl-lang.org/page/auto_load) / 10 | Description (prose) | `wiki-autoload-1`: known absolute index key; external package/load-order graph unmeasured. |
| [upvar](https://wiki.tcl-lang.org/page/upvar) / 11 | pre 1, 10–15, 21–22 | `wiki-upvar-1..4`: explicit/implicit levels and wrapper frames. |
| [global](https://wiki.tcl-lang.org/page/global) / 12 | pre 6, 8 | `wiki-global-1`: declaration in caller; broader arrays/traces unmeasured. |
| [variable](https://wiki.tcl-lang.org/page/variable) / 13 | pre 3–5, 13–17 | `wiki-variable-1`: core assignment grammar; TclOO declaration grammar is distinct. |
| [TclOO](https://wiki.tcl-lang.org/page/TclOO) / 14 | pre 1, 4–7 | `wiki-oo-1..7`: private methods, callbacks, factories and mutations. |
| [metaclass](https://wiki.tcl-lang.org/page/metaclass) / 15 | pre 0; final paragraph | `wiki-oo-3..5`: forwards, method creation and non-object `new` result. |
| [Page not found](https://wiki.tcl-lang.org/page/forward) / 16 | No example | Unanswered URL; forwarding evidence comes from `oo::define` and `metaclass`. |
| [callback](https://wiki.tcl-lang.org/page/callback) / 17 | Name resolution / Customization tricks / Good practices (prose) | `wiki-code-*`, `wiki-oo-1..2`: namespace versus receiver mechanisms; arbitrary host sinks unmeasured. |

## Pattern semantics and proof bounds

These are reusable questions for the shared name, registry, binding, frame,
source and receiver owners. A recogniser should return proved facts,
conditional facts with assumptions, bounded candidates or unknown with a
reason. Popular idioms alone cannot establish a particular call target.

| Pattern | Positive and adversarial experiment | Required facts / invalidation boundary |
| --- | --- | --- |
| Computed qualified names | `computed-qualified` constructs a known head; `computed-binding-mutation` keeps bytes but replaces target. | Track value construction separately from call-time binding; class origin, writes and rename/redefinition can withdraw a target. |
| Canonical command prefixes | `list-prefix`, `wiki-prefix-1` preserve spaces/semicolons; `string-prefix-near-miss` splits the head; `wiki-prefix-2..3` distinguish script/list execution and builder shadowing. | Prove builder binding, list-word provenance, exact fixed/appended arguments and sink context. Constant-looking `list` spelling is insufficient. |
| Namespace capture | `namespace-captured-prefix`, `wiki-code-1` select local commands; `wiki-code-2..5` distinguish local lifetime, replacement, deletion and recreation; `wiki-code-6..7` release-gate fallback. | Capture namespace context, not a proc-local closure or durable command identity. Entered frame and namespace lifetime remain separate. |
| Alias, import and rename | `alias-prefix-lifetime`, `wiki-alias-1..2`, `wiki-rename-1` distinguish allocation token, current dispatch spelling, imported identity and target interpreter. | Retain interpreter and command identity, prefix lineage and creation order; retarget, replacement, forget, rename and interpreter deletion invalidate different facts. |
| Variable/frame wrappers | `frame-wrapper` versus near miss, `wiki-upvar-1..4`, `wiki-global-1`, `wiki-variable-1` distinguish frames, declaration grammars and release parsing. | Prove composed frame reach and variable-name provenance; unknown levels, reentrant traces and wrapper mutation require abstention. |
| Deferred trace producers and access spelling | [Real Tcllib lazyset](lazyset-reader-context/README.md), with pinned helper and Wiki trace provenance: moving the read selects a shadowing reader producer; qualifying the prefix restores its target. Original package C8.5–9.1 and separate core C8.4–9.1 controls reached; C8.4/Jim package setup and Jim trace refuse. | Require the triggering reader frame/namespace, actual implementation and prefix binding. Registration namespace and installation spelling do not establish the later producer or trace access name. F5 remains unanswered. |
| Keyed state arrays | `keyed-variable-link` links a known spaced key. | Keep namespace array identity, element key and cell link separate. Widget-derived identity remains conditional on actual Tk facts. |
| Ensemble dispatch | `ensemble-prefix-map` preserves fixed-prefix words. | Fixed map coverage does not answer unknown-handler/parameter/channel-transform behaviour; use the actual selected mapping and mutation facts. |
| Finite definition factories | `finite-factory`, `wiki-factory-1` enumerate literal names; factory near misses do not capture a loop local. | Prove finite input set, substitution stages and source origin. Finite identity does not guarantee editable geometry. Counter families and arbitrary templates remain symbolic. |
| Unknown/autoload | `wiki-autoload-1`, `wiki-unknown-1` make initially absent heads executable. | Prove actual handler/index/loader effects and load order. Absence from one command table cannot establish an invalid invocation. |
| OO receivers, forwards, metaclasses | `oo-self-prefix`, deleted near miss and `wiki-oo-1..7` exercise public/internal dispatch, destruction, target replacement, dynamic methods, export reset and overridden `new`. | Prove receiver/class/metaclass, export and selected method facts; registration does not prove later dispatch. Stock-looking `new` does not guarantee an object result. |
| Framework-generated definitions | Reviewed Clay, Snit and open factory contexts are source-only beyond core reductions. | Require framework grammar, summaries and complete relevant bindings; keyword guesses cannot issue definitive targets. |
| F5 rule procedure calls | Two independent partition-qualified `call` sources motivate `corpus-f5-*` questions. | Appliance object path, rule-local procedure, attachment, partition, TMM/frame/storage lifetime and load versus runtime acceptance need actual BIG-IP answers. |
| Keyword data near miss | Reviewed argparse metadata contains namespace syntax as data. | Establish executed argument roles before recognising a naming operation; lexical matches alone create no reference or body. |

## Exact portable corpus observations

[Probe source](probes), [capture runner](run_corpus_probes.py),
[75 original record joins](results/records.json) and
[capture provenance](results/provenance.json) retain full input, stdout/stderr,
setup/lookup completion, return options where supported and actual binary hashes.
Each row links its exact source and canonical question. Jim entries link their
separate original capture/channel; details are below. C columns report lookup
completion/result, not process exit: all capture processes exited zero.
`setup refusal` means the intended lookup was not reached. Expected guest errors
are actual observations, not missing answers.

| Experiment / canonical question | 8.4.20 | 8.5.19 | 8.6.18 | 9.0.4 | 9.1.0 | Latest Jim | BIG-IP/F5 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [alias-prefix-lifetime](probes/alias-prefix-lifetime.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-alias-prefix-lifetime.md) | `0: {::nh::emit {a b}} {{a b} {c d}}` | `0: {::nh::emit {a b}} {{a b} {c d}}` | `0: {::nh::emit {a b}} {{a b} {c d}}` | `0: {::nh::emit {a b}} {{a b} {c d}}` | `0: {::nh::emit {a b}} {{a b} {c d}}` | setup refusal: `wrong # args: should be "interp"`; lookup unreached · [capture](jim-results-5bac7c9/corpus/alias-prefix-lifetime/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [computed-binding-mutation](probes/computed-binding-mutation.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-computed-binding-mutation.md) | `0: replacement` | `0: replacement` | `0: replacement` | `0: replacement` | `0: replacement` | `0: replacement` · [capture](jim-results-5bac7c9/corpus/computed-binding-mutation/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [computed-qualified](probes/computed-qualified.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-computed-qualified.md) | `0: first` | `0: first` | `0: first` | `0: first` | `0: first` | `0: first` · [capture](jim-results-5bac7c9/corpus/computed-qualified/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [ensemble-prefix-map](probes/ensemble-prefix-map.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-ensemble-prefix-map.md) | setup refusal (`namespace ensemble` absent) | `0: {a b} {c d}` | `0: {a b} {c d}` | `0: {a b} {c d}` | `0: {a b} {c d}` | setup refusal: `wrong # args: should be "ensemble command ?-automap prefix?"`; lookup unreached · [capture](jim-results-5bac7c9/corpus/ensemble-prefix-map/record.json) | original stock 8.5 observed · [review](f5/answers/stock-original-source-review.md) |
| [factory-body-near-miss](probes/factory-body-near-miss.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-factory-body-near-miss.md) | `1: can't read "name": no such variable` | `1: can't read "name": no such variable` | `1: can't read "name": no such variable` | `1: can't read "name": no such variable` | `1: can't read "name": no such variable` | `1: can't read "name": no such variable` · [capture](jim-results-5bac7c9/corpus/factory-body-near-miss/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [finite-factory](probes/finite-factory.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-finite-factory.md) | `0: red blue` | `0: red blue` | `0: red blue` | `0: red blue` | `0: red blue` | `0: red blue` · [capture](jim-results-5bac7c9/corpus/finite-factory/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [frame-wrapper](probes/frame-wrapper.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-frame-wrapper.md) | `0: changed` | `0: changed` | `0: changed` | `0: changed` | `0: changed` | `0: changed` · [capture](jim-results-5bac7c9/corpus/frame-wrapper/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [frame-wrapper-near-miss](probes/frame-wrapper-near-miss.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-frame-wrapper-near-miss.md) | `0: original` | `0: original` | `0: original` | `0: original` | `0: original` | `0: original` · [capture](jim-results-5bac7c9/corpus/frame-wrapper-near-miss/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [keyed-variable-link](probes/keyed-variable-link.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-keyed-variable-link.md) | `0: changed` | `0: changed` | `0: changed` | `0: changed` | `0: changed` | `0: changed` · [capture](jim-results-5bac7c9/corpus/keyed-variable-link/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [list-prefix](probes/list-prefix.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-list-prefix.md) | `0: {a b} {c d}` | `0: {a b} {c d}` | `0: {a b} {c d}` | `0: {a b} {c d}` | `0: {a b} {c d}` | `0: {a b} {c d}` · [capture](jim-results-5bac7c9/corpus/list-prefix/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [namespace-captured-prefix](probes/namespace-captured-prefix.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-namespace-captured-prefix.md) | `0: local {a b} {{c d}}` | `0: local {a b} {{c d}}` | `0: local {a b} {{c d}}` | `0: local {a b} {{c d}}` | `0: local {a b} {{c d}}` | `1: can't read "::nh::prefix": no such variable` · [capture](jim-results-5bac7c9/corpus/namespace-captured-prefix/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [oo-prefix-deleted-near-miss](probes/oo-prefix-deleted-near-miss.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-oo-prefix-deleted-near-miss.md) | setup refusal (`oo::class` absent) | setup refusal (`oo::class` absent) | `0: 1` | `0: 1` | `0: 1` | setup refusal: `invalid command name "oo::class"`; lookup unreached · [capture](jim-results-5bac7c9/corpus/oo-prefix-deleted-near-miss/record.json) | whole-program error measured; exact lookup pending · [capture](f5/answers/nh20261012stock2/ANSWERS.md) |
| [oo-self-prefix](probes/oo-self-prefix.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-oo-self-prefix.md) | setup refusal (`oo::class` absent) | setup refusal (`oo::class` absent) | `0: ok {a b}` | `0: ok {a b}` | `0: ok {a b}` | setup refusal: `invalid command name "oo::class"`; lookup unreached · [capture](jim-results-5bac7c9/corpus/oo-self-prefix/record.json) | whole-program error measured; exact lookup pending · [capture](f5/answers/nh20261012stock2/ANSWERS.md) |
| [string-prefix-near-miss](probes/string-prefix-near-miss.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-string-prefix-near-miss.md) | `1: invalid command name "::nh::two"` | `1: invalid command name "::nh::two"` | `1: invalid command name "::nh::two"` | `1: invalid command name "::nh::two"` | `1: invalid command name "::nh::two"` | `1: invalid command name "::nh::two"` · [capture](jim-results-5bac7c9/corpus/string-prefix-near-miss/record.json) | original hosted final operation observed · [review](f5/answers/hosted-original-source-review.md) |
| [tcltest-shared-lifecycle](probes/tcltest-shared-lifecycle.tcl) · [proof](../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-tcltest-shared-lifecycle.md) | `0: {{a b} {a b}} 0` | `0: {{a b} {a b}} 0` | `0: {{a b} {a b}} 0` | `0: {{a b} {a b}} 0` | `0: {{a b} {a b}} 0` | `1: can't read "::nh::observed": no such variable` · [capture](jim-results-5bac7c9/corpus/tcltest-shared-lifecycle/record.json) | original stock 8.4/8.5 observed · [review](f5/answers/stock-original-source-review.md) |

## Exact Wiki oracle observations

[Original Wiki probe source](wiki-probes.tcl) and
[runner](run_wiki_probes.py) execute fixed cases using the matching real upstream
`init.tcl` and `tcltest.tcl`. Each child interpreter starts normally.
[Provenance](wiki-results/provenance.json) records library bytes, binary identity
and counts; [canonical case joins](../../../rust/tcl-registry/tests/data/native_wiki_naming_patterns/observations.json)
identify exact raw PASS/SKIP lines. The result column is the assertion proved by
PASS, not an invented trace of intermediate execution. `skip` supplies no answer.
Jim entries are completions of the separately captured inner script; they are
not Tcltest PASS/SKIP outcomes.

| C provider | Cases passed | Skipped | Failed |
| --- | ---: | ---: | ---: |
| 8.4.20 | 21 | 9 | 0 |
| 8.5.19 | 22 | 8 | 0 |
| 8.6.18 | 29 | 1 | 0 |
| 9.0.4 | 29 | 1 | 0 |
| 9.1.0 | 29 | 1 | 0 |

| Case / canonical question | Proved result expression | 8.4.20 | 8.5.19 | 8.6.18 | 9.0.4 | 9.1.0 | Latest Jim | BIG-IP/F5 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [wiki-prefix-1](../../../docs/design/analysis/name-resolution-proofs/wiki-prefix-1.md) | `{{x; error injected} {a b}}` | PASS | PASS | PASS | PASS | PASS | `0: {x; error injected} {a b}` · [capture](jim-results-5bac7c9/wiki/wiki-prefix-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-prefix-2](../../../docs/design/analysis/name-resolution-proofs/wiki-prefix-2.md) | `{extra script}` | skip | PASS | PASS | PASS | PASS | `0: extra script` · [capture](jim-results-5bac7c9/wiki/wiki-prefix-2/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-prefix-3](../../../docs/design/analysis/name-resolution-proofs/wiki-prefix-3.md) | `actual` | PASS | PASS | PASS | PASS | PASS | `0: actual` · [capture](jim-results-5bac7c9/wiki/wiki-prefix-3/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-code-1](../../../docs/design/analysis/name-resolution-proofs/wiki-code-1.md) | `{local fixed {a b}}` | PASS | PASS | PASS | PASS | PASS | `0: local fixed {a b}` · [capture](jim-results-5bac7c9/wiki/wiki-code-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-code-2](../../../docs/design/analysis/name-resolution-proofs/wiki-code-2.md) | `namespace` | PASS | PASS | PASS | PASS | PASS | `1: can't read "x": no such variable` · [capture](jim-results-5bac7c9/wiki/wiki-code-2/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-code-3](../../../docs/design/analysis/name-resolution-proofs/wiki-code-3.md) | `replacement` | PASS | PASS | PASS | PASS | PASS | `0: replacement` · [capture](jim-results-5bac7c9/wiki/wiki-code-3/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-code-4](../../../docs/design/analysis/name-resolution-proofs/wiki-code-4.md) | `1` | PASS | PASS | PASS | PASS | PASS | `0: 1` · [capture](jim-results-5bac7c9/wiki/wiki-code-4/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-code-5](../../../docs/design/analysis/name-resolution-proofs/wiki-code-5.md) | `recreated` | PASS | PASS | PASS | PASS | PASS | `1: can't read "x": no such variable` · [capture](jim-results-5bac7c9/wiki/wiki-code-5/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-code-6](../../../docs/design/analysis/name-resolution-proofs/wiki-code-6.md) | `fixed` | PASS | PASS | skip | skip | skip | `1: invalid command name "::namespace inscope ::n take"` · [capture](jim-results-5bac7c9/wiki/wiki-code-6/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-code-7](../../../docs/design/analysis/name-resolution-proofs/wiki-code-6.md) | `1` | skip | skip | PASS | PASS | PASS | `0: 1` · [capture](jim-results-5bac7c9/wiki/wiki-code-7/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |
| [wiki-factory-1](../../../docs/design/analysis/name-resolution-proofs/wiki-factory-1.md) | `{alpha beta}` | PASS | PASS | PASS | PASS | PASS | `0: alpha beta` · [capture](jim-results-5bac7c9/wiki/wiki-factory-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-factory-2](../../../docs/design/analysis/name-resolution-proofs/wiki-factory-2.md) | `1` | PASS | PASS | PASS | PASS | PASS | `0: 1` · [capture](jim-results-5bac7c9/wiki/wiki-factory-2/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-alias-1](../../../docs/design/analysis/name-resolution-proofs/wiki-alias-1.md) | `{{replacement fixed} {old fixed}}` | PASS | PASS | PASS | PASS | PASS | capability rejection: `wrong # args: should be "interp"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-alias-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-alias-2](../../../docs/design/analysis/name-resolution-proofs/wiki-alias-2.md) | `helper` | PASS | PASS | PASS | PASS | PASS | capability rejection: `wrong # args: should be "interp"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-alias-2/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-rename-1](../../../docs/design/analysis/name-resolution-proofs/wiki-rename-1.md) | `two` | PASS | PASS | PASS | PASS | PASS | `0: two` · [capture](jim-results-5bac7c9/wiki/wiki-rename-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-upvar-1](../../../docs/design/analysis/name-resolution-proofs/wiki-upvar-1.md) | `changed` | PASS | PASS | PASS | PASS | PASS | `0: changed` · [capture](jim-results-5bac7c9/wiki/wiki-upvar-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-upvar-2](../../../docs/design/analysis/name-resolution-proofs/wiki-upvar-2.md) | `1` on 8.4/8.5; `0` on 8.6–9.1 (release expression) | PASS | PASS | PASS | PASS | PASS | `0: 0` · [capture](jim-results-5bac7c9/wiki/wiki-upvar-2/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-upvar-3](../../../docs/design/analysis/name-resolution-proofs/wiki-upvar-3.md) | `changed` | PASS | PASS | PASS | PASS | PASS | `0: changed` · [capture](jim-results-5bac7c9/wiki/wiki-upvar-3/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-upvar-4](../../../docs/design/analysis/name-resolution-proofs/wiki-upvar-4.md) | `old` | PASS | PASS | PASS | PASS | PASS | `0: old` · [capture](jim-results-5bac7c9/wiki/wiki-upvar-4/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-variable-1](../../../docs/design/analysis/name-resolution-proofs/wiki-variable-1.md) | `{b 0}` | PASS | PASS | PASS | PASS | PASS | `0: b 0` · [capture](jim-results-5bac7c9/wiki/wiki-variable-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-global-1](../../../docs/design/analysis/name-resolution-proofs/wiki-global-1.md) | `global` | PASS | PASS | PASS | PASS | PASS | `0: global` · [capture](jim-results-5bac7c9/wiki/wiki-global-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-autoload-1](../../../docs/design/analysis/name-resolution-proofs/wiki-autoload-1.md) | `{{} loaded ::late}` | PASS | PASS | PASS | PASS | PASS | `1: invalid command name "::late"` · [capture](jim-results-5bac7c9/wiki/wiki-autoload-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-unknown-1](../../../docs/design/analysis/name-resolution-proofs/wiki-unknown-1.md) | `{{} {absent fixed}}` | PASS | PASS | PASS | PASS | PASS | `0: {} {absent fixed}` · [capture](jim-results-5bac7c9/wiki/wiki-unknown-1/record.json) | selected original stock body observed via real-harness trace · [review](f5/answers/stock-wiki-review.md) |
| [wiki-oo-1](../../../docs/design/analysis/name-resolution-proofs/wiki-oo-1.md) | `{1 fixed}` | skip | skip | PASS | PASS | PASS | capability rejection: `invalid command name "oo::class"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-oo-1/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |
| [wiki-oo-2](../../../docs/design/analysis/name-resolution-proofs/wiki-oo-2.md) | `1` | skip | skip | PASS | PASS | PASS | capability rejection: `invalid command name "oo::class"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-oo-2/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |
| [wiki-oo-3](../../../docs/design/analysis/name-resolution-proofs/wiki-oo-3.md) | `{replacement fixed}` | skip | skip | PASS | PASS | PASS | capability rejection: `invalid command name "oo::class"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-oo-3/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |
| [wiki-oo-4](../../../docs/design/analysis/name-resolution-proofs/wiki-oo-4.md) | `generated` | skip | skip | PASS | PASS | PASS | capability rejection: `invalid command name "oo::class"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-oo-4/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |
| [wiki-oo-5](../../../docs/design/analysis/name-resolution-proofs/wiki-oo-5.md) | `{ordinary 0}` | skip | skip | PASS | PASS | PASS | capability rejection: `invalid command name "oo::class"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-oo-5/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |
| [wiki-oo-6](../../../docs/design/analysis/name-resolution-proofs/wiki-oo-6.md) | `{1 internal}` | skip | skip | PASS | PASS | PASS | capability rejection: `invalid command name "oo::class"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-oo-6/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |
| [wiki-oo-7](../../../docs/design/analysis/name-resolution-proofs/wiki-oo-7.md) | `{old 1}` | skip | skip | PASS | PASS | PASS | capability rejection: `invalid command name "oo::class"`; target question unreached · [capture](jim-results-5bac7c9/wiki/wiki-oo-7/record.json) | real stock constraint skip; hosted setup unreached · [review](f5/answers/stock-wiki-review.md) |

The seven OO cases skip on stock 8.4/8.5 because TclOO is absent; the expansion
case skips on 8.4. Legacy and modern nested-prefix controls are mutually
exclusive. [Discovery captures](wiki-results/discovery/provenance.json) retain
initial hypotheses and corrections; final expectations use measured C results.

## Latest Jim: exact channels and separate controls

[Latest Jim captures](jim-results-5bac7c9/README.md),
[49 records](jim-results-5bac7c9/records.json) and
[provenance](jim-results-5bac7c9/provenance.json) identify upstream revision
`5bac7c99ad65864c87da513e22e2f01703fa4e03` (`0.84-9-g5bac7c9`), checked against
upstream HEAD on 2026-10-12 at 13:13 UTC. The driver sources the real upstream
`tests/testing.tcl` and `tcltest.tcl`; no C Tcl init library substitutes for Jim.

The corpus lane uses exact authored setup and final question: seven returned
values, four raised errors and four setup refusals with unreached lookup. Wiki
rows execute each exact inner `isolated` script in a fresh Jim process: seventeen
returned values, four raised errors and nine program capability rejections
(seven missing `oo::class`, two incompatible `interp` interfaces). They do not
execute the unchanged C Tcltest wrapper. The
[unchanged Wiki suite attempt](jim-results-5bac7c9/original-wiki-suite/record.json)
stops at missing `::tcltest::configure` before any case runs. It supplies one
harness refusal, not thirty test failures. C expected values retained in Jim
metadata are comparison aids, not asserted Jim conformance expectations.

Jim table entries above report actual program completion/result or unreached
lookup. A capability rejection cannot answer later alias/OO target selection.
The corpus lifecycle probe's final read of `::nh::observed` raises an error,
although its raw harness counters record one passed test and no failures. Its
plain namespace `set` and final qualified variable read are distinct operations;
the final error does not erase the narrower recorded phase execution.
Four separately authored Jim-native controls answer different inputs; they
cannot replace those original failures or infer C equivalence:

| Separate Jim-native source | Actual result | Capture / scope |
| --- | --- | --- |
| [alias-prefix-rebinding](jim-probes/alias-prefix-rebinding.tcl) | `0: {::nh::emit {a b}} {replacement {a b} {c d}}` | [record](jim-results-5bac7c9/jim-native/alias-prefix-rebinding/record.json); Jim alias API, current target replacement and preserved words. |
| [ensemble-native-dispatch](jim-probes/ensemble-native-dispatch.tcl) | `0: {old {a b} {c d}} {replacement {a b} {c d}}` | [record](jim-results-5bac7c9/jim-native/ensemble-native-dispatch/record.json); Jim ensemble API, not C namespace map syntax. |
| [namespace-explicit-variable-prefix](jim-probes/namespace-explicit-variable-prefix.tcl) | `0: local {a b} {{c d}}` | [record](jim-results-5bac7c9/jim-native/namespace-explicit-variable-prefix/record.json); explicit namespace `variable` avoids relying on ordinary `set` creation. |
| [test-shared-lifecycle](jim-probes/test-shared-lifecycle.tcl) | `0: {{a b} {a b}} 0 1 0` | [record](jim-results-5bac7c9/jim-native/test-shared-lifecycle/record.json); real Jim `test` records one pass, zero failures and shared lifecycle values. |

The 11 reached original corpus questions have registered Jim provider answers
in their existing canonical records: seven returned values and four guest
errors. Their exact unchanged setup/final-question wrappers, raw fields,
upstream revision, harness hashes and build configuration remain attached.
The four corpus setup failures supply no intended-question answer. Wiki inner
adaptations and the four separately authored Jim-native controls remain
supplemental captures with their own driver/source bytes and adaptation metadata.
Consult each canonical provider record before using an observation as registered
evidence; these answers establish no Rust consumer conformance.

## Other provider and source-specific questions

Future Jim answers must identify the exact upstream revision, binary/build,
startup/support library bytes and input. A Jim suite result from another input
cannot fill these rows. If Jim's native testing distribution needs a different
harness, retain that harness and exact extracted body and mark the execution
channel; do not label an adapted run as execution of the unchanged C Tcltest
program. Capability rejection during setup supplies no answer to an unreached
lookup. Differences from C are observations to investigate, not automatic Jim
conformance failures.

Reviewed original corpus and selected Wiki-body outcomes are linked from the
[stock corpus review](f5/answers/stock-original-source-review.md),
[stock Wiki review](f5/answers/stock-wiki-review.md), and
[hosted original-source review](f5/answers/hosted-original-source-review.md).
The [pattern-specific appliance bundle](f5/README.md) and
[current context status matrix](f5/questions.json) map 45 portable cases and
five appliance-specific questions across eight independent contexts. The matrix
separates accepted answers, measured attempts and unmeasured boundaries.
Stock interpreter observations establish no TMM/CLI/iApp/iCall/APL answer.
C/Jim desktop reductions are not ready-to-load iRules. [Appliance request](../f5/REQUEST.md),
[answer template](../f5/ANSWER-TEMPLATE.md) and
[corpus F5 supplement](corpus-f5-questions.md) specify separate TMM event,
`RULE_INIT`, standalone Tcl and CLI/iApp contexts. Retain exact accepted and
rejected source, administrative object/rule/partition context, version/hotfix,
load result, reached runtime result and cleanup. Load rejection cannot answer
runtime dispatch. Do not manufacture an appliance answer from TesTcl mocks.

Source-specific gaps remain: Clay/Snit grammar and dynamic enumeration; open
counter factories; Tk widget identities and callback argument rules; EDA
unknown/parameter/channel-transform dispatch; external loader order; application
OO receiver/class provenance; conditional CLI/HTTP alias selection; and the
actual F5 rule/storage model. Their nearest core controls above do not close
these questions. Add separate paired reductions and canonical questions when
these contexts are investigated.

The [VM/runtime capture boundaries](wiki-findings.md#vmruntime-capture-boundaries)
retain supplied-binary observations and rebuilt-engine startup refusals. They
are subject-engine measurements, not reference oracle answers. No current
static-consumer accuracy or whole-application conformance is established here.

## Maintaining this catalogue

For each new source pattern retain a pinned source coordinate, licence reference,
full-source/range hash and reviewed execution context before counting a site.
Keep derivatives grouped. State the observed idiom and the semantic question,
then add an authored positive reduction and adversarial variant with original
input bytes. Record context-specific gaps separately from the reduced question.

Execute each applicable provider on exact retained inputs. Preserve input and
runner bytes, startup/build/source receipts, raw stdout/stderr, completion/return
options and reached-stage markers. Rejected setup, skipped constraints, failed
captures and unavailable providers stay explicit. Record patchlevels and Jim
revisions; attach BIG-IP answers through the appliance protocol. New captures
use a fresh output directory; existing evidence is immutable.

Register new questions or separately retained answers through the existing
canonical proof owner and update its source/case joins. Update this catalogue's
source mapping, result/status tables and scope limits from those actual records.
Do not replace raw evidence with a summary or create a competing result manifest.
Run the complete proof check after registration:

```sh
python3 scripts/dev/check-name-resolution-proofs.py
```

Promote observed results into deterministic shared-owner conformance fixtures,
then test static recognisers separately for proof category, withheld context,
wrong targets and missed provable targets. Completion candidates, navigation,
rename and optimisation have different proof obligations; every surface must
consume the same shared facts. Keep those implementation results in the
canonical consumer integration map.

[Stock final-error review](f5/answers/stock-final-review.md) retains source-reviewed original near-error final operations from sealed stock2 captures without claiming a new execution.

[Current appliance coverage](f5/answers/nh20261012coverage2/answers.md) retains
400 original context states: 93 observed, 244 unreached and 63 unavailable.
Bounded variants and per-unit/invocation receipt counts remain separate.
The [portable report index](../f5/answers/README.md#portable-report-index)
links the 23 input reports, raw records and seals. All 30 retained F5 answer
seals verified at `755f78207b993cdfa4b0fd230abcafb1f3f606bc` (26,097 entries;
no missing or mismatching blobs). The
[broad numbered-question snapshot](../f5/answers/nh20261012coverage2/answers.md)
keeps F5-N01–F5-N16 remaining variants explicit, including suspended frames,
restart/reload after deletion, other traffic contexts and earlier-state
cleanup limits. These counts establish the recorded inputs and contexts.

The [clock bootstrap questions](../clock-bootstrap-epoch/registration-repair/README.md)
retain actual C8.6 registration and compiler-epoch observations. Separate
[mathfunc](../clock-bootstrap-epoch/registration-repair/mathfunc-bootstrap/README.md)
and [procedure/foreach](../clock-bootstrap-epoch/registration-repair/compiled-foreach/README.md)
questions retain original inputs and provider-specific outcomes; stock-shell
completion supplies no native iterator lifetime or F5 answer.

The [private clock-worker evidence](../results/runtime-c86-real-suites/README.md)
separates original release-selected C/Jim observations from nine exact Runtime
controls, two pure calendar controls and one Registry control. Native C8.6
implements `ParseFormatArgs`, `getenv` and `GetDateFields` with genuine shared
three-worker ownership; original getter order, reached table access and
timezone-name/Dict child identity remain explicit. Empty-table native localtime
refuses. Four original workers and full seven-worker lifetime remain unsupported.
Original all.tcl reaches orchestration but selects zero test files; no complete
upstream-suite or F5 worker conformance is established.
