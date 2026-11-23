# Fourteen tricky areas of Tcl: audit summary

**Question:** for each area of Tcl that had not yet been studied, how well do the engines, the registry, the analyser, the LSP and the tools handle it, where is the code duplicated, and what should the design be?

**Method:**
- Six audits, each with fixtures run under tclsh 8.4–9.1, Jim, the VM and the runtime, a code census and a design page.
- The audits measured the analyser through the cached MCP build, which lags the source. **Every analyser and optimiser finding was then re-run on the source at `b0311765`.** Each rewrite was executed before and after under tclsh ([`tcl-areas-head/`](handover/oo-dispatch-probe/tcl-areas-head/)).
- Findings that do not reproduce on the current source are listed as such below, and the plan treats them as regression fixtures, not fixes.

| Area | Design page | Fixtures |
| --- | --- | --- |
| Traces | [traces](traces-design.md) | `traces/` (23) |
| Special and global variables | [special variables](special-variables-design.md) | `special-variables/` (20) |
| Coroutines and `tailcall` | [coroutines](coroutines-design.md) | `coroutines/` (13) |
| Event loop and deferred code | [event loop](event-loop-design.md) | `event-loop/` (17) |
| Errors and return codes | [error handling](error-handling-design.md) | `errors-returns/` (32) |
| `unknown` and autoloading | [unknown and autoload](unknown-autoload-design.md) | `unknown-autoload/` (16) |
| Multiple interpreters | [interpreters](interps-design.md) | `interps/` (16) |
| `exec` and pipelines | [exec](exec-design.md) | `exec-pipes/` (10) |
| Threads and newer subsystems | [threads and subsystems](threads-subsystems-design.md) | `threads-subsystems/` (11) |
| Encodings and channels | [encodings and channels](encodings-channels-design.md) | `encodings-channels/` (15 + 9 raw) |
| Regular expressions and patterns | [regex and patterns](regex-patterns-design.md) | `regex-patterns/` (16) |
| Tcl 8.6 → 9.0/9.1 changes | [Tcl 9 changes](tcl9-changes-design.md) | `tcl9-changes/` (38) |
| Comments and continuations | [comments and continuations](comments-continuations-design.md) | `comments-continuations/` (27) |
| Jim | [Jim](jim-design.md) | `jim/` (23) |

## Verdict

- **The engines are right in the core of most areas:**
  - completion codes, `try` and `catch`, `return` options;
  - coroutine semantics, the event loop and timer order;
  - `unknown` and `namespace unknown` dispatch;
  - trace firing;
  - the regular-expression engine;
  - script segmentation, formatting and minifying.
- **Their gaps are whole subsystems and a few wrong answers:**
  - `exec` passes pipeline operators to the program as ordinary arguments;
  - channel reads ignore encoding and translation;
  - there is no `chan` in the VM;
  - the runtime lacks cross-interpreter aliases, `share`, `transfer` and limits;
  - the VM panics on a lone surrogate;
  - POSIX `errorCode` is `NONE` after filesystem errors;
  - neither engine can run Jim.
- **The registry describes most of these areas well.** It lacks:
  - effects for trace firing, coroutine suspension and completion-code interception;
  - one deferral descriptor;
  - interpreter selectors;
  - an `exec` pipeline grammar;
  - Thread specs;
  - value grammars for channel options.
- **The analyser's faults cluster in a few families:**
  - W210 where a command writes a variable implicitly (`catch` result variables, `errorCode`, `switch -matchvar`, nested `info exists`);
  - W214 on fixed-signature callbacks (every trace callback, ensemble `-unknown` handlers);
  - W123 where a command is created by something other than `proc` (a coroutine inside a substitution, `auto_index` entries);
  - W241 on generators;
  - missing checks (exec grammar, redirection injection, Thread without `package require`, `break` leaving a procedure, `#` lines in `switch` data).
- **Engine code is duplicated** in every area audited:
  - coroutines (1,245 and 1,480 lines);
  - traces (about 5,500 and 5,000);
  - interpreters (about 4,000 and 4,250);
  - `try` (three interpreters plus Jim's);
  - the event-loop driver;
  - about ten Jim modules.

  Each pair has only argument decoding or a small kernel in common.

## Wrong rewrites by the optimiser on the current source

Each was run before and after under tclsh:

- `regsub -all {\A.} abc X` and related restart-context folds (`r14`; tclsh `XXX`, rewrite `Xbc`): the regex engine keeps left context after a match.
- A seeded `errorInfo`/`errorCode` forwarded across `catch {error …}` (`sv14`, `sv17`; tclsh `x NONE`, rewrite `seeded seeded`).
- `string length abc` folded although an execution trace on `string` prints on every call (`tr20`; the trace output disappears).
- `[list … \xc0\x80 …]` turned into a braced literal of raw characters (O116, `e03`), which changes `encoding convertfrom` results under 8.6.
- `set x [expr {1 + 2}]` folded to `set x 3`, which changes what `tcl::unsupported::representation` reports (`t07`). This only matters for introspection of internal representations.

**Not reproduced on the current source** (they came from the stale MCP build):
- the trace-callback folds (`tr22`, `tr23`);
- deleting `set tcl_precision 3` under 8.6 and folding the float (`sv18`, and the Tcl 9 page's precision case);
- deleting a `set auto_path` (`sv19`);
- O107 emptying a `finally` (`e27`) and deleting code after `return -level 0` (`e28`).

## Analyser findings on the current source

- **Confirmed:**
  - **Coroutines:** W123 on calls to a coroutine created inside `[…]` (`co01`, `co05`); W241 on every generator loop.
  - **Errors:** W210 on the result variable of `catch` (`e15`), on `::errorCode` after an error (`e19`) and in a `for` loop inside `catch` (`e25`).
  - **Special variables:** W210 on `errorCode` after `catch` (`sv16`); no W210 for an unqualified read in `namespace eval` under 9.0 (`sv10`).
  - **`unknown` and autoloading:** W123 on `auto_index`-defined commands (`u04`, `u14`); W214 on an ensemble `-unknown` handler (`u09`) and on every trace callback (`tr02`–`tr04`, `tr13`).
  - **Interpreters:** W120 on the auto-loaded `safe::` commands (`i09`).
  - **Regex:** W210 on `switch -matchvar`/`-indexvar` in procedures (`r11`).
  - **Missing diagnostics:** no diagnostic for exec redirection injection (`e08`), for `thread::*` without `package require Thread` (`t09`), or for `break` leaving a procedure (`e04`).
- **Not reproduced on the current source:**
  - the event-loop W210s and W220 (`ev02`, `ev08`);
  - the interpreter W123, E003 and W210 (`i03`, `i16`, `i05`);
  - the `switch -matchvar` W210 at top level (`r06`);
  - the legacy-trace W211 and W220 (`tr08`);
  - the coroutine-frame W211 (`co04`);
  - the `return -level 0` W210 (`e30`).

  These stay as regression fixtures.
- **Not re-run on the current source:** navigation, hover and rename findings (definition across interpreters, rename re-pointing an alias at a child command, `trace info` missing from references, hover ignoring the release). They come from the cached MCP build and the release server of 22 November, and the plan items start by turning them into tests.

## Cross-cutting themes

1. **Effects the registry does not state yet:**
   - trace firing as a call to the callback;
   - coroutine suspension;
   - completion-code interception for `catch`, `try`, loops and procedure boundaries;
   - one deferral descriptor (today four facts);
   - interpreter selectors on `interp` words;
   - the `exec` pipeline grammar;
   - channel-option value grammars;
   - the Thread package.

   They extend plan items 80 (registry descriptors) and 120 (one statement per fact).
2. **Implicit writes are invisible to W210:** `catch` and `try` variables, `errorInfo`/`errorCode` on the error edge, `switch -matchvar`, trace-created variables and nested `info exists`. One fix covers the family: binding's write effects reach SSA (plan items 22, 66, 101).
3. **Fixed-signature callbacks** (traces, ensemble `-unknown`, `namespace unknown`, the TclOO `unknown` method, `interp bgerror`): the registry already knows how many arguments each receives. W214 should read that (plan item 12).
4. **Commands created by something other than `proc`** (coroutines inside substitutions, `auto_index`, `interp create`, `thread::create`): created-command facts at every nested site (plan items 66, 84).
5. **Engine duplication with no trait:** coroutines, traces, interpreters, `try`, the event loop, channels and Jim modules. Each needs an engine trait or a shared kernel (plan items 20, 72, 88).
6. **Whole subsystems missing from an engine:**
   - `exec` pipelines and POSIX error codes;
   - channel decoding;
   - `chan` in the VM;
   - interpreter `share`, `transfer` and limits in the runtime;
   - `zipfs` and `tcl::process`;
   - Jim execution.

   Each needs implementing or an explicit non-goal with a catchable refusal that names the engine.
7. **The MCP tool runs a stale build** and has no Jim dialect. Results from the tools cannot be trusted as current: the MCP server should rebuild from source or report its build stamp (plan item 104).

## Plan

Items 153–199 in [plan.md](plan.md), grouped by area. Wrong answers in the engines and the five wrong rewrites come first.
