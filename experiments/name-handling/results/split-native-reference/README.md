# Native split reference question

Which existing native byte/character owner can remove the original pre-suite Unicode refusal while preserving split's actual branches?
The original all.tcl failure remains open; this batch implements nothing and runs no Tcltest suite.

`cases.json` has 18 deterministic synthetic inputs: ASCII/default/single/duplicate/empty sets, literal and encoded NUL, non-BMP, surrogate pairs/lone surrogate, naked FF and truncated UTF8.
Actual C8.4.20, C8.5.19, C8.6.18, C9.0.4 and C9.1.0 execute the same counted raw scripts through real Tcl_Init/Tcl_EvalEx.
This bypasses a script source channel's decoding, deliberately preserving the written native byte extent; C also directly reports each result element's original bytes and identity 0/2.
Pinned Jim's original jimsh executes the same files unadapted; its own real static-extension/bootstrap setup is retained, with no C init substitution.
Jim reports 0.84-9-g5bac7c9, pinned revision 5bac7c99ad65864c87da513e22e2f01703fa4e03.
Native uses the unchanged retained runner with `--quiet --tcl-version 8.6 --init` and the matching real C86 library.
No environment values are queried/output; all inspected values are benign synthetic case data.

`capture-01` preserves the faulty 512MiB limit: all 18 Native processes panic before evaluation because the existing runner needs a 512MiB stack plus address space.
It also retains six post-split C9 binary-projection guest errors; these are not split errors.
`capture-02` uses 1GiB and catches only post-split guest projection errors, preserving their diagnostic rows while restoring the original split result for C object inspection.
It contains 5 successful C control compilations and 126 case processes: 90 C, 18 Jim, 18 Native.
C/Jim complete split in all 18 cases per engine. Native completes 9 and operationally refuses 9 before a COUNT row (exit 2), including empty input with invalid delimiter.
Six C9 case rows contain later binary-projection errors, separate from successful split and retained raw element bytes.
All captured source/binary before/after hashes agree; initial failures and exact historical capture source copies remain retained.

`counts.tsv` reports element counts or Native process refusal; `element-results.json` retains C raw results/identity and projection errors, whose byte domains must not be conflated.
All C releases stop at a literal NUL in the single-byte comma branch (one element); duplicate commas select the counted general branch (two). Native/Jim return two in both.
A raw four-byte supplementary delimiter yields 9 elements in C84/85, 5 in C86, 3 in C90/91/Jim/Native.
Empty delimiters on a raw surrogate pair yield 2/2/1/2/2/2 elements in C84/C85/C86/C90/C91/Jim; Native refuses.
C86 encoded NUL returns two elements; Jim returns three; Native refuses at byte 3. These are independently measured version axes.
C empty-set repeated ASCII elements 0 and 2 are the same actual object in all five releases; Native physical reuse remains unmeasured without a new Rust build.

`api-proposal.md` scopes reuse to the current shared decoder, generic physical-value API and shared split owner; no global UTF8 loosening, invented target fallback or cache parity claim.
Actual F5 TMM/MCPD native contexts, browser/WASI and other Native releases remain unanswered; no standalone Tcltest result is attributed to F5.
Exact scripts, source/library/binary hashes, invocation/limits, process exits and raw stdout/stderr channels are in capture-*/provenance.json and engine/case directories.
Compiled C executables are ignored by the scoped .gitignore; source/build commands and binary hashes are retained.
No Rust edits, Cargo, Git, network or canonical documentation mutations occurred. The separate env synchronization gap is unchanged.
