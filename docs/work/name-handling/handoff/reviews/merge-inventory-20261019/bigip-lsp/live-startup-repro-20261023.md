# Current simulation startup repro and owner contract

Production source `d392eb63`; exact commands, source hashes and finite results
are in `live-startup-probe-20261023.json`, with raw isolated startup output.

Minimal production-library repro:

```rust
let lib = tcl_irule_test::embedded::EmbeddedLib::materialise().unwrap();
let result = tcl_irule_test::LiveSession::new(lib.path());
```

The framework materialises, but session construction does not return within
this finite 60s probe. The actual CLI simulation also produces no report within
120s. Neither observation reaches an admission/semantic result or identifies the
hot function. Original CLI assertions and budgets are unchanged; their full
executions remain separately reported.

The earlier CI5206 heavy log reports `NativePreflightRequired` from session
construction, including both F5 CLI simulations and tcl-irule-test live tests.
That is separate from these current finite startup observations. The relevant
entry is `LiveSession::new` in the VM host crate, which constructs a native C9
core, installs the F5 source profile and a compile service, and loads the bundled
orchestrator. BIG-IP consumers must not disable native preflight or substitute a
logical runtime entry to make the tests pass.

VM/compiler/performance owners must restore the existing orchestrator startup
contract and run unchanged simulation/live assertions under its actual C9
engine, distinct F5 source grammar, and measured logical providers. Keep native
admission, event-name and variable/name policy guards intact. This agent owns
the CLI consumer, not tcl-irule-test or VM/compiler startup internals.

The original current explain-flow tests subsequently finished naturally:
3harness PASS/2FAIL in278.56s, with both simulation outputs explicitly
reporting `native compiler preflight provider required`. Neither was externally
interrupted. Full raw/summary: native-combined-20261023.{txt.gz,json}.
Two tshark paths early-returned; that is not tshark coverage.

On73c43bdb the two original simulations again finish naturally with the same
native-preflight refusal (explain-flow3/5 in243.26s); published-event-repair
receipts preserve both outputs. No VM/admission guard was bypassed.
