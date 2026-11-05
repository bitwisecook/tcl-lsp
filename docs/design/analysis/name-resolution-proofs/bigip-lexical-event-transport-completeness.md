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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### bigip

Status: `observed`. Channel: Exact r2286lex1 chunk logs/inventory on the recorded appliance/build, with incomplete event channels explicitly excluded from complete-matrix claims. Version: bigip 21.1.0.1.

The retained inventories expose missing sequences and incomplete assembled payloads. They do not certify the complete lexical matrix for those events; complete HTTP_REQUEST and hosted-context outputs remain independent evidence. A reached marker and partial transport are preserved without treating absent chunks as observed results.

## Consumer bindings

No implementation binding is claimed by this observation record.
