# Original C Tcl 9.1.0 naming-suite integrity

Question: does the source-built reference execute the original procedure and rename suites using its matching original init/Tcltest and unchanged constraints?

The runner validates every retained upstream library/test blob against the immutable manifest-selected upstream commit before capture. [Provenance](provenance.json) records the binary hash, source tag/commit, exact measured startup and original library/Tcltest. [Original authority](upstream-integrity.json) and [observed input hashes](upstream-inputs.json) remain separate. Each process has its original suite hash, exact wrapper, invocation, stdout/stderr and completion in [records.json](records.json) and c/.

| Original suite | Passed | Skipped | Failed | Status |
| --- | --- | --- | --- | --- |
| proc | 29 | 9 | 0 | pass |
| rename | 11 | 8 | 0 | pass |

No backend overlay runs. The retained wrapper enables frame debugging; skipped cases supply no oracle answer. These results certify only the identified C interpreter and inputs. No Runtime, VM, Jim, F5 or current consumer equivalence is implied. The exact runner bytes are retained in runner-source.txt. Reproduce with run_suites.py, this manifest-selected version/source-built shell, --no-overlay --stem proc --stem rename, and a new output directory.
