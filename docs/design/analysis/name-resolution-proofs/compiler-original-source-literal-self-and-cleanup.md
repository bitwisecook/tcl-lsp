# naming.compiler.original-source-literal-self-and-cleanup

Kind: `native-observation`

## Problem statement

A compiled source object can itself appear in its literal array and retain a self-reference. Deleting its command can release that cycle at a different frontier from execution.

## Question

What original source/literal identity, type and reference windows do the normal, larger Tcl 8.4 and deletion-cleanup probes retain?

## Conclusion

Each retained probe preserves its own compilation and cleanup windows, including actual self-literal identity. The larger Tcl 8.4 and cleanup programs are separate sources and cannot be treated as copies of the normal probe. No unobserved source-channel conversion or Rust cycle behavior is claimed.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4 source purpose and original archive digest; no fresh linked-version query; 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-4-row-5`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "compile_error": "",
  "runs": [
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=4\tcount=1\tself0=1\tlit0_type=bytecode\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=4\tcount=1\tself0=1\tlit0_type=bytecode\n",
      "stderr": ""
    }
  ]
}
```

`cleanup.json` (`file-1`):

```json
{
  "runs": [
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=4\tcount=1\tself0=1\tlit0_type=bytecode\nafter-interp-delete\ttype=bytecode\trefs=1\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=4\tcount=1\tself0=1\tlit0_type=bytecode\nafter-interp-delete\ttype=bytecode\trefs=1\n",
      "stderr": ""
    }
  ]
}
```

`larger84.json` (`file-3`):

```json
{
  "runs": [
    {
      "exit": 0,
      "stdout": "before\trefs=3\ttype=none\nafter\trefs=4\tcount=1\tself0=1\nafter-delete\trefs=1\ttype=bytecode\nreleased-external-originals\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\trefs=3\ttype=none\nafter\trefs=4\tcount=2\tself0=1\tself1=0\nafter-delete\trefs=1\ttype=bytecode\nreleased-external-originals\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\trefs=3\ttype=none\nafter\trefs=4\tcount=2\tself0=0\tself1=1\nafter-delete\trefs=1\ttype=bytecode\nreleased-external-originals\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\trefs=3\ttype=none\nafter\trefs=4\tcount=3\tself0=1\tself1=0\tself2=0\nafter-delete\trefs=1\ttype=bytecode\nreleased-external-originals\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\trefs=3\ttype=none\nafter\trefs=4\tcount=3\tself0=0\tself1=1\tself2=0\nafter-delete\trefs=1\ttype=bytecode\nreleased-external-originals\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\trefs=3\ttype=none\nafter\trefs=4\tcount=3\tself0=0\tself1=0\tself2=1\nafter-delete\trefs=1\ttype=bytecode\nreleased-external-originals\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\trefs=3\ttype=none\nafter\trefs=4\tcount=8\tself0=0\tself1=0\tself2=0\tself3=0\tself4=1\tself5=0\tself6=0\tself7=0\nafter-delete\trefs=1\ttype=bytecode\nreleased-external-originals\n",
      "stderr": ""
    }
  ]
}
```

`observations.tsv` (`file-10`):

```json
{
  "provider_rows": [
    "8.4.20\t78\t3\t4\t1",
    "8.4.20\t\t3\t4\t1"
  ]
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-4-row-6`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "compile_error": "",
  "runs": [
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=2\nafter\tcode=0\ttype=bytecode\trefs=2\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=2\nafter\tcode=0\ttype=bytecode\trefs=2\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    }
  ]
}
```

`observations.tsv` (`file-10`):

```json
{
  "provider_rows": [
    "8.5.19\t78\t2\t2\t0",
    "8.5.19\t\t2\t2\t0"
  ]
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-4-row-7`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_error": "",
  "runs": [
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=3\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=3\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    }
  ]
}
```

`observations.tsv` (`file-10`):

```json
{
  "provider_rows": [
    "8.6.18\t78\t3\t3\t0",
    "8.6.18\t\t3\t3\t0"
  ]
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-4-row-8`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_error": "",
  "runs": [
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=3\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=3\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    }
  ]
}
```

`observations.tsv` (`file-10`):

```json
{
  "provider_rows": [
    "9.0.4\t78\t3\t3\t0",
    "9.0.4\t\t3\t3\t0"
  ]
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-4-row-9`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_error": "",
  "runs": [
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=3\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    },
    {
      "exit": 0,
      "stdout": "before\ttype=none\trefs=3\nafter\tcode=0\ttype=bytecode\trefs=3\tcount=1\tself0=0\tlit0_type=none\n",
      "stderr": ""
    }
  ]
}
```

`observations.tsv` (`file-10`):

```json
{
  "provider_rows": [
    "9.1.0\t78\t3\t3\t0",
    "9.1.0\t\t3\t3\t0"
  ]
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.c](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.c)
- [rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.json)
- [rust/tcl-runtime-api/tests/data/native_source_literals/larger.c](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/larger.c)
- [rust/tcl-runtime-api/tests/data/native_source_literals/larger84.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/larger84.json)
- [rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json)
- [rust/tcl-runtime-api/tests/data/native_source_literals/observations.tsv](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/observations.tsv)
- [rust/tcl-runtime-api/tests/data/native_source_literals/probe.c](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/probe.c)
