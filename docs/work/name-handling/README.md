# LSP naming migration on work

Updated 04 November 2026, 10:07 UTC. [current-state.md](current-state.md) records exact sources, outcomes and limits; [outstanding work](outstanding/README.md) lists open acceptance.

**The LSP is not release-ready.** Scope is the LSP and its transitive libraries. Separate VM/BIG-IP/ancillary features are deferred; linked hook execution and required repository gates remain in scope.

Reviewed work and truthful status are pushed every five minutes, with remote ancestry checked and incoming changes safely integrated. [repository-sync.json](repository-sync.json) records actual checkpoints, not test acceptance. No PR, merge, tag or release exists.

Compiler exact-input6/6 and closed-cut KernelV5 11/11 pass on their recorded source. CoreV3 passes all11 autoload controls and15/15 installed fresh/full vectors; its joint20 controls fail five rename cases. Reviewed1fa97847b adds a distinct captured-row source-reference producer and corrects invalid test prerequisites. The pre-merge focusedCore run builds0 and passes7/11; positives remain blocked, one no-op negative mutation is corrected but unrerun. CompilerV7 build failed a remaining slice assertion before tests; its fix is adopted and V8 is running. No repaired positive is established.

RMWv6 remains failed: Engine1/6, hooks10/13, Jim1/1. Adopted error-contract/budget test corrections await the prepared v7 run. Earlier Server7/7 and native15+20+5 passes predate current changes; the13-control Server run and final native/browser/WASI acceptance remain required. Public host CI on its older exact source still has symbol/autoload nulls, rename refusals and a test-helper type error; later local fixes do not relabel those results.

ClippyV19 fails41 emitted type/inference diagnostics; scoped repairs are adopted, full workspace formatting passes, and final lint remains pending. The sealed global proof recount fails2171errors before a subsequent two-binding authenticated source correction; no post-adoption count or pass exists. Packaging, performance and required final gates remain incomplete.

The separate remote [issue sweep](../../LSP_RELEASE_STATUS.md) does not certify naming-migration acceptance. State/process material must be removed before merge while preserving reproducible fixtures and oracle provenance.
