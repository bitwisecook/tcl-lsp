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

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [runtime/rust/tests/data/native_call_argv/argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/argv-manifest.json). SHA-256 `dffb571accd4cb468d7a9236a12468733903e1e8b9dc9c6a268a638d484ae9ef`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-0-row-1` (provider): [runtime/rust/tests/data/native_call_argv/argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/argv-manifest.json). SHA-256 `dffb571accd4cb468d7a9236a12468733903e1e8b9dc9c6a268a638d484ae9ef`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-0-row-2` (provider): [runtime/rust/tests/data/native_call_argv/argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/argv-manifest.json). SHA-256 `dffb571accd4cb468d7a9236a12468733903e1e8b9dc9c6a268a638d484ae9ef`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-0-row-3` (provider): [runtime/rust/tests/data/native_call_argv/argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/argv-manifest.json). SHA-256 `dffb571accd4cb468d7a9236a12468733903e1e8b9dc9c6a268a638d484ae9ef`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-0-row-4` (provider): [runtime/rust/tests/data/native_call_argv/argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/argv-manifest.json). SHA-256 `dffb571accd4cb468d7a9236a12468733903e1e8b9dc9c6a268a638d484ae9ef`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-0-row-5` (provider): [runtime/rust/tests/data/native_call_argv/argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/argv-manifest.json). SHA-256 `dffb571accd4cb468d7a9236a12468733903e1e8b9dc9c6a268a638d484ae9ef`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-6` (input): [runtime/rust/tests/data/native_call_argv/argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/argv-probe.c). SHA-256 `ed23b1280ee62d2e683bf04e5a349d680bbaec9a2604e65e6de6db263332359b`. Exact retained input/program bytes; purpose is limited to this question.
- `file-7` (input): [runtime/rust/tests/data/native_call_argv/constructor-argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/constructor-argv-probe.c). SHA-256 `c2ef9dac4a852156801a3f52fe1bbf595a86aa4c655d0413097f49babaa8f90c`. Exact retained input/program bytes; purpose is limited to this question.
- `file-8` (observation): [runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json). SHA-256 `93094cd598a36479a8f493f17ebe2a5b2e24e82b138949f8c66e19e5e9c24c3b`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-8-row-9` (provider): [runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json). SHA-256 `93094cd598a36479a8f493f17ebe2a5b2e24e82b138949f8c66e19e5e9c24c3b`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-8-row-10` (provider): [runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json). SHA-256 `93094cd598a36479a8f493f17ebe2a5b2e24e82b138949f8c66e19e5e9c24c3b`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-8-row-11` (provider): [runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/destructor-argv-manifest.json). SHA-256 `93094cd598a36479a8f493f17ebe2a5b2e24e82b138949f8c66e19e5e9c24c3b`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-12` (input): [runtime/rust/tests/data/native_call_argv/destructor-argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/destructor-argv-probe.c). SHA-256 `4592430c6cde5e6851fdea6aef1bfa0680a4ff6a33d9ed0f3b1cc4740103682b`. Exact retained input/program bytes; purpose is limited to this question.
- `file-13` (input): [runtime/rust/tests/data/native_call_argv/nextto-argv-probe.c](../../../../runtime/rust/tests/data/native_call_argv/nextto-argv-probe.c). SHA-256 `f433ea2995c54e600ac1bfef323c27de2731ef4256ea796538e9b7570352bf69`. Exact retained input/program bytes; purpose is limited to this question.
- `file-14` (observation): [runtime/rust/tests/data/native_call_argv/observations.tsv](../../../../runtime/rust/tests/data/native_call_argv/observations.tsv). SHA-256 `de3fa5426b5f917cc35a238c8840677dab71772165d41abe223aa412ed72f588`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-15` (observation): [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json). SHA-256 `158ea7c2d5d0be8de84556537ba37da4763a80e28c85a2827637f9671aeea6c4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-15-row-16` (provider): [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json). SHA-256 `158ea7c2d5d0be8de84556537ba37da4763a80e28c85a2827637f9671aeea6c4`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-15-row-17` (provider): [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json). SHA-256 `158ea7c2d5d0be8de84556537ba37da4763a80e28c85a2827637f9671aeea6c4`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-15-row-18` (provider): [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json). SHA-256 `158ea7c2d5d0be8de84556537ba37da4763a80e28c85a2827637f9671aeea6c4`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-15-row-19` (provider): [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json). SHA-256 `158ea7c2d5d0be8de84556537ba37da4763a80e28c85a2827637f9671aeea6c4`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-15-row-20` (provider): [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json). SHA-256 `158ea7c2d5d0be8de84556537ba37da4763a80e28c85a2827637f9671aeea6c4`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-15-row-21` (provider): [runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json](../../../../runtime/rust/tests/data/native_call_argv/oo-argv-additional-manifest.json). SHA-256 `158ea7c2d5d0be8de84556537ba37da4763a80e28c85a2827637f9671aeea6c4`. JSON pointer `/5`. Original provider/capture association at its exact selected row.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
