# #2376 source-result and admission checks

Frozen candidate `e3a5196d1` plus `candidate.patch` passes all six focused unit
tests. This includes six canonical RHS-dispatch controls, unsupported
numeric/composite/frozen-argv guards and 12 actual-input source-result probes.
All 41,075 file hashes match through build and runtime. The library test binary
ran directly outside the shared build lock; its command, digest and raw output
are retained here.

All 12 actual-input probes retain `AuthoredSimulation`, Direct compilation,
complete supplied metadata, no Native entry, no source-point entry and no
retained inline/named compiler admission. The four final-store observations
return `0` and report that invocation completes normally when reached. The
eight whole-expression observations retain an unavailable result and no
complete-normal proof. In particular the child `9` cannot become the whole
Boolean AND/OR result. All 36 native C Tcl 8.4–9.1/current Jim checks independently
confirm the final-store and whole-expression expected values.

`SourceNormalResult` denotes the value on represented Normal alternatives; its
owner grants neither a physical object nor totality. The 30 newly available
final-store values in the prior archive are therefore stronger source
dataflow, not by themselves a compiler admission or physical result receipt.
That prior blanket-unavailable assertion remains archived without alteration.
The current tests replace it with exact source values and explicit independent
gates. Their strengthened assertions await the scheduled final combined run;
candidate code remains local and full `rust-check`/`prep-pr` gates are pending.
