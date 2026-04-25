# KCS: feature — Tcl 9.0 upstream test corpus

> **Audience:** Contributor
> **Type:** Functionality

## Summary

The upstream Tcl 9.0.3 C test suite, fetched on demand to
`tmp/tcl9.0.3/tests/`, is our source of truth for runtime and compiler
correctness. This note catalogues the 168 test files and groups them by
core subsystem so contributors can find the right test when fixing a bug
and judge what is in or out of scope for the correctness effort.

## Applies to

tcl-lsp CLI, compiler, runtime, vm, test-suite

## Question

What test files does the Tcl 9.0 corpus contain, which subsystem does
each one cover, and how do I run them?

## How to use

### Fetch the corpus

The corpus is lazy-fetched by the test harness and is also available
through the `fetch-tcl-source` skill:

```
bash .claude/skills/fetch-tcl-source/fetch_tcl_source.sh 9.0
```

This produces `tmp/tcl9.0.3/tests/` with 168 `*.test` files plus a few
support scripts (`all.tcl`, `internals.tcl`, `tcltests.tcl`). `tmp/` is
gitignored; do not commit the source tree.

Runtime fetch happens inside `tests/conftest.py` via
`ensure_tcl_source("9.0")`, which sparse-checks out `tests/` and
`library/` from the `core-9-0-3` GitHub tag. On a machine without
network, `ensure_tcl_source` calls `pytest.skip` — `make test-tcl9-full`
promotes that skip to a hard failure via `--tcl9-required`.

### Run the corpus

| Target | Purpose |
|---|---|
| `make test-tcl9` | Curated correctness baseline (fast, PR gate). |
| `make test-tcl9-full` | Every in-scope test file; requires network. |

Both targets emit a JSON report under `tmp/` that feeds the triage
table in [`kcs-tcl9-triage.md`](kcs-tcl9-triage.md).

## In-scope subsystems

Tests here exercise the core Tcl semantics we must match exactly:
parsing, commands, variable handling, list/dict/string primitives,
`expr`, control flow, namespaces, TclOO, `eval`/`upvar`/`uplevel`,
coroutines, and tailcalls.

### Parsing and substitution

`parse.test`, `parseOld.test`, `parseExpr.test`, `subst.test`,
`word.test`.

### List primitives

`list.test`, `listObj.test`, `listRep.test`, `llength.test`,
`lindex.test`, `linsert.test`, `lrange.test`, `lreplace.test`,
`lsearch.test`, `lset.test`, `lsetComp.test`, `lmap.test`, `lpop.test`,
`lseq.test`, `lrepeat.test`, `foreach.test`, `abstractlist.test`.

`lsort` and `lassign` live inside `cmdIL.test` and `cmdMZ.test` by
upstream convention (see "Command dispatch" below), not in dedicated
files.

### Dict primitives

`dict.test`.

### String, format, regexp

`string.test`, `stringObj.test`, `format.test`, `scan.test`,
`regexp.test`, `regexpComp.test`, `reg.test`, `get.test`, `split.test`,
`join.test`.

### Expr and math

`expr.test`, `expr-old.test`, `compExpr.test`, `compExpr-old.test`,
`mathop.test`. Math functions are exercised inside `expr.test`; there
is no separate `mathfunc.test` in the 9.0 tree.

### Control flow

`if.test`, `if-old.test`, `for.test`, `for-old.test`, `while.test`,
`while-old.test`, `foreach.test`, `switch.test`, `error.test`,
`result.test`. There is no dedicated `try.test`, `break.test`,
`continue.test`, or `return.test`; `try`/`throw`/`return` behaviour is
covered under `error.test`, `result.test`, and the command-dispatch
bundles. `break`/`continue` are covered implicitly inside the loop
files.

### Variables, scopes, namespaces

`set.test`, `set-old.test`, `var.test`, `upvar.test`, `uplevel.test`,
`namespace.test`, `namespace-old.test`, `trace.test`, `resolver.test`.
`global` and `variable` commands are covered inside `namespace.test`
and `var.test`.

### Procs, apply, info

`proc.test`, `proc-old.test`, `apply.test`, `info.test`,
`cmdInfo.test`, `rename.test`, `unknown.test`.

### Eval, subst, execution

`eval.test`, `subst.test`, `compile.test`, `execute.test`, `basic.test`.

### Command dispatch (letter buckets)

Upstream Tcl groups per-command tests alphabetically when no dedicated
file exists: `cmdAH.test` (A–H), `cmdIL.test` (I–L, includes `lsort`),
`cmdMZ.test` (M–Z, includes `lassign`, `lreverse`, `linsert` edge
cases). Treat these as cross-cutting and expect wide blast radius when
they fail.

### TclOO

`oo.test`, `ooNext2.test`, `ooProp.test`, `ooUtil.test`.

### Coroutine, NRE, tailcall

`coroutine.test`, `nre.test`, `tailcall.test`.

### Interp, safe, source

`interp.test`, `safe.test`, `safe-stock.test`, `safe-stock86.test`,
`safe-zipfs.test`, `source.test`. These depend on `file`/`source` and
may be partially deferred depending on which primitives we lower.

### Miscellaneous scalar and object machinery

`append.test`, `appendComp.test`, `concat.test`, `incr.test`,
`incr-old.test`, `obj.test`, `indexObj.test`, `dstring.test`,
`assocd.test`, `opt.test`, `stack.test`, `misc.test`, `brodnik.test`,
`range.test`, `bigdata.test`, `assemble.test`, `aaa_exit.test`,
`internals.tcl`.

## Deferred-by-design subsystems

These are recorded so triage can classify failures as **D** without
re-discovering scope on every run. They will not be fixed in this
effort.

### I/O, channels, sockets, event loop

`io.test`, `ioCmd.test`, `ioTrans.test`, `iogt.test`, `chan.test`,
`chanio.test`, `socket.test`, `http.test`, `http11.test`,
`httpPipeline.test`, `httpProxy.test`, `httpcookie.test`, `event.test`,
`async.test`, `notify.test`, `pid.test`, `process.test`, `pwd.test`,
`chan*`, `timer.test`.

### Filesystem

`fCmd.test`, `fileName.test`, `fileSystem.test`,
`fileSystemEncoding.test`, `link.test`.

### Encoding / i18n

`encoding.test`, `icu.test`, `utf.test`, `utfext.test`, `binary.test`.

### Packages, load, init, tm

`package.test`, `load.test`, `unload.test`, `pkgMkIndex.test`,
`autoMkindex.test`, `tm.test`, `init.test`, `main.test`, `config.test`,
`platform.test`.

### Threads and mutexes

`thread.test`, `mutex.test`.

### Registry, zipfs, zlib, clock, msgcat, history, env, security

`registry.test`, `zipfs.test`, `zlib.test`, `clock.test`,
`clock-ivm.test`, `msgcat.test`, `history.test`, `env.test`,
`security.test`, `dcall.test`, `tcltest.test`.

### Platform-specific

`macOSXFCmd.test`, `macOSXLoad.test`, `unixFCmd.test`, `unixFile.test`,
`unixForkEvent.test`, `unixInit.test`, `unixNotfy.test`,
`winConsole.test`, `winDde.test`, `winFCmd.test`, `winFile.test`,
`winNotify.test`, `winPipe.test`, `winTime.test`.

## Options

- `ensure_tcl_source("9.0")` — lazy fetch, `pytest.skip` on network
  failure. Defined in `tests/conftest.py:27-79`.
- `--tcl9-required` — harness flag; promote skip to hard failure.
- `--tcl9-report=<path>` — harness flag; emit the JSON triage artefact.

## Example

The expected layout after a fetch:

```
tmp/tcl9.0.3/
├── library/tcltest/tcltest.tcl      # bundled into every test run
└── tests/
    ├── list.test                    # in-scope, subsystem: list
    ├── socket.test                  # deferred-by-design
    └── ... (166 more)
```

A single file runs via pytest:

```
uv run pytest tests/external/run_tcl9_tests.py::TestTcl9_list -q
```

## Related

- [kcs-tcl9-triage.md](kcs-tcl9-triage.md) — triage table fed by the
  harness JSON report.
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
