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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

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

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-cshim/tests/data/native_command_publication](../../../../rust/tcl-cshim/tests/data/native_command_publication)
