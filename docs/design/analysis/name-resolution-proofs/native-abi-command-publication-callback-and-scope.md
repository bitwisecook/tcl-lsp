# naming.native-abi.command-publication-callback-and-scope

Kind: `native-observation`

## Problem statement

A command delete callback can publish a new command at the old slot while an outer create/delete is still active. Removing by displayed slot after the callback can remove the wrong token; unqualified C API creation also has a different scope from script execution.

## Question

What command survives callback replacement, and which namespace receives the original unqualified C API publication?

## Conclusion

A replacement callback sees the old command. An outer creation supersedes a callback-created command; outer deletion leaves the callback replacement. The separate scope probe preserves C API publication addressing. Original token identity and scope must therefore remain separate from display spelling.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-10-row-11`):

```json
{
  "version": "8.4.20",
  "expected": "c8.4.20.txt"
}
```

`scope-manifest.json` (`file-17-row-18`):

```json
{
  "release": "8.4.20",
  "expected": "c8.4.20.scope.txt"
}
```

`c8.4.20.scope.txt` (`file-0`):

```json
{
  "original_table": "current=::a:\nunqualified-full=::p\nqualified-full=::a:::q::p\nqualified-local-token=1\nlocal-simple status=0 result=GLOBAL\nlocal-qualified status=0 result=QUALIFIED\npublication status=0 result=\nglobal-simple status=0 result=GLOBAL\nglobal-a-q status=1 result=invalid command name \"::a::q::p\"\nglobal-a-colon-q status=1 result=invalid command name \"::a:::q::p\"\n"
}
```

`c8.4.20.txt` (`file-1`):

```json
{
  "original_table": "case=0 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\nreplace-token=1\nafter exists=1 value=outer\ncase=1 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-status=0\nafter exists=0 value=none\ncase=2 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\nreplace-token=1\nafter exists=1 value=outer\ncase=3 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\ndelete-status=0\nafter exists=1 value=nested\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-10-row-12`):

```json
{
  "version": "8.5.19",
  "expected": "c8.5.19.txt"
}
```

`scope-manifest.json` (`file-17-row-19`):

```json
{
  "release": "8.5.19",
  "expected": "c8.5.19.scope.txt"
}
```

`c8.5.19.scope.txt` (`file-2`):

```json
{
  "original_table": "current=::a:\nunqualified-full=::p\nqualified-full=::a:::q::p\nqualified-local-token=1\nlocal-simple status=0 result=GLOBAL\nlocal-qualified status=0 result=QUALIFIED\npublication status=0 result=\nglobal-simple status=0 result=GLOBAL\nglobal-a-q status=1 result=invalid command name \"::a::q::p\"\nglobal-a-colon-q status=1 result=invalid command name \"::a:::q::p\"\n"
}
```

`c8.5.19.txt` (`file-3`):

```json
{
  "original_table": "case=0 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\nreplace-token=1\nafter exists=1 value=outer\ncase=1 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-status=0\nafter exists=0 value=none\ncase=2 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\nreplace-token=1\nafter exists=1 value=outer\ncase=3 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\ndelete-status=0\nafter exists=1 value=nested\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-10-row-13`):

```json
{
  "version": "8.6.18",
  "expected": "c8.6.18.txt"
}
```

`scope-manifest.json` (`file-17-row-20`):

```json
{
  "release": "8.6.18",
  "expected": "c8.6.18.scope.txt"
}
```

`c8.6.18.scope.txt` (`file-4`):

```json
{
  "original_table": "current=::a:\nunqualified-full=::p\nqualified-full=::a:::q::p\nqualified-local-token=1\nlocal-simple status=0 result=GLOBAL\nlocal-qualified status=0 result=QUALIFIED\npublication status=0 result=\nglobal-simple status=0 result=GLOBAL\nglobal-a-q status=1 result=invalid command name \"::a::q::p\"\nglobal-a-colon-q status=1 result=invalid command name \"::a:::q::p\"\n"
}
```

`c8.6.18.txt` (`file-5`):

```json
{
  "original_table": "case=0 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\nreplace-token=1\nafter exists=1 value=outer\ncase=1 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-status=0\nafter exists=0 value=none\ncase=2 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\nreplace-token=1\nafter exists=1 value=outer\ncase=3 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\ndelete-status=0\nafter exists=1 value=nested\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-10-row-14`):

```json
{
  "version": "9.0.4",
  "expected": "c9.0.4.txt"
}
```

`scope-manifest.json` (`file-17-row-21`):

```json
{
  "release": "9.0.4",
  "expected": "c9.0.4.scope.txt"
}
```

`c9.0.4.scope.txt` (`file-6`):

```json
{
  "original_table": "current=::a:\nunqualified-full=::p\nqualified-full=::a:::q::p\nqualified-local-token=1\nlocal-simple status=0 result=GLOBAL\nlocal-qualified status=0 result=QUALIFIED\npublication status=0 result=\nglobal-simple status=0 result=GLOBAL\nglobal-a-q status=1 result=invalid command name \"::a::q::p\"\nglobal-a-colon-q status=1 result=invalid command name \"::a:::q::p\"\n"
}
```

`c9.0.4.txt` (`file-7`):

```json
{
  "original_table": "case=0 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\nreplace-token=1\nafter exists=1 value=outer\ncase=1 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-status=0\nafter exists=0 value=none\ncase=2 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\nreplace-token=1\nafter exists=1 value=outer\ncase=3 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\ndelete-status=0\nafter exists=1 value=nested\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-10-row-15`):

```json
{
  "version": "9.1.0",
  "expected": "c9.1.0.txt"
}
```

`scope-manifest.json` (`file-17-row-22`):

```json
{
  "release": "9.1.0",
  "expected": "c9.1.0.scope.txt"
}
```

`c9.1.0.scope.txt` (`file-8`):

```json
{
  "original_table": "current=::a:\nunqualified-full=::p\nqualified-full=::a:::q::p\nqualified-local-token=1\nlocal-simple status=0 result=GLOBAL\nlocal-qualified status=0 result=QUALIFIED\npublication status=0 result=\nglobal-simple status=0 result=GLOBAL\nglobal-a-q status=1 result=invalid command name \"::a::q::p\"\nglobal-a-colon-q status=1 result=invalid command name \"::a:::q::p\"\n"
}
```

`c9.1.0.txt` (`file-9`):

```json
{
  "original_table": "case=0 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\nreplace-token=1\nafter exists=1 value=outer\ncase=1 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-status=0\nafter exists=0 value=none\ncase=2 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\nreplace-token=1\nafter exists=1 value=outer\ncase=3 seed=0\nbefore exists=1 value=old\ndelete-enter exists=1 value=old\ndelete-created exists=1 value=nested\ndelete-status=0\nafter exists=1 value=nested\n"
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c8.4.20.scope.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c8.4.20.scope.txt). SHA-256 `16c916b5ca264e2af1e3f459c2f522404b53341f664228773801cab1e75d84ae`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c8.4.20.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c8.4.20.txt). SHA-256 `29bc2437c8c5f4f6a177afbc70eadac4ac0f6e8c0622f1a75da5b4d0a9f4144f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c8.5.19.scope.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c8.5.19.scope.txt). SHA-256 `16c916b5ca264e2af1e3f459c2f522404b53341f664228773801cab1e75d84ae`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c8.5.19.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c8.5.19.txt). SHA-256 `29bc2437c8c5f4f6a177afbc70eadac4ac0f6e8c0622f1a75da5b4d0a9f4144f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c8.6.18.scope.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c8.6.18.scope.txt). SHA-256 `16c916b5ca264e2af1e3f459c2f522404b53341f664228773801cab1e75d84ae`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c8.6.18.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c8.6.18.txt). SHA-256 `29bc2437c8c5f4f6a177afbc70eadac4ac0f6e8c0622f1a75da5b4d0a9f4144f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c9.0.4.scope.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c9.0.4.scope.txt). SHA-256 `16c916b5ca264e2af1e3f459c2f522404b53341f664228773801cab1e75d84ae`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c9.0.4.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c9.0.4.txt). SHA-256 `29bc2437c8c5f4f6a177afbc70eadac4ac0f6e8c0622f1a75da5b4d0a9f4144f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-8` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c9.1.0.scope.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c9.1.0.scope.txt). SHA-256 `16c916b5ca264e2af1e3f459c2f522404b53341f664228773801cab1e75d84ae`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-9` (observation): [rust/tcl-cshim/tests/data/native_command_publication/c9.1.0.txt](../../../../rust/tcl-cshim/tests/data/native_command_publication/c9.1.0.txt). SHA-256 `29bc2437c8c5f4f6a177afbc70eadac4ac0f6e8c0622f1a75da5b4d0a9f4144f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-10` (observation): [rust/tcl-cshim/tests/data/native_command_publication/manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/manifest.json). SHA-256 `96e788d238c335ef911ef92e0bcf6a93d1fc5b2b3db4e52f4771464fe6ed3b58`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-10-row-11` (provider): [rust/tcl-cshim/tests/data/native_command_publication/manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/manifest.json). SHA-256 `96e788d238c335ef911ef92e0bcf6a93d1fc5b2b3db4e52f4771464fe6ed3b58`. JSON pointer `/observations/0`. Original provider/capture association at its exact selected row.
- `file-10-row-12` (provider): [rust/tcl-cshim/tests/data/native_command_publication/manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/manifest.json). SHA-256 `96e788d238c335ef911ef92e0bcf6a93d1fc5b2b3db4e52f4771464fe6ed3b58`. JSON pointer `/observations/1`. Original provider/capture association at its exact selected row.
- `file-10-row-13` (provider): [rust/tcl-cshim/tests/data/native_command_publication/manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/manifest.json). SHA-256 `96e788d238c335ef911ef92e0bcf6a93d1fc5b2b3db4e52f4771464fe6ed3b58`. JSON pointer `/observations/2`. Original provider/capture association at its exact selected row.
- `file-10-row-14` (provider): [rust/tcl-cshim/tests/data/native_command_publication/manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/manifest.json). SHA-256 `96e788d238c335ef911ef92e0bcf6a93d1fc5b2b3db4e52f4771464fe6ed3b58`. JSON pointer `/observations/3`. Original provider/capture association at its exact selected row.
- `file-10-row-15` (provider): [rust/tcl-cshim/tests/data/native_command_publication/manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/manifest.json). SHA-256 `96e788d238c335ef911ef92e0bcf6a93d1fc5b2b3db4e52f4771464fe6ed3b58`. JSON pointer `/observations/4`. Original provider/capture association at its exact selected row.
- `file-16` (input): [rust/tcl-cshim/tests/data/native_command_publication/ordering.c](../../../../rust/tcl-cshim/tests/data/native_command_publication/ordering.c). SHA-256 `a289847ededb29634c533034f88cad6411090d412109b9a1db74301a840ad339`. Exact retained input/program bytes; purpose is limited to this question.
- `file-17` (observation): [rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json). SHA-256 `db06bd00c4ae0362894984b8b2fd96d5ae3dd489d185aa65829783d05cce9c02`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-17-row-18` (provider): [rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json). SHA-256 `db06bd00c4ae0362894984b8b2fd96d5ae3dd489d185aa65829783d05cce9c02`. JSON pointer `/observations/0`. Original provider/capture association at its exact selected row.
- `file-17-row-19` (provider): [rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json). SHA-256 `db06bd00c4ae0362894984b8b2fd96d5ae3dd489d185aa65829783d05cce9c02`. JSON pointer `/observations/1`. Original provider/capture association at its exact selected row.
- `file-17-row-20` (provider): [rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json). SHA-256 `db06bd00c4ae0362894984b8b2fd96d5ae3dd489d185aa65829783d05cce9c02`. JSON pointer `/observations/2`. Original provider/capture association at its exact selected row.
- `file-17-row-21` (provider): [rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json). SHA-256 `db06bd00c4ae0362894984b8b2fd96d5ae3dd489d185aa65829783d05cce9c02`. JSON pointer `/observations/3`. Original provider/capture association at its exact selected row.
- `file-17-row-22` (provider): [rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json](../../../../rust/tcl-cshim/tests/data/native_command_publication/scope-manifest.json). SHA-256 `db06bd00c4ae0362894984b8b2fd96d5ae3dd489d185aa65829783d05cce9c02`. JSON pointer `/observations/4`. Original provider/capture association at its exact selected row.
- `file-23` (input): [rust/tcl-cshim/tests/data/native_command_publication/scope.c](../../../../rust/tcl-cshim/tests/data/native_command_publication/scope.c). SHA-256 `05146bbdf986d745924d0e0f790835ce29930564a294a30e477508c72f034187`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
