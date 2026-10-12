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

Status: `observed`. Version: 8.4 source purpose and original archive digest; no fresh linked-version query; 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (input): [rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.c](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.c). SHA-256 `4d615761e0fe67ffe108236074819f536f18d00d9de9cb47893fa0de2dfad8fa`. Exact retained input/program bytes; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/cleanup.json). SHA-256 `d2c69cb4c248175e181a4da68902d3d8b1031a5a1087abed20451ee4544cf962`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (input): [rust/tcl-runtime-api/tests/data/native_source_literals/larger.c](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/larger.c). SHA-256 `d00fb2dd8dabf47d231ce7fee87da3a972f5f59d462773bde58b8a60b7f01303`. Exact retained input/program bytes; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-runtime-api/tests/data/native_source_literals/larger84.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/larger84.json). SHA-256 `daa0994ac732daac9833ca28c0162c995ec7d84c8ba8ab9030c8750688f9818f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json). SHA-256 `3786682a24436ab320722f00ab571497e100c16a679220a5b53dcddefb5263c6`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4-row-5` (provider): [rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json). SHA-256 `3786682a24436ab320722f00ab571497e100c16a679220a5b53dcddefb5263c6`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-4-row-6` (provider): [rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json). SHA-256 `3786682a24436ab320722f00ab571497e100c16a679220a5b53dcddefb5263c6`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-4-row-7` (provider): [rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json). SHA-256 `3786682a24436ab320722f00ab571497e100c16a679220a5b53dcddefb5263c6`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-4-row-8` (provider): [rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json). SHA-256 `3786682a24436ab320722f00ab571497e100c16a679220a5b53dcddefb5263c6`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-4-row-9` (provider): [rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/manifest.json). SHA-256 `3786682a24436ab320722f00ab571497e100c16a679220a5b53dcddefb5263c6`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-10` (observation): [rust/tcl-runtime-api/tests/data/native_source_literals/observations.tsv](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/observations.tsv). SHA-256 `d61746ad6c4e6a906206aef0b57a20ebdbff6efd794facc4e0ada5345a7f6dbd`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-11` (input): [rust/tcl-runtime-api/tests/data/native_source_literals/probe.c](../../../../rust/tcl-runtime-api/tests/data/native_source_literals/probe.c). SHA-256 `bbd7ec6b2a485c9ab96e6467f45215be47754583510350858f312bde5091b0ce`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
