# Runtime Native C8.6 namespace-forget validation

The first and only offline standalone Runtime invocation passed the exact
three `native_c86_forget_` tests: original #2300 witness, fixed adverse
controls, and five original import-chain controls. The tests use the genuine
Native C8.6 interpreter fixture, retained source blocks and real C8.6 raw FIELD
expectations. Three tests passed, none failed or were ignored; 2,086 other
Runtime tests were filtered out. No test or source repair/retry was needed.
The complete build log retains its 15 Runtime warnings; no lint gate was run.

The namespace driver consumes shared purpose-selected pattern parts and the
actual current namespace's constructed components. Storage resolves those
components directly to a live namespace token and follows actual imported
source generations to the current ultimate command slot. Qualified matching
selects the ultimate tail when its namespace matches; only a namespace mismatch
permits the immediate first source's current tail. It does not search other
middle hops or fall back after an ultimate-tail mismatch. Unqualified matching
uses the imported binding's current own name. Command retirement and original
argument currency remain delegated to their existing owners.

The [result](result.json) retains exact argv, UTC start/completion, process exit,
log hash and executed test binary hash. The before/after source envelope covers
2,897 source/include/csrc/manifest/config and exact input files across declared
local dependencies; all recorded hashes remained identical. It conservatively
includes optional and other-target dependencies, so it does not establish that
every recorded file compiled. The separate fixture context was observed during
the frozen compilation rather than before the command. The retained binary
[dep-info](executed-runtime-test.d) and compressed source membership record
identify the actual Runtime lib-test source paths observed after execution and
distinguish their earlier capture windows. The observed dep-info names 1852 actual Runtime source/input files, with
1304 older shared fixtures first hashed after completion; their pre-build bytes
are not independently attested here. No missing paths or changed previously
captured files were observed. These observations are not a trace of every
compiler source read or a complete transitive binary attestation.

[Question](question.md) · [Reproducible runner](run.py) · [Complete log](cargo-test.log)

The original C5/Jim witness/adverse raw evidence and seals remain unchanged.
The separate [chain evidence](../../namespace-forget-import-chain/README.md)
records all C5 successes and Jim's unchanged unsupported-forget boundary.
No native F5 execution, Tcltest harness, other-release Rust consumer run or
broader namespace suite was performed. Import replacement and teardown
lifetimes (#1778) remain unproved. This validation supplies bounded #2300
consumer evidence and its explicit adjacent controls, not a full name-handling
or repository gate result.
