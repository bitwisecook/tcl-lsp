# How can WASM preserve the original publication name and compiled body key?

Status: **UNAPPLIED / UNCOMPILED / UNEXECUTED**. These sources propose a software
repair and controls; they supply no implementation pass or emitted-WASM result.
The independently executed [original question](../question.md),
[input](../input.tcl) and provider streams remain unchanged.

The current definition transport reuses a constructed procedure key as fresh
publication input. The proposed `DefineProc` instead retains the existing
evaluated original argv and a separate internal compiled-entry key. One additive
`ProcDefineOriginalArgv(argv, argc, entry, out)` import shares the original Runtime
procedure parser, validation and actual namespace owner, with an optional
compiled entry. It does not evaluate the source words again or look up a holder
from the rendered key.

The general emitter declines its direct `ProcRegister` shortcut into the existing
once-evaluated leaf argv path. That shortcut currently installs no compiled entry;
native table installation retains its independent internal key. The generic path
changes dispatch/parser work and can expose original validation/admission behavior
that the old shortcut omitted. Performance and complete provider equivalence are
unmeasured. Unknown argument/context ownership preserves the existing generic
invocation or typed admission refusal.

The additive import retains both legacy import signatures and exports, but changes
the derived ABI fingerprint. The existing manifest gate compares fingerprints
strictly and refuses older fingerprinted artifacts. The proposal does not waive
that check; new compiler and Runtime artifacts must use the same descriptor table.

Artifacts are separate:

| Artifact | Purpose |
| --- | --- |
| [design.md](design.md) | Actual owner chain, representation/lifetime boundaries, compatibility and remaining validation |
| [implementation.patch](implementation.patch) | Unapplied diff for seven implementation owners |
| [test-updates.patch](test-updates.patch) | Proposed existing-test, import-counter and descriptor checks |
| [compiler-controls.rs](compiler-controls.rs) | Fixed C86/C9 control requiring an actual `DefineProc`, plus original operand/shared-slot checks and adverse inputs |
| [runtime-controls.rs](runtime-controls.rs) | Proposed actual Runtime object/current-holder and missing-holder controls |
| [provenance.json](provenance.json) | Exact source preimage and artifact byte hashes |
| [prepare-implementation.py](prepare-implementation.py), [prepare-test-updates.py](prepare-test-updates.py) | Read source and prepare temporary diffs without applying them; required source shapes are asserted |

The compiler control must prove that the source reaches `DefineProc` under its
actual retained entry and admission policy. Comparing a QName formula alone is
insufficient. If that setup refuses the declaration, the refusal remains explicit;
a default Native frame/entry cannot force the control through. Counting
substitution, a replaced definer, missing evaluated argv and changed namespace
ownership independently require the correct fallback. Actual Runtime/WASM
execution must establish call results and once-only effects separately.
