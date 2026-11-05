# naming.tcloo.original-call-argv-ownership

Kind: `native-observation`

## Problem statement

Method, constructor, destructor and nextto entry can retain the original argument object while rewriting prefix positions. Inferring object ownership from the printed call would lose both frame argv and cleanup distinctions.

## Question

Which original object identities, argv counts and reference windows do the method, constructor, destructor and nextto probes actually observe?

## Conclusion

The purpose-labelled original probes retain separate frame/call/cleanup observations. Constructor, destructor and nextto rows do not share one argv layout. Tcl 8.4/8.5 absence rows do not establish later TclOO entry ownership; only each captured purpose and release is covered.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`argv-manifest.json` (`file-0-row-1`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "oo\tabsent-native-core\n",
    "stderr": ""
  }
}
```

`observations.tsv` (`file-14`):

```json
{
  "provider_rows": [
    "8.4.20\tnext\too\tabsent-native-core"
  ]
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`argv-manifest.json` (`file-0-row-2`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "oo\tabsent-native-core\n",
    "stderr": ""
  }
}
```

`observations.tsv` (`file-14`):

```json
{
  "provider_rows": [
    "8.5.19\tnext\too\tabsent-native-core"
  ]
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`argv-manifest.json` (`file-0-row-3`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=4\nafter\tcode=1\targType=list\targRefs=3\ncall\tcount=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=3\ncall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3\n",
    "stderr": ""
  }
}
```

`destructor-argv-manifest.json` (`file-8-row-9`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=0\nafter\tcode=1\targType=list\targRefs=1\n",
    "stderr": ""
  }
}
```

`oo-argv-additional-manifest.json` (`file-15-row-16`):

```json
{
  "purpose": "nextto",
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=4\nafter\tcode=1\targType=list\targRefs=3\ncall\tcount=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=3\ncall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3\n",
    "stderr": ""
  }
}
```

`oo-argv-additional-manifest.json` (`file-15-row-19`):

```json
{
  "purpose": "constructor",
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=4\tsame0=0\ttype0=cmdName\trefs0=1\tsame1=0\ttype1=TclOO method name\trefs1=2\tsame2=0\ttype2=none\trefs2=1\tsame3=1\ttype3=list\trefs3=2\nafter\tcode=1\targType=list\targRefs=2\ncall\tcount=4\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=0\ttype2=none\trefs2=2\tsame3=1\ttype3=list\trefs3=2\n",
    "stderr": ""
  }
}
```

`observations.tsv` (`file-14`):

```json
{
  "provider_rows": [
    "8.6.18\tnext\tframe\tobjc=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=4",
    "8.6.18\tnext\tafter\tcode=1\targType=list\targRefs=3",
    "8.6.18\tnext\tcall\tcount=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=3",
    "8.6.18\tnext\tcall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "8.6.18\tnextto\tframe\tobjc=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=4",
    "8.6.18\tnextto\tafter\tcode=1\targType=list\targRefs=3",
    "8.6.18\tnextto\tcall\tcount=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "8.6.18\tnextto\tcall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "8.6.18\tconstructor\tframe\tobjc=4\tsame0=0\ttype0=cmdName\trefs0=1\tsame1=0\ttype1=TclOO method name\trefs1=2\tsame2=0\ttype2=none\trefs2=1\tsame3=1\ttype3=list\trefs3=2",
    "8.6.18\tconstructor\tafter\tcode=1\targType=list\targRefs=2",
    "8.6.18\tconstructor\tcall\tcount=4\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=0\ttype2=none\trefs2=2\tsame3=1\ttype3=list\trefs3=2",
    "8.6.18\tdestructor\tframe\tobjc=0",
    "8.6.18\tdestructor\tafter\tcode=1\targType=list\targRefs=1"
  ]
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`argv-manifest.json` (`file-0-row-4`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=4\nafter\tcode=1\targType=list\targRefs=3\ncall\tcount=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=3\ncall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3\n",
    "stderr": ""
  }
}
```

`destructor-argv-manifest.json` (`file-8-row-10`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=0\nafter\tcode=1\targType=list\targRefs=1\n",
    "stderr": ""
  }
}
```

`oo-argv-additional-manifest.json` (`file-15-row-17`):

```json
{
  "purpose": "nextto",
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=4\nafter\tcode=1\targType=list\targRefs=3\ncall\tcount=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=3\ncall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3\n",
    "stderr": ""
  }
}
```

`oo-argv-additional-manifest.json` (`file-15-row-20`):

```json
{
  "purpose": "constructor",
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=4\tsame0=0\ttype0=cmdName\trefs0=1\tsame1=0\ttype1=TclOO method name\trefs1=2\tsame2=0\ttype2=none\trefs2=1\tsame3=1\ttype3=list\trefs3=2\nafter\tcode=1\targType=list\targRefs=2\ncall\tcount=4\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=0\ttype2=none\trefs2=2\tsame3=1\ttype3=list\trefs3=2\n",
    "stderr": ""
  }
}
```

`observations.tsv` (`file-14`):

```json
{
  "provider_rows": [
    "9.0.4\tnext\tframe\tobjc=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=4",
    "9.0.4\tnext\tafter\tcode=1\targType=list\targRefs=3",
    "9.0.4\tnext\tcall\tcount=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=3",
    "9.0.4\tnext\tcall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "9.0.4\tnextto\tframe\tobjc=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=4",
    "9.0.4\tnextto\tafter\tcode=1\targType=list\targRefs=3",
    "9.0.4\tnextto\tcall\tcount=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "9.0.4\tnextto\tcall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "9.0.4\tconstructor\tframe\tobjc=4\tsame0=0\ttype0=cmdName\trefs0=1\tsame1=0\ttype1=TclOO method name\trefs1=2\tsame2=0\ttype2=none\trefs2=1\tsame3=1\ttype3=list\trefs3=2",
    "9.0.4\tconstructor\tafter\tcode=1\targType=list\targRefs=2",
    "9.0.4\tconstructor\tcall\tcount=4\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=0\ttype2=none\trefs2=2\tsame3=1\ttype3=list\trefs3=2",
    "9.0.4\tdestructor\tframe\tobjc=0",
    "9.0.4\tdestructor\tafter\tcode=1\targType=list\targRefs=1"
  ]
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`argv-manifest.json` (`file-0-row-5`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=4\nafter\tcode=1\targType=list\targRefs=3\ncall\tcount=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=3\ncall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3\n",
    "stderr": ""
  }
}
```

`destructor-argv-manifest.json` (`file-8-row-11`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=0\nafter\tcode=1\targType=list\targRefs=1\n",
    "stderr": ""
  }
}
```

`oo-argv-additional-manifest.json` (`file-15-row-18`):

```json
{
  "purpose": "nextto",
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=4\nafter\tcode=1\targType=list\targRefs=3\ncall\tcount=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=3\ncall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3\n",
    "stderr": ""
  }
}
```

`oo-argv-additional-manifest.json` (`file-15-row-21`):

```json
{
  "purpose": "constructor",
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_error": "",
  "run": {
    "exit": 0,
    "stdout": "frame\tobjc=4\tsame0=0\ttype0=cmdName\trefs0=1\tsame1=0\ttype1=TclOO method name\trefs1=2\tsame2=0\ttype2=none\trefs2=1\tsame3=1\ttype3=list\trefs3=2\nafter\tcode=1\targType=list\targRefs=2\ncall\tcount=4\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=0\ttype2=none\trefs2=2\tsame3=1\ttype3=list\trefs3=2\n",
    "stderr": ""
  }
}
```

`observations.tsv` (`file-14`):

```json
{
  "provider_rows": [
    "9.1.0\tnext\tframe\tobjc=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=4",
    "9.1.0\tnext\tafter\tcode=1\targType=list\targRefs=3",
    "9.1.0\tnext\tcall\tcount=2\tsame0=0\ttype0=none\trefs0=3\tsame1=1\ttype1=list\trefs1=3",
    "9.1.0\tnext\tcall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "9.1.0\tnextto\tframe\tobjc=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=4",
    "9.1.0\tnextto\tafter\tcode=1\targType=list\targRefs=3",
    "9.1.0\tnextto\tcall\tcount=3\tsame0=0\ttype0=none\trefs0=3\tsame1=0\ttype1=cmdName\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "9.1.0\tnextto\tcall\tcount=3\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=1\ttype2=list\trefs2=3",
    "9.1.0\tconstructor\tframe\tobjc=4\tsame0=0\ttype0=cmdName\trefs0=1\tsame1=0\ttype1=TclOO method name\trefs1=2\tsame2=0\ttype2=none\trefs2=1\tsame3=1\ttype3=list\trefs3=2",
    "9.1.0\tconstructor\tafter\tcode=1\targType=list\targRefs=2",
    "9.1.0\tconstructor\tcall\tcount=4\tsame0=0\ttype0=cmdName\trefs0=2\tsame1=0\ttype1=TclOO method name\trefs1=3\tsame2=0\ttype2=none\trefs2=2\tsame3=1\ttype3=list\trefs3=2",
    "9.1.0\tdestructor\tframe\tobjc=0",
    "9.1.0\tdestructor\tafter\tcode=1\targType=list\targRefs=1"
  ]
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

- [runtime/rust/src/frame/native_variable_names.rs](../../../../runtime/rust/src/frame/native_variable_names.rs), `frame::native_variable_names::original_frame_index_tests::indexed_original_argv_keeps_level_selection_and_retained_allocation_roles` (linked): Finite source-defined control: indexed original argv keeps level selection and retained allocation roles. Software indexing and allocation ownership; no native frame admission. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Evidence

- [runtime/rust/tests/data/native_call_argv/argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/argv-manifest.json)
- [runtime/rust/tests/data/native_call_argv/argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/argv-probe.c)
- [runtime/rust/tests/data/native_call_argv/constructor-argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/constructor-argv-probe.c)
- [runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json)
- [runtime/rust/tests/data/native_call_argv/destructor-argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/destructor-argv-probe.c)
- [runtime/rust/tests/data/native_call_argv/nextto-argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/nextto-argv-probe.c)
- [runtime/rust/tests/data/native_call_argv/observations.tsv](../../../../runtime/rust/tests/data/native_call_argv/observations.tsv)
- [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json)
