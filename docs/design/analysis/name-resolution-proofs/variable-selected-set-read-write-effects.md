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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 8.4.20.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl9.1 9.1.0.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

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

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_set_effect_forms](../../../../rust/tcl-registry/tests/data/native_set_effect_forms)
