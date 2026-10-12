# naming.variable.selected-set-read-write-effects

Kind: `native-observation`

## Problem statement

The single set command has distinct read and write forms, while incr performs both. A broad union of all variable effects would invent read callbacks for ordinary writes.

## Question

Which actual read/write trace events occur for set-read, set-write and incr on the six selected engines?

## Conclusion

Captured C writes invoke write observers, reads invoke read observers, and incr invokes both. The same source selects each release trace grammar. The recorded Jim process lacks trace and therefore supplies a guest refusal, not a positive effect-event observation.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-12-row-13`):

```json
{
  "engine": "8.4.20",
  "exit": 0
}
```

`8.4.20.tsv` (`file-1`):

```json
{
  "original_table": "write 7 0 1\nread 7 1 1\nupdate 8 2 2\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-12-row-14`):

```json
{
  "engine": "8.5.19",
  "exit": 0
}
```

`8.5.19.tsv` (`file-3`):

```json
{
  "original_table": "write 7 0 1\nread 7 1 1\nupdate 8 2 2\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-12-row-15`):

```json
{
  "engine": "8.6.18",
  "exit": 0
}
```

`8.6.18.tsv` (`file-5`):

```json
{
  "original_table": "write 7 0 1\nread 7 1 1\nupdate 8 2 2\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-12-row-16`):

```json
{
  "engine": "9.0.4",
  "exit": 0
}
```

`9.0.4.tsv` (`file-7`):

```json
{
  "original_table": "write 7 0 1\nread 7 1 1\nupdate 8 2 2\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-12-row-17`):

```json
{
  "engine": "9.1.0",
  "exit": 0
}
```

`9.1.0.tsv` (`file-9`):

```json
{
  "original_table": "write 7 0 1\nread 7 1 1\nupdate 8 2 2\n"
}
```

### jim

Status: `observed`. Version: jim. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-12-row-18`):

```json
{
  "engine": "jim",
  "exit": 0
}
```

`jim.tsv` (`file-11`):

```json
{
  "original_table": "unsupported 1 {invalid command name \"trace\"}\n"
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/8.4.20.stderr](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/8.4.20.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/8.4.20.tsv). SHA-256 `34ee06deb2143107141972e18940593580fcc69cdc25cca66185fd99356c771a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/8.5.19.stderr](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/8.5.19.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/8.5.19.tsv). SHA-256 `34ee06deb2143107141972e18940593580fcc69cdc25cca66185fd99356c771a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/8.6.18.stderr](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/8.6.18.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/8.6.18.tsv). SHA-256 `34ee06deb2143107141972e18940593580fcc69cdc25cca66185fd99356c771a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/9.0.4.stderr](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/9.0.4.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/9.0.4.tsv). SHA-256 `34ee06deb2143107141972e18940593580fcc69cdc25cca66185fd99356c771a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-8` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/9.1.0.stderr](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/9.1.0.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-9` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/9.1.0.tsv). SHA-256 `34ee06deb2143107141972e18940593580fcc69cdc25cca66185fd99356c771a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-10` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/jim.stderr](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/jim.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-11` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/jim.tsv](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/jim.tsv). SHA-256 `a4427eb6cfc5d40ba459157bb805dba811832a7f928c4cac815ab970e4d69488`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-12` (observation): [rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json). SHA-256 `233cacd721f19bfe9b9e6115125fa3f2b219fba5c6fa60cc4301cc8fccf2bb2f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-12-row-13` (provider): [rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json). SHA-256 `233cacd721f19bfe9b9e6115125fa3f2b219fba5c6fa60cc4301cc8fccf2bb2f`. JSON pointer `/runs/0`. Original provider/capture association at its exact selected row.
- `file-12-row-14` (provider): [rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json). SHA-256 `233cacd721f19bfe9b9e6115125fa3f2b219fba5c6fa60cc4301cc8fccf2bb2f`. JSON pointer `/runs/1`. Original provider/capture association at its exact selected row.
- `file-12-row-15` (provider): [rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json). SHA-256 `233cacd721f19bfe9b9e6115125fa3f2b219fba5c6fa60cc4301cc8fccf2bb2f`. JSON pointer `/runs/2`. Original provider/capture association at its exact selected row.
- `file-12-row-16` (provider): [rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json). SHA-256 `233cacd721f19bfe9b9e6115125fa3f2b219fba5c6fa60cc4301cc8fccf2bb2f`. JSON pointer `/runs/3`. Original provider/capture association at its exact selected row.
- `file-12-row-17` (provider): [rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json). SHA-256 `233cacd721f19bfe9b9e6115125fa3f2b219fba5c6fa60cc4301cc8fccf2bb2f`. JSON pointer `/runs/4`. Original provider/capture association at its exact selected row.
- `file-12-row-18` (provider): [rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/manifest.json). SHA-256 `233cacd721f19bfe9b9e6115125fa3f2b219fba5c6fa60cc4301cc8fccf2bb2f`. JSON pointer `/runs/5`. Original provider/capture association at its exact selected row.
- `file-19` (input): [rust/tcl-registry/tests/data/native_set_effect_forms/probe.tcl](../../../../rust/tcl-registry/tests/data/native_set_effect_forms/probe.tcl). SHA-256 `f1c15d67d465f3de5c9b3d7e9813605fd3e50ed934d1d7648c5a34b95241f008`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
