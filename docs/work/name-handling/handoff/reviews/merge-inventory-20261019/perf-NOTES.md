# perf agent notes: Analyser and SpecTcl

Status at 2026-10-22 18:40 UTC: full batch applied with a temporary switch in
`/workspace/tcl-lsp-work`, production base e4a01bbe. No production change is
committed. The required complete AnalysisResult proof is blocked by an all-old
object-report comparison failure; selected independent checks are still running.
Exact continuation instructions are in `docs/work/name-handling/outstanding/perf.md`.

## Changes and ownership

`perf/perf.patch` is the complete clean batch. `perf/perf-owned.patch` contains
only six files: lexer parse_cut.rs/source_map.rs; compiler original_word.rs,
original_name_value.rs, formal_call_advice.rs, incomplete_body_metadata.rs.
`perf/perf-realm-interprocedural.patch` contains the two compiler-owned callers.
Both patches exclude the switch. Main artifacts are refreshed against
origin/work 6df7bbea; clean application is byte-exact, but semantic proof on
that revision is pending. Exact e4a01bbe proof patches are retained in
perf/evidence/latest-e4a01bbe/proof-patches/. The six-file snapshot compiles independently
of the routed caller; zero owned Clippy diagnostics were observed, with 783
foreign diagnostics. Global warning/fmt gates are not claimed green.

- Build a SourceImage once per script rather than once per command. SourceImage
  equality first checks shared allocation, retaining full channel/byte fallback.
- Bound SpecTcl words check the immediate parent link; constructors establish
  the full chain inductively. Fixed tests cover a chain of at least three links,
  CRLF, bare CR, EOF, opaque bytes and both source channels.
- Cache original realm query geometry per immutable realm, retaining allocation,
  registry and metadata identities. Caller-alias surveys are cached per callee;
  argument-to-formal mapping still runs per invocation.
- Cache incomplete-body headers by retained image, lexer configuration and region.
- Compiler-owned callers reuse the realm invocation memo and pass the query memo.

These target measured repeated binding construction (~50% of samples),
caller-alias declaration surveys (~35%), repeated invocation joins, document-tail
header parsing, whole-script image construction and full ancestor-chain checks.

## Equality blocker

`perf/analysis-equality-routing.md` records the owner handoff and exact traces.
The aliases fixture fails complete PartialEq with all optimizations disabled,
including when the same independently prepared ResolvedAnalysisInput is reused.
The first false helper is ObjectHandleFacts::analysis_report_eq. Both admitted
report-source options are absent; raw facts then compare unequal across runtime
owners. A hosted iRule also fails the all-old control; its helper has not been
privately traced. Diagnostics/public-field equality does not replace the whole
result verdict. No assertion, owner guard or comparator was weakened.

The text roster is now exactly 102 UTF-8 Tcl inputs plus 15 iRules. The original
roster had 101 text files and an intentional non-UTF-8 native fixture; the latter
remains a separate actual native codegen control (passes in both modes), and a
class fixture supplies the additional text case. No positive input was dropped.
Latest calls 10/20/40/80, procs 41 and upvar 10/30 all passed complete
old/new/old equality with deterministic old endpoints. Latest full ordered PackSet Debug comparison also PASSED (484,936 lines,
deterministic old endpoints). Full 102-file text corpus completed 100 equal / 2 unequal (E003 and aliases),
zero panics; both mismatches fail isolated all-old controls. All 100 successful
rows have deterministic old endpoints. All 15 iRules failed strict equality.
Earlier base-8a01a718 ordered full PackSet Debug comparison passed deterministically
(484,936 lines); this is not a latest-source verdict. Never sort Debug output or
strip fields to manufacture equality.

## Measurements

Full raw results and provenance are in `perf/evidence/timings.md` and
`perf/evidence/latest-e4a01bbe/`. Wall times are contended opt-level-1 observations,
with debug assertions enabled, rather than release latency claims. origin/rust
is fcb7927f02b5395753247b322fdd7be87ae5c731, built in its own checkout/target.

Latest new-only tclIndex timings (milliseconds):

| Lines | origin/rust | work + full batch |
| --- | ---: | ---: |
| 10 | 163 | 1,319 |
| 40 | 492 | 11,962 |
| 100 | 1,021 | 53,223 |
| 253 | 1,596 | 255,893 |

The new-only probe prints an equality placeholder; these rows prove timing only.
The older base three-way run established equality at 10/40/100; the old 100-line
latency was perturbed by debugger sampling and must not support a speedup ratio.
The full old/new/old 253-line check remains active.

Ordinary LSP positive-reference controls passed at n=10/20/40/80, and scope-chain
controls at 250/500/1000/2000. The work harness uses retained editing inputs as
the server document driver does. Every lens has one actual reference.

| Procedures | origin index ms | work index ms | origin lenses ms | work lenses ms |
| --- | ---: | ---: | ---: | ---: |
| 10 | 0.041 | 64.244 | 1.180 | 83.642 |
| 20 | 0.082 | 445.391 | 2.172 | 1,380.511 |
| 40 | 0.381 | 675.448 | 27.844 | 8,034.283 |
| 80 | 0.497 | 1,557.801 | 106.465 | 25,542.691 |

These are medians of three queries. Ordinary Logical lenses repeatedly rebuild
call inventory/index, including inside currentness checks. Request-scoped admitted
inventory reuse is routed to the LSP owner; no identity check may be bypassed.

Latest load_embedded: origin 4,665 ms, work old1 44,471 ms / old2 82,646 ms,
new 10,673 ms. Full ordered PackSet Debug is equal and old runs deterministic;
the endpoint timing difference prevents a stable speedup-factor claim.

Supplemental paired process CPU: calls 80 old1/new/old2 =
35,450 / 3,760 / 21,660 ms; calls 40 = 10,350 / 2,460 / 8,480 ms.
Calls 20 new falls between the old endpoints. All results are exactly equal
with deterministic endpoints; see synthetic-cpu-summary.json. This measures
analysis, excluding report comparison/formatting, rather than LSP or WASI exit.

## Test status

Latest lexer lib: 583 pass and the same three fail in each mode, zero pass-to-fail.
The isolated formal-advice test has the same [] versus [notacommand] assertion
failure in both modes. Selected compiler BOTH modes completed all 226: identical 151 pass /
75 fail, zero pass-to-fail and identical complete inventories. Startup metadata fails the same first 0-versus-1 assertion in both
modes (24.451 / 23.893 s), so later reindex assertions are not reached.
Workspace-symbol times out at 180 s in BOTH modes (180.085 / 180.224 s).
No pass-to-fail was observed; neither server gate passes.
These are failed gates, not evidence that the two server cases are repaired.

Earlier base SpecTcl opt-level-1 eval_loader: all 28 pass in both modes
(new 1,482.833 s, old 2,614.057 s). Default-profile golden_packs: two pass and the
same interpreter 30-minute budget failure in both modes (new 3,537.662 s,
old 5,717.749 s). The earlier default eval_loader run lacked one sparse fixture;
complete-input default new mode finished 26 pass / 2 fail (4418.488 s),
now budget-axis and eda_xilinx interpreted wall-budget assertions; old is active. Latest optimized eval_loader completed all 28 PASS in BOTH modes
(new 1,488.967 s / old 1,568.082 s), zero pass-to-fail. Latest optimized
golden_packs completed all 3 PASS in BOTH modes
(new 651.764 s / old 864.494 s), zero pass-to-fail. Profile
or fixture differences are recorded separately, never presented as green retries.

## Routed residual work

`perf/perf-analyser-images.patch` then `perf/perf-analyser-segments.patch` are
UNPROVEN drafts, not applied, for compiler-names-owned paths. Their controls and
limits are in `perf/residual-routing.md`. Image reuse retains full byte/channel
checks, still O(N) per command; eliminating those checks requires an admitted
immutable operation context. Segmentation reuse retains the authentic origin,
configuration and complete token vector. Review section 4.1 still needs owner
fixes after this batch; the LSP timings above demonstrate the remaining cost.

Do not commit the six production files or mark the compiler patch proven until
the complete required equality proof is valid. Finish pending comparisons, update
this file with final before/after values, and stop without opening a PR.
