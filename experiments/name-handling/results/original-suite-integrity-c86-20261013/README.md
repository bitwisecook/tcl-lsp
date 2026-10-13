# Original C Tcl 8.6.18 harness and naming suites

Question: does the manifest-selected source-built C interpreter start with its
matching original init/Tcltest and execute the unmodified procedure and rename
suite definitions with unchanged upstream constraints?

The full original `generic/tcl.h`, `library/` and `tests/` inventory matches
upstream commit `515167c897a7c8066abfbd71e8fe7bd8bf3569de`, including init, Tcltest,
package index, suite definitions, orchestration and support inputs. The
[upstream receipt](upstream-integrity.json) supplies independent original Git
blob identities; [observed SHA-256 inventory](upstream-inputs.json) identifies
the validated local input bytes. These are different evidence channels.

[Provenance](provenance.json) retains exact reference identity, binary hash,
measured startup stdout/stderr/status, original Tcltest 2.5.11, matching library
path, manifest identity and [runner bytes](runner-source.txt). Each process
retains its exact input wrapper, stdout and stderr beneath `c/`, with commands,
input/suite hashes and counts in [records.json](records.json).

| Original suite | Passed | Skipped | Failed | Skip constraint |
| --- | --- | --- | --- | --- |
| proc.test | 29 | 9 | 0 | procbodytest |
| rename.test | 11 | 8 | 0 | testdel |

Both processes exit zero and complete their original harness summary. Skipped
cases supply no reached semantic answer. Frame debugging is explicitly enabled
by the retained wrapper. No backend constraint overlay executes. These
measurements establish only this real C reference, exact inputs and invocation;
no VM/native-runtime, Jim, F5 or other-release result is implied. Seven separate
deterministic admission tests in `../../tests/test_run_suites.py` exercise
fixture corruption and invalid selection; their assertions are harness
validation, independently of these real C suite outcomes.

Reproduce from the repository root into a new output directory:

```sh
python3 experiments/name-handling/run_suites.py \
  --tcl-root tmp/tcl8.6.18 --tclsh tmp/tcl8.6.18/unix/tclsh \
  --version 8.6 --no-overlay --stem proc --stem rename \
  --output /absolute/new-capture-directory
```
