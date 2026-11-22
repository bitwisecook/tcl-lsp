# Errors in the registry's command definitions (Tcl, Tk, packages)

**Scope:** the command definitions for Tcl 8.4–9.1, their bundled packages (`http`, `msgcat`, `tcltest`, `platform`, `opt`, `safe`), TclOO and Tk. iRules and BIG-IP are left for a separate review. tcllib could not be checked: the tcllib download failed in this container.

**Method:**
1. Every command and subcommand that tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 know was called with 0–7 and 12 dummy arguments, each in a fresh child interpreter. So was every Tcl command and subcommand the registry defines that the interpreter did not list.
2. Tk commands were checked for existence against Tk 9.0.3.
3. The same calls were analysed, one per file, by the current analyser under the matching dialect (43,650 analyses).
4. Mismatches were treated as candidates. **Each error below was then confirmed by running a valid (or invalid) call under tclsh and through the analyser.** Harness, candidate lists and confirmation cases are in [`definitions/`](handover/oo-dispatch-probe/definitions/).

**Not errors:** the per-release subcommand availability checks are right throughout, for example W002 on `array default` in 8.6, `string bytelength` in 9.0, `dict getdef` in 8.6 and `string reverse` in 8.4, and W001 on `info alias`. Arity is right for the great majority of commands.

## Valid code reported as an error

| Call | Release | Tcl | Analyser | Cause |
| --- | --- | --- | --- | --- |
| `destroy` (no arguments, Tk) | 9.0 | accepted | E002 "expected at least 1" | arity says at least 1; the synopsis says `?window …?` |
| `http::formatQuery` (no arguments) | all | accepted | E002 | minimum set to 2 |
| `http::formatQuery a` | 8.4, 8.5 | accepted | E002 | no even-count rule before http 2.7 |
| `http::meta token name` | 9.0, 9.1 | accepted (`?headerName?` added in http 2.10) | E003 | one arity for every release |
| `http::registerError sock msg` | 8.6–9.1 | accepted | E003 | maximum set to 1 |
| `tcltest::cleanupTests 0` | all | accepted (`?calledFromAllFile?`) | E003 | maximum set to 0 |
| `puts stdout x nonewline` | 8.4 | accepted (legacy form) | E003 | maximum set to 2 for 8.4 |
| `link` with no arguments in a method | 9.0, 9.1 | accepted | E002 | minimum set to 1 |

## Invalid code accepted

**Commands that do not exist, accepted as commands:**
- **`tcl::mathop` and `tcl::mathfunc`.** These are namespaces, and the spec's own hover text says "a namespace, not a command".
- **The bare operator names** `+ - * / % ** ! ~ & | ^ << >> == != < <= > >= eq ne in ni`, and `lt le gt ge` on 9.0+. They exist only as `::tcl::mathop::*` and are callable bare only after `namespace path` or `namespace import`; `+ 1 2` is accepted without either.
- **`http`.** A package name, not a command.
- **`filename`.** A `source` command-form name that leaks into the command list.
- **`disabled_in_irules`.** An internal marker spec with `surface: ALL_TCL`.
- **`pkg_mkindex`.** The real command is `pkg_mkIndex`; the lower-case spec is an extra.
- **`noop`, `lstring`, `lgen`, `gettimes` and about 120 other `test*` commands** from Tcl's C test binary (`tclTest.c`). They are offered in every Tcl dialect, though they exist only in the `tcltest` executable.
- **`memory`** exists only in `TCL_MEM_DEBUG` builds.
- **`registry`** is Windows-only and needs `package require registry`.
- **`bgerror`** is a handler the application defines, not a built-in.
- **`next`** exists only inside methods, but is accepted as a global command.

**Commands accepted in releases that lack them:**
- **`unload` in 8.4.** It was added in 8.5.
- **`msgcat`:**
  - `mcexists`, `mcforgetpackage`, `mcloadedlocales`, `mcpackageconfig` and `mcpackagelocale` in 8.4 and 8.5;
  - `mcflset` and `mcflmset` in 8.4;
  - `mcn`, `mcpackagenamespaceget` and `mcutil` in 8.4–8.6.

  Each depends on the msgcat version bundled with the release.
- **`http` 2.10 commands in 8.4–8.6:** `responseBody`, `responseCode`, `responseHeaders`, `responseHeaderValue`, `responseInfo`, `responseLine`, `requestHeaders`, `requestHeaderValue`, `requestLine`, `reasonPhrase` and `postError`; also `quoteString` and `registerError` in 8.4 and 8.5.
- **`oo::Helpers::classvariable`, `link` and `mymethod` in 8.6.** They were added in 9.0.
- **`tcltest::bytestring` in 9.0 and 9.1.**

**Subcommands that are not callable:** `oo::class new` and `oo::class createWithNamespace`, and `createWithNamespace` on `oo::abstract`, `oo::configurable` and `oo::singleton`. These are unexported methods, so Tcl says "unknown method … must be create or destroy".

**Arity too loose:**
- **Arity that changed between releases, stored once:**
  - `catch script res opts` in 8.4 (one variable only);
  - `close ch dir` and `chan close ch dir` in 8.4/8.5 (direction added in 8.6);
  - `encoding convertfrom|convertto` with options in 8.x;
  - `file delete`, `file mkdir`, `glob`, `global` and `variable` with no arguments in 8.4/8.5;
  - `file stat|lstat name` without a variable in 8.x;
  - `linsert list index` in 8.4/8.5;
  - `lrepeat n` in 8.5;
  - `package require|present` with several versions in 8.4;
  - `source -encoding e file` in 8.4, and `source x y` in 8.4–8.6;
  - `http::register` with 4 arguments in 8.x;
  - `msgcat::mcpreferences` with arguments in 8.x;
  - `unicode tonf* -profile p` in 9.1.
- **Option-value pairs not modelled:** an odd argument count is accepted for `fconfigure`, `chan configure`, `fcopy`, `chan copy`, `clock add`, `clock format`, `clock scan`, `zlib gunzip`, `http::formatQuery` and `pkg::create`.
- **Maximum or minimum wrong:**
  - `chan puts` with more than 3 arguments;
  - `string compare` and `string equal` with more than 5;
  - `string is` with more than 5;
  - `string map` and `string match` with more than 3;
  - `regexp` with 1;
  - `if` with none;
  - Tk `bell` with more than 3 (`bell -displayof . -nice extra`).

## Definitions missing

`tcl::tm::add`, `tcl::tm::list` and `tcl::tm::remove` (documented in `tm.n` since 8.5), `http::mapReply`, `pkg_compareExtension`, and 8.4's `tclLdAout` are reported as unresolved (W123).

## Not checked

- **tcllib:** no tcllib available in this container.
- **Jim:** the harness needs `interp` and namespaces.
- **Tk arity:** only checked by hand for `destroy` and `bell`.
- **Tk releases other than 9.0.3.**
- **`zlib` subcommands on 9.0 and 9.1:** tclsh 9.0.4 and 9.1.0 segfault when probed with dummy arguments.
- **Options and value grammars:** the probe only counts arguments.
- **Internal helpers** (`tcl::clock::*`, history internals, `safe::` internals, `auto_mkindex_parser::*`, the assembler): reported as unresolved, which is reasonable.

## Causes and fixes

- **Single arity across releases.** Many specs carry one `arity` where the command changed; `arity_windows` exists but is little used. Every entry under "arity that changed between releases" needs a window.
- **No parity or shape rules** for option-value commands. `Arity::stepped` exists but is unused for these.
- **Package commands are not tied to the package version** bundled with each release, so `http` 2.10 and `msgcat` 1.7 commands appear in 8.x.
- **Names that are not commands** (namespaces, packages, form names, markers, test-binary commands, debug-build and platform commands, application hooks) are registered with `surface: ALL_TCL`.
- **Unexported TclOO methods** are listed as subcommands.

The registry review's plan item 116 (a usage-conformance vector from each interpreter's "wrong # args" line) would have caught most of the arity faults. The harness here generalises it to existence, subcommands and arity per release. Plan item 123 makes it a gate and fixes the errors above.
