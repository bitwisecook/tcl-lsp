# VM original namespace and variable-query observations

This capture executes all 13 exact selectors against one frozen Rust source
closure. Six pass and seven fail; every original stdout/stderr, exit status,
command and source hash is retained. Before/after source hashes agree.
[Outcomes](outcomes.json), [records](records.json) and [provenance](provenance.json)
identify the exact binary, toolchain, source scope and capture runner.

| Controls | Results | Established scope |
| --- | --- | --- |
| C 8.4 namespace command rows, `01` | Pass | 12 ASCII empty/absolute-root original command observations |
| C 8.5/8.6/9.0/9.1 namespace controls, `02`–`05` | Fail | Test requires global errorCode publication after direct original object entry; observed completion code/result reach the nonroot empty creation error, but full row coverage is not established |
| Retained namespace frame and same-named recreation, `06` | Pass | Genuine allocated namespace tokens retain relative child selection while absolute lookup selects the live root tree |
| Retired namespace owner, `07` | Pass | Checked namespace lookup preserves unavailable owner as an error |
| C 8.4/8.5 and latest retained Jim variable-query source, `08`, `09`, `13` | Pass | Eight original `namespace which -variable` result rows per provider, 24 total |
| C 8.6/9.0/9.1 original query source, `10`–`12` | Fail | `CompilationAdmission(NativePreflightRequired)` before an original query result; no query conformance claim |

The namespace command entry is an actual original object vector. Its failed
completion's `-errorcode` metadata is the appropriate observable; global
`errorCode` publication is separately performed by the oracle fixture's
`catch`. Those channels must not be equated. The failed test outcomes remain
retained rather than supplying completed namespace coverage.

All fixtures construct the selected native core before namespace or variable
allocation. Namespace body and query-source execution independently install
the matching compiler. Expected rows come from the retained real interpreter
executions under `namespace-address-purpose` and
`variable-namespace-query-bases`. Jim's built-core `PATCHLEVEL 0.84` and the
external oracle's `PATCHLEVEL 0.84-9-g5bac7c9` are explicitly checked separately;
the eight query results are compared exactly. The external Jim identity is
commit `5bac7c99ad65864c87da513e22e2f01703fa4e03`.

The six Tcl STRING Unicode-NUL namespace rows per C release are outside the
ASCII control, as is errorInfo formatting. These tests do not establish raw
C-NUL behavior, native C allocation/cache equivalence, general compiler or VM
parity, or F5 outcomes. They do not use Tcltest.

Each selector uses:

```sh
CARGO_TARGET_DIR=/absolute/isolated-target \
  cargo test -p tcl-vm --lib FULL_SELECTOR \
  -- --exact --nocapture --test-threads=1
```

`capture-runner.py.txt` lists all exact selectors and configuration. New
captures require a new output directory; these observations are immutable.
