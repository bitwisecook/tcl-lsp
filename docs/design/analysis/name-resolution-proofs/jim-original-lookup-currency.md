# Which original Jim command and variable cached-name hits survive replacement, rename, frame change, unset/recreate and active command deletion?

Proof ID: `naming.jim.original-lookup-currency`

## Problem statement

Equal name bytes can have different retained lookup state. A Jim command-name cache can refer to a replaced or retired command, and a variable cache can carry an earlier frame identifier. Dispatch must validate current epoch, frame and live-worker ownership.

## Question

Which original Jim command and variable cached-name hits survive replacement, rename, frame change, unset/recreate and active command deletion?

## Provider answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 not recorded | not-tested | No observation for this question. |
| tcl8.5 not recorded | not-tested | No observation for this question. |
| tcl8.6 not recorded | not-tested | No observation for this question. |
| tcl9.0 not recorded | not-tested | No observation for this question. |
| tcl9.1 not recorded | not-tested | No observation for this question. |
| jim not recorded by this retained invocation | observed | Original caches are checked against actual procedure epoch, frame identity and live command state; active deletion distinguishes original cached head from a fresh equal-byte head. |
| bigip not recorded | not-tested | No appliance observation for this question. |

## Conclusion

The retained Jim probe separates original cached-name observations from reached lookups and records their epoch, frame and liveness boundaries. These rows do not grant a compiler cache, current command token or variable cell merely from equal bytes.

## Scope

Original Jim native headers and reached cache lookups; C and BIG-IP are not tested by this Jim-only probe.

## Evidence and reconfirmation

The [original source](../../../../rust/tcl-syntax/tests/data/native_jim_lookup/probe.c) and [capture manifest](../../../../rust/tcl-syntax/tests/data/native_jim_lookup/manifest.json) retain the source and provider fingerprints. Peer observation files contain the complete recorded outputs. These are retained observations; no new capture is asserted by this page.

Reconfirmation requires the exact provider release, retained source program and observer window. Use the exact source, provider versions and hashed build/observer purposes recorded in the manifest. Physical probes need matching native internal headers and libraries; no generic shell command reproduces them. Retained observations are not a fresh reconfirmation; failed or missing providers supply no passing assertion.

The Rust fixture assertions validate the shared implementation against the retained rows; they are distinct from the original native capture.
