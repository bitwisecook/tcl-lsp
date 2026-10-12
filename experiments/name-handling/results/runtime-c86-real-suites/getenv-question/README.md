# Original all.tcl: next missing clock worker

The unchanged original C8.6 `all.tcl` invokes `runAllTests`. The real tcltest
procedure's line2926 constructs `clock format [clock seconds] -format
"%Y-%m-%d %H:%M:%S %Z" -locale en`; line2927 evaluates it before sourcing a
test file. Real clock.tcl's format procedure gets an empty default timezone
from ParseFormatArgs, then calls GetSystemTimeZone. The first missing call is
the ordinary relative `getenv TCL_TZ` at clock.tcl:2992. `getenv TZ` at2994
would follow if that value is empty. The [existing frozen Native runner
stack](capture-02/runtime/all-stack/stdout.txt) retains that exact path and
`TCL LOOKUP COMMAND getenv`. The outer diagnostic catch exits the process zero
but records semantic failure1; no suite starts. Startup and original source
bodies are not replaced or weakened.

Original C8.6 `ClockGetenvObjCmd` at tclClock.c:1649–1688 accepts exactly one
name and calls libc `getenv(TclGetString(objv[1]))` on Unix. Missing names
return a freshly created empty string successfully. Its source comment's
capitalised `getEnv` usage and missing-variable error description do not match
the actual registration or measured implementation. C8.6's private table
registers lowercase `getenv` with the seven-worker ClockClientData pool and
NULL compiler hook. Client data is ignored by this getter; its delete callback
still participates in the shared pool lifetime.

[Fresh matching C controls](capture-02/provenance.json) measure:

| Provider | Private worker | Constructor hook | Getter channel |
| --- | --- | --- | --- |
| C8.4.20 | Absent | No worker | Five caught absence rows |
| C8.5.19 | Present | NULL | Current Unix process environment |
| C8.6.18 | Present | NULL | Current Unix process environment |
| C9.0.4 | Present | Present | Current Unix process environment |
| C9.1.0 | Present | NULL | Current Unix process environment |

For each present worker, direct object-vector controls return host-value,
empty success for missing name, host-value for a counted original containing
NUL before a suffix, raw-name-value for a literal Unix `0xff` key, and exactly
one `0xff` byte for a raw value. The script controls observe zero guest read
trace calls, process updates after normal `env` element write/unset, and the
old process value after the whole guest env array is replaced with local-only
storage. Only explicitly named benign NH_CLOCK_GETENV_* entries are queried
or output. C8.5 arity errors have errorCode NONE; C8.6/C9 arity errors have
TCL WRONGARGS. Successful calls preserve seeded UNSET.

The [first capture](capture-01/provenance.json) is retained. Its legacy `trace
variable` spelling caused C9 script errors after the five getter rows; the
second capture uses the common `trace add/remove variable` API and all five
stock scripts exit zero. All compile/control streams, actual argv, exits,
input and source/binary/library hashes remain raw. Source excerpts and complete
source hashes are in [source-excerpts.json](source-excerpts.json); validated
scope and exits are in [status.json](status.json). No Rust edit or Cargo build
was made. Jim, F5 and Windows are unmeasured.

The current registry has no getenv worker spec/admission. The selected C8.6
runtime's clock bootstrap test explicitly expects it absent. Its `::env`
array is a bootstrap snapshot, and the registry's env runtime_hook is None.
`tcl_platform::Env` is deliberately Unicode-only: `get(&str) -> Option<String>`.
NativeEnv reads current process data with `std::env::var` plus per-host Unicode
overrides; it omits non-Unicode entries when enumerating. It supplies no raw
environment byte capability, and ordinary guest env writes currently have no
host synchronisation hook. Reading that guest snapshot would be the wrong
owner for the original getter.

The [bounded proposal](proposal.md) is unimplemented. The existing command
backing xtask selects C9.0/C9.1 specs and therefore proves no C8.6 worker parity.
