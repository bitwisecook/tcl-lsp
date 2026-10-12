# naming.package.files-original-list-lifetime

Kind: `native-observation`

## Problem statement

A package-files query can publish a retained List whose lifetime outlasts forgetting the package. Reconstructing a new equal List would match text while losing the original header/reference transitions.

## Question

Which original List header/reference windows occur across package-files creation, queries, an external hold and package forget?

## Conclusion

The two captured C9 releases preserve the created/query/held/forgotten/retained List windows. The retained external value survives package removal with its separately recorded references. Older C, Jim and BIG-IP were not measured for this object-lifetime probe.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.5.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.6.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-2-row-3`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "created\t0\t1\tlist\nquery\t0\t1\t2\nheld\t0\t1\t3\nforgotten\t0\t1\t1\nretained\t0\t1\t2\n",
  "stderr": ""
}
```

`9.0.4.tsv` (`file-0`):

```json
{
  "original_table": "created\t0\t1\tlist\nquery\t0\t1\t2\nheld\t0\t1\t3\nforgotten\t0\t1\t1\nretained\t0\t1\t2\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-2-row-4`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "created\t0\t1\tlist\nquery\t0\t1\t2\nheld\t0\t1\t3\nforgotten\t0\t1\t1\nretained\t0\t1\t2\n",
  "stderr": ""
}
```

`9.1.0.tsv` (`file-1`):

```json
{
  "original_table": "created\t0\t1\tlist\nquery\t0\t1\t2\nheld\t0\t1\t3\nforgotten\t0\t1\t1\nretained\t0\t1\t2\n"
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_package_files/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_package_files/9.0.4.tsv). SHA-256 `a65b1e3aec69880719819434939dd0b72d9498956a84f3a73da21d9f49a75ba8`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_package_files/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_package_files/9.1.0.tsv). SHA-256 `a65b1e3aec69880719819434939dd0b72d9498956a84f3a73da21d9f49a75ba8`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_package_files/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_files/manifest.json). SHA-256 `302172f7eafd627e8002c26ac554fbabcded5d6fdf77803260321d395736039c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2-row-3` (provider): [rust/tcl-registry/tests/data/native_package_files/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_files/manifest.json). SHA-256 `302172f7eafd627e8002c26ac554fbabcded5d6fdf77803260321d395736039c`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-2-row-4` (provider): [rust/tcl-registry/tests/data/native_package_files/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_files/manifest.json). SHA-256 `302172f7eafd627e8002c26ac554fbabcded5d6fdf77803260321d395736039c`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-5` (input): [rust/tcl-registry/tests/data/native_package_files/probe.c](../../../../rust/tcl-registry/tests/data/native_package_files/probe.c). SHA-256 `a464aeaf1617a2aed1a156409f2c3d61ebe283ba9446f67536f5a1c4342f0388`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
