# VM C8.6 mathfunc constructor admission

Question: does the actual VM native C8.6 constructor consume the shared audited
29-name roster, allow authentic library procedure births, and preserve compiler
currency at those births?

`cmd_math::register` now receives the existing native-core bootstrap receipt
from `register_builtins_with_native_core` and consumes
`NativeBootstrapProtocol::core_mathfunc_roster`. Its fallback still derives the
implementation inventory from `mathfunc::all()`. No consumer name filter,
procedure-header exception, registry, compiler, or runtime change is included.

The focused integration regression matched one test and passed it. It uses
`Vm::with_native_core`, the real `NativeHost`, and a recording adapter delegating
to `BytecodeCompileService` through its public original-byte entry methods.
Its checks cover:

- C8.6 admission equals the entire shared 29-name roster, including absent
  `min` and `max` before library initialization.
- C8.4, C8.5, C9.0, C9.1 and Jim public mathfunc admission equals the existing
  distribution path at the same profile. Distribution C8.6 still admits
  its `min` and `max` implementations.
- Fresh empty `min` birth with raw `args` acquires a real NoOp/Present header;
  raw `{args}` instead acquires Absent/Absent. Both preserve reuse of the same
  retained source object's bytecode.
- Replacing each actual birth changes its implementation generation. Retiring
  Present recompiles the retained control body once; retiring Absent preserves
  its bytecode. This measures observable cached-body currency, not raw epochs.
- The verbatim original `init.tcl` max definition reaches `set marker after`,
  acquires an Absent/Absent header, and preserves retained-body bytecode reuse.

[fixture-origin.json](fixture-origin.json) identifies original C8.6.18
`init.tcl` lines 103–118 and pins source SHA256
`513985a7db39509e30f1bec641284dff77db22aa44fe9a0e6191eb63dcc28f35` and definition
SHA256 `4ee4778c24baa9f00b04d011f6fec9b3f31c776d1e1c3434611ca71588336d1e`.
[init.tcl.gz](init.tcl.gz) retains the unchanged full original source. The
fixture wrapper supplies only its actual namespace and the following marker.
The original C constructor and birth controls are retained separately in
[native-c86-mathfunc-bootstrap](../native-c86-mathfunc-bootstrap/README.md).

The actual matching real-library attempt did **not** reach the startup marker.
The fresh runner calls `Vm::try_init_library` with the exact original
`TCL_LIBRARY`, then attempts `puts REACHED` only on success. It exited 2 without
timeout, with empty stdout and exactly
`INIT_HOST CompilationAdmission(NativePreflightRequired)` on stderr.
[real-init-record.json](real-init-record.json) pins build argv, cargo-emitted
rlib paths/hashes, runner source/binary hashes, library selection and stream
hashes. This is an initialization attempt with a retained host refusal; it
does not establish that execution entered the library script or ran any suite.

The existing 9.1 derived-registration control matched one test and failed in
its unchanged legacy `svc.compile` call with `NativePreflightRequired`, before
VM execution. Its [log](derived-registration.log) remains explicit; no 9.1
execution pass is claimed from that control. Before moving to public integration
ingress, the focused lib-test target failed to compile with 26 errors
in test owners outside the changed files (their historical baseline was not
measured). No focused lib test ran. Its
[log](focused-tests.log) and attempted source snapshot are retained. The two
initial integration failures and their exact source/receipts are retained too:
one incorrect expectation overlooked C8.4 token visibility, and the recording
adapter initially lacked document-source compilation. The final test uses
original bytes and all its controls executed. The temporary lib regressions
were removed; its retained attempted owner matches the final owner after
removing the added test block.

[checks.json](checks.json) pins final test artifacts, exact scoped commands and
outcomes. The final [Cargo receipt](integration-build.jsonl) records the actual
passing test. [source-integration-frozen.json.gz](source-integration-frozen.json.gz)
and [source-after-build.json.gz](source-after-build.json.gz) compare 3333 declared
Rust source/manifests/lock/toolchain and selected test/fixture inputs with zero
changes. The scope excludes raw F5 data, external Cargo sources and other
non-Rust build inputs; it is not a whole-dependency byte-identity claim.
No F5 raw files were read. F5/Logical code follows the unchanged distribution
fallback; no new F5 execution evidence follows. Formatting of the three changed
Rust files passed. Command-backing, rust-check, prep-pr and broader suites were
not run.

Focused reproduction:

```sh
source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues cargo test --offline \
  -p tcl-vm --test native_mathfunc_bootstrap -- --nocapture
```

The retained [runner source](real-init-runner.rs) and
[build-and-run.py](build-and-run.py) show the exact real-library probe recipe.
Preserve this result directory; copy the recipe and Cargo receipt to a new
capture destination before replaying it.
