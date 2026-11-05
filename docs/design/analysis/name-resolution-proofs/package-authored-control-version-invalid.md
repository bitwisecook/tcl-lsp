# naming.package.authored-control-version-invalid

Kind: `native-observation`

## Problem statement

The explicit authored-profile consumer reaches `package prov Demo bad`. Its selected package/namespace grammar can differ from a physical newer host; an equal command spelling cannot choose the required member, arity or registration state.

## Question

What original caught result does `package prov Demo bad` produce on the separately captured Tcl8.4.20 and Tcl9.0.4 providers?

## Conclusion

Only this exact input and its recorded provider result bytes are established. A successful outer capture process does not mean the guest command succeeded. These native controls do not execute the authored Rust profile, establish a BIG-IP result, or attest native object/table identity.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The original JSON retains the exact inner source string and caught output. The complete output wrapper is unavailable. This record answers only the retained inner-command/result question and cannot claim an exact whole-input replay.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 tcl8.4.20.

Original captured control:

```json
{
  "engine": "tcl8.4.20",
  "binary_sha256": "04f39dc867adab5acc96738a8dc04e47a3674b6487e698a0f8bb690f9fe3dd85",
  "label": "version-invalid",
  "source": "package prov Demo bad",
  "source_sha256": "5a9e4e90009d3c7453d177f4c95ed4cac6d30911d1cf9b0836ac4d307b5865c6",
  "exit": 0,
  "stdout": "1|65787065637465642076657273696f6e206e756d6265722062757420676f74202262616422\n",
  "stderr": ""
}
```

The numeric prefix in stdout is the guest catch outcome, independent of the process status. The hex suffix is the original result encoding, not a physical header receipt.

### tcl8.5, tcl8.6, tcl9.1, jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl9.0 tcl9.0.4.

Original captured control:

```json
{
  "engine": "tcl9.0.4",
  "binary_sha256": "647d9627cd7c2aea30a8c629ed3386d2b06380aa096443ae1595eac9052b2553",
  "label": "version-invalid",
  "source": "package prov Demo bad",
  "source_sha256": "7c3db12fd73eb3830d427f56079b50e390f0349d3ca1cfbd5a19afe710b7242b",
  "exit": 0,
  "stdout": "1|65787065637465642076657273696f6e206e756d6265722062757420676f74202262616422\n",
  "stderr": ""
}
```

The numeric prefix in stdout is the guest catch outcome, independent of the process status. The hex suffix is the original result encoding, not a physical header receipt.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json](../../../../rust/tcl-vm/tests/data/authored_package_runtime/native-controls.json)
