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

### tcl8.4, tcl8.5, tcl8.6, jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_package_files/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_package_files/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_package_files/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_package_files/9.1.0.tsv)
- [rust/tcl-registry/tests/data/native_package_files/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_files/manifest.json)
- [rust/tcl-registry/tests/data/native_package_files/probe.c](../../../../rust/tcl-registry/tests/data/native_package_files/probe.c)
