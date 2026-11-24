# Command definition probes

Run `bash scripts/dev/run-definition-oracles.sh` from the checkout root on
Linux. The runner provisions the pinned Tcl and Tk 8.4–9.1 releases, Jim and
Tcllib, then invokes `cargo xtask definitions-gate`. Tk runs under Xvfb. For
an existing toolchain, set `TCL_LSP_TCLSH84` through `TCL_LSP_TCLSH91`,
`TCL_LSP_TK84` through `TCL_LSP_TK91` (the Tk shared libraries in the
upstream build trees), `TCL_LSP_JIMSH` and `TCL_LSP_TCLLIB_MODULES`.

The driver checks interpreter inventories and authored registry command and
subcommand names. It runs each call through the analyser in its own source
document and records the reference usage or error alongside the diagnostic.
The committed allow-list keys each measured difference by backend, patch
level, command, subcommand and operand count. The confirmed rows also run
from ordinary compiler integration tests without external interpreters.

For commands that can block or abort on dummy arguments, the driver checks
presence and uses representative calls from [confirmed.tsv](confirmed.tsv)
where available. The Tcllib lane exercises `oo::util` and `struct::list`.
The results and usage messages are in `tmp/command-definitions` by default.
CI uploads these reports. The runner checks every observed usage line against
the registry synopsis and runs each distinct C Tcl usage line through both
project engines. The [usage allow-list](known-usage-differences.tsv) records
missing commands, validation differences, and wording that varies across
implementations. Tk and Jim usage is checked against the registry because the
project engines do not offer those execution environments. Six portable
controls must match byte for byte on every Tcl release.

The numbered procedure below produced the original
[registry definition errors](../../../registry-definition-errors.md) report.

1. `oracle.tcl OUT [EXTRA] [only]` under each `tclsh8.4`–`tclsh9.1`, from an empty scratch directory: enumerates global, `oo::`, `tcl::mathfunc`/`mathop`, `tcl::prefix`, exported package commands (`http`, `msgcat`, `tcltest`, `platform`, `opt`) and `auto_index` commands, discovers subcommands from the `__bogus__` error, and calls each command and subcommand with 0–7 and 12 dummy arguments in a fresh child interpreter, recording `ok`/`args`/`nocmd`/`badsub`/`other` and the message. `EXTRA` lists registry commands and subcommands the interpreter did not enumerate (`cmd<TAB>sub`); `only` probes just those.
2. Tk: `run_tk.py OUT EXTRA oracle_tk.tcl` under `xvfb-run` with `TCL_LIBRARY`/`TK_LIBRARY` pointing at Python's Tcl/Tk 9.0.3 libraries.
3. `analyser_probe2.rs` (append temporarily to a `tcl-compiler` test module importing `Analyser`; run with `ANALYSE_IN`/`ANALYSE_OUT`, opt-level 2): analyses each probe line on its own under the matching dialect, with `package require` for package commands, and records E001/E002/E003/E005/W001/W002/W123/W004. One analysis per line matters: W001 is reported once per command per file.
4. `compare.py N` joins the two and lists false arity, missed arity, unknown and claimed-nonexistent commands and subcommands (`compare-results.txt`). Treat it as candidates: dummy arguments fail value checks before some commands count arguments, nested ensembles report their own subcommand, and W123 on `__x__` passed as a command name is correct.
5. `probe.rs` with `cases.tsv` runs exact valid or invalid calls through the analyser; `v.tcl` runs the same calls under tclsh. Every error in the write-up was confirmed this way.
