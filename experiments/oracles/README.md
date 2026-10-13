# Oracle questions and evidence

One Markdown document owns each answered item: its actual question, scoped
answer, measured results, exact input and evidence references, and remaining
gaps. Related reductions, source inspection, setup refusal and another provider
do not silently answer the original item.

The [canonical naming proof pages](../../docs/design/analysis/name-resolution-proofs/README.md)
remain the semantic authority for C Tcl, Jim Tcl and BIG-IP. Their manifest
binds exact evidence, source anchors, consumers and tests. The
[F5 question views](f5/questions/index.json) put the requested context matrix,
filtered measurements and independent C/Jim controls together in one Markdown
file per F5 item, with an adjacent JSON description.

| Location | Owns |
| --- | --- |
| `docs/design/analysis/name-resolution-proofs/*.md` | Individual reviewed questions, answers, provider scope and evidence |
| `experiments/name-handling/patterns/probes/` | Original portable input sources |
| `experiments/name-handling/patterns/results/` | C Tcl capture inputs, separate streams and provider provenance |
| `experiments/name-handling/patterns/wiki-results/` | Real C Tcltest outcomes and matching library identity |
| `experiments/name-handling/patterns/jim-results-5bac7c9/` | Jim input wrappers and independent setup/question completions |
| `experiments/name-handling/results/` | Original C/Jim suites, runtime comparisons, refusal/abort receipts and shared capture-level provenance |
| `experiments/name-handling/patterns/<pattern>/` | Source-derived callers, original and reduced questions, provider results and consumer boundaries |
| `experiments/oracles/f5/` | F5 base, group setup, test deltas, question views, run observations and indexed captures |

C Tcl and Jim remain separate providers. C Tcltest PASS/SKIP measurements and
Jim inner-script completions are different evidence channels. A skipped or
unreached operation supplies no final lookup observation. Original source,
wrapper, binary/library identity and separate stderr remain part of the proof.

The reviewed portable-pattern captures occupy about 642 KB across 567 files;
their original inputs and receipts remain in place. They do not need the large
F5 storage machinery. Future small observations can be embedded directly in
their question page; shared provenance and binary inputs remain referenced once.

The wider suite/comparison tree contains about 133 MB in 5,248 files in the
reviewed snapshot. Its large shared source/binary snapshots belong to a capture,
not each answer. Source-derived lazyset callers refuse during setup on C8.4 and
Jim while later C releases reach the reader lookup. Composed callbacks capture
background errors independently of successful process exits. Original suite
orchestration that sources zero files, and a real allocator abort, remain
incomplete suite results. Preserve those distinctions when reducing the records.

Use this common ownership layout for new evidence:

```text
questions/<item>.md          exact question, answer, filtered observations and gaps
captures/<run>/              shared provider/build/source identity and raw streams
sources/<source>/            pinned upstream input and licence, shared by questions
tools/                      replay runners; provider-specific harnesses
f5/base/                    one deployment base
f5/groups/<group>/          setup and state/lifecycle prerequisites
f5/questions/<item>.json    F5 configuration and command selectors beside its answer
f5/fixtures/                separately indexed product datasets and format controls
```

A question page should name the original operation and its prerequisites,
provider/context, exact invocation and client/server roles; show the smallest
complete result that answers it; and declare `complete`, `partial` or `none` with
a reason and explicit remaining variants. Include setup and operation completion
separately, including empty, missing, false and zero values. A selected log excerpt
needs its source/run/phase/worker and selector. A PCAP excerpt needs the complete
relevant conversation and decoding prerequisites. A suite excerpt needs test
definition, constraints, expected and actual result, and harness outcome.

One Markdown file remains the human answer. Shared capture/source files are
referenced rather than repeated, and F5 sidecars hold executable structure.
Existing C/Jim paths remain stable in this F5 migration; a later consolidation
can preserve their identities through the same indexed reader.

F5's larger runs use indexed, compressed artefacts. This preserves exact
bytes, including NUL, non-UTF-8 values, error options, chunk boundaries and
ordering, without expanding thousands of duplicate files. Decoded summaries
do not replace the capture bytes or change answer coverage.
