# Does the configured namespace unknown handler run for reached missing commands, including rooted names, while known and unreachable commands leave its counter unchanged?

Proof ID: `naming.namespace.unknown-compile-dispatch`

## Problem statement

An unresolved command can invoke a namespace unknown handler at runtime, while an unreachable command in a compiled procedure body must not trigger that handler. Treating compiler naming advice as a reached fallback call would manufacture side effects.

## Question

Does the configured namespace unknown handler run for reached missing commands, including rooted names, while known and unreachable commands leave its counter unchanged?

## Provider answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | The script rejects namespace unknown configuration. |
| tcl8.5 8.5.19 | observed | existing KNOWN 0; compiler OK 0; fallback FALLBACK 1; rootedExisting KNOWN 1; rootedFallback FALLBACK 2. |
| tcl8.6 8.6.18 | observed | existing KNOWN 0; compiler OK 0; fallback FALLBACK 1; rootedExisting KNOWN 1; rootedFallback FALLBACK 2. |
| tcl9.0 9.0.4 | observed | existing KNOWN 0; compiler OK 0; fallback FALLBACK 1; rootedExisting KNOWN 1; rootedFallback FALLBACK 2. |
| tcl9.1 9.1.0 | observed | existing KNOWN 0; compiler OK 0; fallback FALLBACK 1; rootedExisting KNOWN 1; rootedFallback FALLBACK 2. |
| jim jim | unsupported | The script rejects namespace unknown configuration. |
| bigip not recorded | not-tested | No appliance observation for this question. |

## Conclusion

The recorded C Tcl 8.5.19, 8.6.18, 9.0.4 and 9.1.0 file-script controls distinguish reached runtime fallback from known or unreachable commands. Tcl 8.4.20 and Jim reject the namespace unknown configuration in this script.

## Scope

Source namespace unknown lookup and compiler/dispatch separation.

## Evidence and reconfirmation

The [original source](../../../../rust/tcl-compiler/tests/data/native_namespace_unknown_lookup/probe.tcl) and [capture manifest](../../../../rust/tcl-compiler/tests/data/native_namespace_unknown_lookup/manifest.json) retain the source and provider fingerprints. Peer observation files contain the complete recorded outputs. These are retained observations; no new capture is asserted by this page.

Reconfirmation requires the exact provider release, retained source program and observer window. Use the exact source, provider versions and hashed build/observer purposes recorded in the manifest. Physical probes need matching native internal headers and libraries; no generic shell command reproduces them. Retained observations are not a fresh reconfirmation; failed or missing providers supply no passing assertion.

The Rust fixture assertions validate the shared implementation against the retained rows; they are distinct from the original native capture.
