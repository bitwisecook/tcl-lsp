# naming.package.authored-control-multi-requirement

Kind: `native-observation`

## Problem statement

The explicit authored-profile consumer reaches `package vsat 1.2 1.0 2.0`. Its selected package/namespace grammar can differ from a physical newer host; an equal command spelling cannot choose the required member, arity or registration state.

## Question

What original caught result does `package vsat 1.2 1.0 2.0` produce on the separately captured Tcl8.4.20 and Tcl9.0.4 providers?

## Conclusion

Only this exact input and its recorded provider result bytes are established. A successful outer capture process does not mean the guest command succeeded. These native controls do not execute the authored Rust profile, establish a BIG-IP result, or attest native object/table identity.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The original JSON retains the exact inner source string and caught output. Its recorded source_sha256 does not match that inner string or that string followed by a newline; the complete output wrapper is unavailable. This record answers only the retained inner-command/result question and cannot claim an exact whole-input replay.

## Provider answers

### tcl8.4

Status: `observed`. Version: tcl8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.4.

Original captured control:

```json
{
  "engine": "tcl8.4.20",
  "binary_sha256": "04f39dc867adab5acc96738a8dc04e47a3674b6487e698a0f8bb690f9fe3dd85",
  "label": "multi-requirement",
  "source": "package vsat 1.2 1.0 2.0",
  "source_sha256": "97d57d6e552d355bba6cf8b6d5f09764fbbe534b46debd168cebc17282fc5ed8",
  "exit": 0,
  "stdout": "1|77726f6e67202320617267733a2073686f756c6420626520227061636b61676520767361746973666965732076657273696f6e312076657273696f6e3222\n",
  "stderr": ""
}
```

The numeric prefix in stdout is the guest catch outcome, independent of the process status. The hex suffix is the original result encoding, not a physical header receipt.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.5.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.6.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `observed`. Version: tcl9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.0.

Original captured control:

```json
{
  "engine": "tcl9.0.4",
  "binary_sha256": "647d9627cd7c2aea30a8c629ed3386d2b06380aa096443ae1595eac9052b2553",
  "label": "multi-requirement",
  "source": "package vsat 1.2 1.0 2.0",
  "source_sha256": "e998290cceff134a3930522067e1e4b303953598a8005deb633a4706e7d6f23e",
  "exit": 0,
  "stdout": "0|31\n",
  "stderr": ""
}
```

The numeric prefix in stdout is the guest catch outcome, independent of the process status. The hex suffix is the original result encoding, not a physical header receipt.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl9.1.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json](../../../../rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json). SHA-256 `4859b6b5c25aeeb3c3e50bf18d2a98391567305cd514a51caf24285169fb0df2`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `exact-tcl8.4` (observation): [rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json](../../../../rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json). SHA-256 `4859b6b5c25aeeb3c3e50bf18d2a98391567305cd514a51caf24285169fb0df2`. JSON pointer `/5`. Exact original program and caught result/native process streams.
- `exact-tcl9.0` (observation): [rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json](../../../../rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json). SHA-256 `4859b6b5c25aeeb3c3e50bf18d2a98391567305cd514a51caf24285169fb0df2`. JSON pointer `/23`. Exact original program and caught result/native process streams.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted. The original JSON retains the exact inner source string and caught output. Its recorded source_sha256 does not match that inner string or that string followed by a newline; the complete output wrapper is unavailable. This record answers only the retained inner-command/result question and cannot claim an exact whole-input replay.
