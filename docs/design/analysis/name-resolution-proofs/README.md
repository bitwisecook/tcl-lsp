# Name-resolution proof records

Each page answers one concrete naming question: the problem statement, the
question, the conclusion, its scope, each provider's answer, the source excerpts
inspected, the Rust code and tests bound to it, and the retained evidence it
rests on.

| Kind | What it establishes |
| --- | --- |
| `native-observation` | What the retained program actually did on the named provider. It does not establish an unexecuted branch or the Rust implementation's behaviour. |
| `native-attempt` | The probe stopped during setup before reaching its question; every provider answer stays `not-tested`. |
| `source-anchor` | What the quoted source excerpt says at its recorded revision. C source does not establish Jim or BIG-IP behaviour. |
| `implementation-contract` | A Rust ownership or consumer invariant. Observations can motivate it; they do not execute it. |
| `environment-observation` | An SDK/libc/tool environment result, kept separate from the Tcl/Jim/BIG-IP provider answers. |

Provider answers cover Tcl 8.4, 8.5, 8.6, 9.0, 9.1, Jim and BIG-IP. Providers
with identical answers share one heading. `not-tested` means the provider did
not answer this question; `unsupported` is an actual captured rejection. Never
infer a release or behaviour from another provider's answer.

The **Evidence** list points at the inputs and outputs the answer rests on,
usually native captures under `rust/*/tests/data` or `runtime/rust/tests/data`.
Standalone Tcl execution evidence, including every BIG-IP run, lives in
[`evidence/tcl`](../../../../evidence/tcl/README.md) in the format defined by
[tcl-execution-evidence.md](../../contracts/tcl-execution-evidence.md). New
observations go there, not here.

These pages carry no digests: retained bytes in git are the record. Record a
hash only when the hash itself is what the experiment measured.
