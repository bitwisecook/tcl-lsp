# C Tcl 8.4 real naming suites

Twelve bounded processes execute six unmodified upstream Tcl 8.4.20 naming
files against the real C reference and a frozen Original-native runtime.
Both use matching real `init.tcl` and Tcltest 2.2.11. [Provenance](provenance.json)
and [execution subject](execution-subject.json) keep the frozen executable
separate from the repository commit visible when captures started.

| Suite question | C passed / skipped / failed | Runtime passed / skipped / failed | Runtime completion |
| --- | --- | --- | --- |
| [Namespace](questions/namespace.md) | 186 / 3 / 0 | 176 / 3 / 10 | Complete |
| [Variable](questions/var.md) | 64 / 0 / 0 | 64 / 0 / 0 | Complete |
| [Upvar](questions/upvar.md) | 38 / 0 / 0 | No final summary | Host refusal at `upvar-1.1` |
| [Procedure](questions/proc.md) | 19 / 0 / 0 | 19 / 0 / 0 | Complete |
| [Rename](questions/rename.md) | 11 / 0 / 0 | 11 / 0 / 0 | Complete |
| [Interpreter/alias](questions/interp.md) | 261 / 5 / 0 | 13 earlier assertion failures; no final summary | Host refusal at `interp-11.4` |

C completes all files with 579 passes, eight upstream `knownBug` skips and no
failures. The four completed runtime files have 270 passes, three skips and ten
failures. Its interpreter stream contains 13 additional assertion failures
before a host refusal; there is no complete runtime total for that file or
upvar. Aborted cases and their remaining tests are unmeasured, not failed tests.

Namespace gaps cover teardown error-state preservation, forgotten imports,
empty non-global namespace creation, namespace-eval error text, import-query
state and missing-name `namespace which` diagnostics. Interpreter gaps include
`slaves` error presentation, current-interpreter deletion, alias argument errors,
cross-interpreter alias capabilities and target-query validation. The next
unreached operations require an original simple-variable cell in upvar and a
nested alias source-interpreter policy. Per-question documents identify exact
cases and retain assertions separately from host refusals.

## Scope and reproduction

The [runner snapshot](runner-source.txt) runs each file in a fresh temporary
working directory with a 90-second process limit. No timeout was reached. The
native invocation selects `--init --quiet --tcl-version 8.4`; no Logical
adapter ran. `--no-overlay` retains the upstream harness constraints because
the repository's optional backend overlay uses Tcl 8.5 syntax. That overlay is
absent from both providers; its recorded hash does not imply execution.

[Retained oracle inputs](oracle-inputs.json) include the exact six suites,
real init, real Tcltest and Tcltest package index. Their original upstream
copyrights remain unchanged. Every wrapper, original suite, executable and
library digest was checked against the capture records; [validation](validation.json)
records counts and source-hash checks. [Per-process records](records.json), raw
streams and wrappers retain setup, skips, failures and refusal boundaries.

These are newly reached actual-native 8.4 measurements after the
[existing physical-cell repair](../../native-c84-declared-cell/README.md).
Earlier 8.4 startup attempts stopped before the suites; historical Logical
captures cannot supply a previous native pass rate. The earlier 8.5 namespace
capture reported some matching case IDs, but different releases and suite
inputs cannot establish a regression comparison. Neither Jim nor F5 was
queried for these exact suite files, and no skipped case gains an oracle answer.

The ordered full-file state is part of each question. Classifications identify
observed discrepancies; they do not prove an isolated cause or authorize
rewriting provider answers to agree with an implementation.
