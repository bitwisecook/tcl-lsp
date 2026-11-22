# Command definition probes

Harness behind [registry definition errors](../../../registry-definition-errors.md).

1. `oracle.tcl OUT [EXTRA] [only]` under each `tclsh8.4`–`tclsh9.1`, from an empty scratch directory: enumerates global, `oo::`, `tcl::mathfunc`/`mathop`, `tcl::prefix`, exported package commands (`http`, `msgcat`, `tcltest`, `platform`, `opt`) and `auto_index` commands, discovers subcommands from the `__bogus__` error, and calls each command and subcommand with 0–7 and 12 dummy arguments in a fresh child interpreter, recording `ok`/`args`/`nocmd`/`badsub`/`other` and the message. `EXTRA` lists registry commands and subcommands the interpreter did not enumerate (`cmd<TAB>sub`); `only` probes just those.
2. Tk: `run_tk.py OUT EXTRA oracle_tk.tcl` under `xvfb-run` with `TCL_LIBRARY`/`TK_LIBRARY` pointing at Python's Tcl/Tk 9.0.3 libraries.
3. `analyser_probe2.rs` (append temporarily to a `tcl-compiler` test module importing `Analyser`; run with `ANALYSE_IN`/`ANALYSE_OUT`, opt-level 2): analyses each probe line on its own under the matching dialect, with `package require` for package commands, and records E001/E002/E003/E005/W001/W002/W123/W004. One analysis per line matters: W001 is reported once per command per file.
4. `compare.py N` joins the two and lists false arity, missed arity, unknown and claimed-nonexistent commands and subcommands (`compare-results.txt`). Treat it as candidates: dummy arguments fail value checks before some commands count arguments, nested ensembles report their own subcommand, and W123 on `__x__` passed as a command name is correct.
5. `probe.rs` with `cases.tsv` runs exact valid or invalid calls through the analyser; `v.tcl` runs the same calls under tclsh. Every error in the write-up was confirmed this way.
