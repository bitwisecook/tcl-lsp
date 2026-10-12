# naming.return-instruction-original-result-owners

Kind: `native-observation`

## Problem statement

A semantic Return or Error result can be confused with the exact retained message, option dictionary, stack alias or interpreter owner. Observing after a getter or reset may change the very cache and reference counts being tested.

## Question

Which original result, return-options and error-line objects survive the immediate, syntax, capture and reset instruction windows?

## Conclusion

The C8.5–9.1 probe records exact original identity/reference/cache flags at each named instruction window, with an explicit retained message lease across reset. These private owner observations do not certify unrelated Return operands or normal source completion; C8.4 and Jim are not observed here.

## Scope

Four C builds, matching private headers and the exact probe.c observer order. The observations.tsv columns are defined by the producer; no borrowed Native owner.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact build/provider fields retained in the attached original receipt; unavailable fields are not inferred.. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact build/provider fields retained in the attached original receipt; unavailable fields are not inferred.. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact build/provider fields retained in the attached original receipt; unavailable fields are not inferred.. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact build/provider fields retained in the attached original receipt; unavailable fields are not inferred.. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [runtime/rust/tests/data/native_return_instructions/manifest.json](../../../../runtime/rust/tests/data/native_return_instructions/manifest.json). SHA-256 `5ae70175d2666f506f06924b959bfa56f305979aa5cfe99e20a56150c98cecb9`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [runtime/rust/tests/data/native_return_instructions/observations.tsv](../../../../runtime/rust/tests/data/native_return_instructions/observations.tsv). SHA-256 `1e462d1cc22e6d41bf4e2848d29bd414a4ecaca678d58cbcd309e1bd9c0e9da6`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (input): [runtime/rust/tests/data/native_return_instructions/probe.c](../../../../runtime/rust/tests/data/native_return_instructions/probe.c). SHA-256 `881e6ce2c426772db97409add707e1a25d046544e37d4e7139e5bde91768cb00`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
