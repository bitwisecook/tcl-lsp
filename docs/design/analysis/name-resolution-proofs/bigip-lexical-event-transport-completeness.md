# naming.bigip.lexical-event-transport-completeness

Kind: `native-observation`

## Problem statement

Chunked RULE_INIT/CLIENT_ACCEPTED logs may declare a complete byte-scan payload while losing chunks. Joining partial transport with complete HTTP_REQUEST output would manufacture event equivalence for missing rows.

## Question

Do the actual retained chunk inventories establish a complete lexical matrix for RULE_INIT and CLIENT_ACCEPTED?

## Conclusion

The retained inventories expose missing sequences and incomplete assembled payloads. They do not certify the complete lexical matrix for those events; complete HTTP_REQUEST and hosted-context outputs remain independent evidence. A reached marker and partial transport are preserved without treating absent chunks as observed results.

## Scope

Exact r2286lex1 chunk logs/inventory on the recorded appliance/build, with incomplete event channels explicitly excluded from complete-matrix claims.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1. Channel: Exact r2286lex1 chunk logs/inventory on the recorded appliance/build, with incomplete event channels explicitly excluded from complete-matrix claims.. Dialect: F5 TMM and captured traffic/log transport.

The retained inventories expose missing sequences and incomplete assembled payloads. They do not certify the complete lexical matrix for those events; complete HTTP_REQUEST and hosted-context outputs remain independent evidence. A reached marker and partial transport are preserved without treating absent chunks as observed results.

## Exact evidence

- `artifact-0` (limitation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/decoded/lex-log-chunk-inventory.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/decoded/lex-log-chunk-inventory.json). SHA-256 `2870ac829611d095b04482ca1f6627317c8a028ffdac6b64537653d199048a6a`. Actual chunk completeness/missing-sequence inventory; retains incomplete transport and grants no result for missing event rows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Retains actual existing execution artifacts only; no new appliance run is claimed. Reconfirmation requires the same isolated contexts and complete raw capture, not inferred expected-roster membership.
