# naming.variable.unset-option-and-name-grammar

Kind: `native-observation`

## Problem statement

An option-shaped variable name, a repeated quiet flag or a dynamic NUL object can be interpreted differently by C and Jim. Assuming every leading dash is an option changes the target and error frontier.

## Question

What completions and native result bytes do the eleven original unset grammar programs produce on each engine?

## Conclusion

C accepts one initial -nocomplain then optional --, while Jim accepts repetitions in the captured programs. Unknown flags/prefixes are variable names and recognized lone options succeed. Binary-scan results do not establish raw String storage or compiler-word ownership.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The complete original catch/binary-scan input bytes are independently recovered in the attached wire-input artifact by exact agreement with each original source digest; this is an offline byte check and adds no native observation.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-8`):

```json
{
  "engine": "8.4",
  "case": "none",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-9`):

```json
{
  "engine": "8.4",
  "case": "unknown",
  "stdout": "1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-10`):

```json
{
  "engine": "8.4",
  "case": "defined-unknown",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-11`):

```json
{
  "engine": "8.4",
  "case": "repeated",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-12`):

```json
{
  "engine": "8.4",
  "case": "only-quiet",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-13`):

```json
{
  "engine": "8.4",
  "case": "only-end",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-14`):

```json
{
  "engine": "8.4",
  "case": "quiet-end",
  "stdout": "0\t30",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-15`):

```json
{
  "engine": "8.4",
  "case": "end-quiet",
  "stdout": "1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-16`):

```json
{
  "engine": "8.4",
  "case": "unknown-prefix",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-17`):

```json
{
  "engine": "8.4",
  "case": "nul-quiet",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-18`):

```json
{
  "engine": "8.4",
  "case": "nul-end",
  "stdout": "1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`8.4.tsv` (`file-0`):

```json
{
  "original_table": "none\t756e736574\t0\t\nunknown\t756e736574202d6261642078\t1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65\ndefined-unknown\t736574202d626164204241443b736574207820583b756e736574202d62616420783b6c697374205b696e666f20657869737473202d6261645d205b696e666f2065786973747320785d\t0\t302030\nrepeated\t736574202d6e6f636f6d706c61696e204b4545503b736574207820583b756e736574202d6e6f636f6d706c61696e202d6e6f636f6d706c61696e20783b6c697374205b696e666f20657869737473202d6e6f636f6d706c61696e5d205b696e666f2065786973747320785d\t0\t302030\nonly-quiet\t756e736574202d6e6f636f6d706c61696e\t0\t\nonly-end\t756e736574202d2d\t0\t\nquiet-end\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d6e6f636f6d706c61696e202d2d202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t0\t30\nend-quiet\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d2d202d6e6f636f6d706c61696e206d697373696e67\t1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65\nunknown-prefix\t756e736574202d6e6f636f6d70206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65\nnul-quiet\t73657420666c6167205b62696e61727920666f726d617420482a2032643665366636333666366437303663363136393665303037343631363936635d3b756e7365742024666c6167206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65\nnul-end\t73657420666c6167205b62696e61727920666f726d617420482a2032643264303037343631363936635d3b736574202d6e6f636f6d706c61696e204b4545503b756e7365742024666c6167202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-19`):

```json
{
  "engine": "8.5",
  "case": "none",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-20`):

```json
{
  "engine": "8.5",
  "case": "unknown",
  "stdout": "1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-21`):

```json
{
  "engine": "8.5",
  "case": "defined-unknown",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-22`):

```json
{
  "engine": "8.5",
  "case": "repeated",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-23`):

```json
{
  "engine": "8.5",
  "case": "only-quiet",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-24`):

```json
{
  "engine": "8.5",
  "case": "only-end",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-25`):

```json
{
  "engine": "8.5",
  "case": "quiet-end",
  "stdout": "0\t30",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-26`):

```json
{
  "engine": "8.5",
  "case": "end-quiet",
  "stdout": "1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-27`):

```json
{
  "engine": "8.5",
  "case": "unknown-prefix",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-28`):

```json
{
  "engine": "8.5",
  "case": "nul-quiet",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-29`):

```json
{
  "engine": "8.5",
  "case": "nul-end",
  "stdout": "1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`8.5.tsv` (`file-1`):

```json
{
  "original_table": "none\t756e736574\t0\t\nunknown\t756e736574202d6261642078\t1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65\ndefined-unknown\t736574202d626164204241443b736574207820583b756e736574202d62616420783b6c697374205b696e666f20657869737473202d6261645d205b696e666f2065786973747320785d\t0\t302030\nrepeated\t736574202d6e6f636f6d706c61696e204b4545503b736574207820583b756e736574202d6e6f636f6d706c61696e202d6e6f636f6d706c61696e20783b6c697374205b696e666f20657869737473202d6e6f636f6d706c61696e5d205b696e666f2065786973747320785d\t0\t302030\nonly-quiet\t756e736574202d6e6f636f6d706c61696e\t0\t\nonly-end\t756e736574202d2d\t0\t\nquiet-end\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d6e6f636f6d706c61696e202d2d202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t0\t30\nend-quiet\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d2d202d6e6f636f6d706c61696e206d697373696e67\t1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65\nunknown-prefix\t756e736574202d6e6f636f6d70206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65\nnul-quiet\t73657420666c6167205b62696e61727920666f726d617420482a2032643665366636333666366437303663363136393665303037343631363936635d3b756e7365742024666c6167206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65\nnul-end\t73657420666c6167205b62696e61727920666f726d617420482a2032643264303037343631363936635d3b736574202d6e6f636f6d706c61696e204b4545503b756e7365742024666c6167202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-30`):

```json
{
  "engine": "8.6",
  "case": "none",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-31`):

```json
{
  "engine": "8.6",
  "case": "unknown",
  "stdout": "1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-32`):

```json
{
  "engine": "8.6",
  "case": "defined-unknown",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-33`):

```json
{
  "engine": "8.6",
  "case": "repeated",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-34`):

```json
{
  "engine": "8.6",
  "case": "only-quiet",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-35`):

```json
{
  "engine": "8.6",
  "case": "only-end",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-36`):

```json
{
  "engine": "8.6",
  "case": "quiet-end",
  "stdout": "0\t30",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-37`):

```json
{
  "engine": "8.6",
  "case": "end-quiet",
  "stdout": "1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-38`):

```json
{
  "engine": "8.6",
  "case": "unknown-prefix",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-39`):

```json
{
  "engine": "8.6",
  "case": "nul-quiet",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-40`):

```json
{
  "engine": "8.6",
  "case": "nul-end",
  "stdout": "1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`8.6.tsv` (`file-2`):

```json
{
  "original_table": "none\t756e736574\t0\t\nunknown\t756e736574202d6261642078\t1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65\ndefined-unknown\t736574202d626164204241443b736574207820583b756e736574202d62616420783b6c697374205b696e666f20657869737473202d6261645d205b696e666f2065786973747320785d\t0\t302030\nrepeated\t736574202d6e6f636f6d706c61696e204b4545503b736574207820583b756e736574202d6e6f636f6d706c61696e202d6e6f636f6d706c61696e20783b6c697374205b696e666f20657869737473202d6e6f636f6d706c61696e5d205b696e666f2065786973747320785d\t0\t302030\nonly-quiet\t756e736574202d6e6f636f6d706c61696e\t0\t\nonly-end\t756e736574202d2d\t0\t\nquiet-end\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d6e6f636f6d706c61696e202d2d202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t0\t30\nend-quiet\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d2d202d6e6f636f6d706c61696e206d697373696e67\t1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65\nunknown-prefix\t756e736574202d6e6f636f6d70206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65\nnul-quiet\t73657420666c6167205b62696e61727920666f726d617420482a2032643665366636333666366437303663363136393665303037343631363936635d3b756e7365742024666c6167206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65\nnul-end\t73657420666c6167205b62696e61727920666f726d617420482a2032643264303037343631363936635d3b736574202d6e6f636f6d706c61696e204b4545503b756e7365742024666c6167202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-41`):

```json
{
  "engine": "9.0",
  "case": "none",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-42`):

```json
{
  "engine": "9.0",
  "case": "unknown",
  "stdout": "1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-43`):

```json
{
  "engine": "9.0",
  "case": "defined-unknown",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-44`):

```json
{
  "engine": "9.0",
  "case": "repeated",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-45`):

```json
{
  "engine": "9.0",
  "case": "only-quiet",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-46`):

```json
{
  "engine": "9.0",
  "case": "only-end",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-47`):

```json
{
  "engine": "9.0",
  "case": "quiet-end",
  "stdout": "0\t30",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-48`):

```json
{
  "engine": "9.0",
  "case": "end-quiet",
  "stdout": "1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-49`):

```json
{
  "engine": "9.0",
  "case": "unknown-prefix",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-50`):

```json
{
  "engine": "9.0",
  "case": "nul-quiet",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-51`):

```json
{
  "engine": "9.0",
  "case": "nul-end",
  "stdout": "1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`9.0.tsv` (`file-3`):

```json
{
  "original_table": "none\t756e736574\t0\t\nunknown\t756e736574202d6261642078\t1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65\ndefined-unknown\t736574202d626164204241443b736574207820583b756e736574202d62616420783b6c697374205b696e666f20657869737473202d6261645d205b696e666f2065786973747320785d\t0\t302030\nrepeated\t736574202d6e6f636f6d706c61696e204b4545503b736574207820583b756e736574202d6e6f636f6d706c61696e202d6e6f636f6d706c61696e20783b6c697374205b696e666f20657869737473202d6e6f636f6d706c61696e5d205b696e666f2065786973747320785d\t0\t302030\nonly-quiet\t756e736574202d6e6f636f6d706c61696e\t0\t\nonly-end\t756e736574202d2d\t0\t\nquiet-end\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d6e6f636f6d706c61696e202d2d202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t0\t30\nend-quiet\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d2d202d6e6f636f6d706c61696e206d697373696e67\t1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65\nunknown-prefix\t756e736574202d6e6f636f6d70206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65\nnul-quiet\t73657420666c6167205b62696e61727920666f726d617420482a2032643665366636333666366437303663363136393665303037343631363936635d3b756e7365742024666c6167206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65\nnul-end\t73657420666c6167205b62696e61727920666f726d617420482a2032643264303037343631363936635d3b736574202d6e6f636f6d706c61696e204b4545503b756e7365742024666c6167202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-52`):

```json
{
  "engine": "9.1",
  "case": "none",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-53`):

```json
{
  "engine": "9.1",
  "case": "unknown",
  "stdout": "1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-54`):

```json
{
  "engine": "9.1",
  "case": "defined-unknown",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-55`):

```json
{
  "engine": "9.1",
  "case": "repeated",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-56`):

```json
{
  "engine": "9.1",
  "case": "only-quiet",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-57`):

```json
{
  "engine": "9.1",
  "case": "only-end",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-58`):

```json
{
  "engine": "9.1",
  "case": "quiet-end",
  "stdout": "0\t30",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-59`):

```json
{
  "engine": "9.1",
  "case": "end-quiet",
  "stdout": "1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-60`):

```json
{
  "engine": "9.1",
  "case": "unknown-prefix",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-61`):

```json
{
  "engine": "9.1",
  "case": "nul-quiet",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-62`):

```json
{
  "engine": "9.1",
  "case": "nul-end",
  "stdout": "1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`9.1.tsv` (`file-4`):

```json
{
  "original_table": "none\t756e736574\t0\t\nunknown\t756e736574202d6261642078\t1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65\ndefined-unknown\t736574202d626164204241443b736574207820583b756e736574202d62616420783b6c697374205b696e666f20657869737473202d6261645d205b696e666f2065786973747320785d\t0\t302030\nrepeated\t736574202d6e6f636f6d706c61696e204b4545503b736574207820583b756e736574202d6e6f636f6d706c61696e202d6e6f636f6d706c61696e20783b6c697374205b696e666f20657869737473202d6e6f636f6d706c61696e5d205b696e666f2065786973747320785d\t0\t302030\nonly-quiet\t756e736574202d6e6f636f6d706c61696e\t0\t\nonly-end\t756e736574202d2d\t0\t\nquiet-end\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d6e6f636f6d706c61696e202d2d202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t0\t30\nend-quiet\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d2d202d6e6f636f6d706c61696e206d697373696e67\t1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65\nunknown-prefix\t756e736574202d6e6f636f6d70206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65\nnul-quiet\t73657420666c6167205b62696e61727920666f726d617420482a2032643665366636333666366437303663363136393665303037343631363936635d3b756e7365742024666c6167206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d706c61696e007461696c223a206e6f2073756368207661726961626c65\nnul-end\t73657420666c6167205b62696e61727920666f726d617420482a2032643264303037343631363936635d3b736574202d6e6f636f6d706c61696e204b4545503b756e7365742024666c6167202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t1\t63616e277420756e73657420222d2d007461696c223a206e6f2073756368207661726961626c65\n"
}
```

### jim

Status: `observed`. Version: jim. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-63`):

```json
{
  "engine": "jim",
  "case": "none",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-64`):

```json
{
  "engine": "jim",
  "case": "unknown",
  "stdout": "1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-65`):

```json
{
  "engine": "jim",
  "case": "defined-unknown",
  "stdout": "0\t302030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-66`):

```json
{
  "engine": "jim",
  "case": "repeated",
  "stdout": "0\t312030",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-67`):

```json
{
  "engine": "jim",
  "case": "only-quiet",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-68`):

```json
{
  "engine": "jim",
  "case": "only-end",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-69`):

```json
{
  "engine": "jim",
  "case": "quiet-end",
  "stdout": "0\t30",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-70`):

```json
{
  "engine": "jim",
  "case": "end-quiet",
  "stdout": "1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-71`):

```json
{
  "engine": "jim",
  "case": "unknown-prefix",
  "stdout": "1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-72`):

```json
{
  "engine": "jim",
  "case": "nul-quiet",
  "stdout": "0\t",
  "exit_code": 0
}
```

`provenance.json` (`file-7-row-73`):

```json
{
  "engine": "jim",
  "case": "nul-end",
  "stdout": "0\t30",
  "exit_code": 0
}
```

`jim.tsv` (`file-5`):

```json
{
  "original_table": "none\t756e736574\t0\t\nunknown\t756e736574202d6261642078\t1\t63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c65\ndefined-unknown\t736574202d626164204241443b736574207820583b756e736574202d62616420783b6c697374205b696e666f20657869737473202d6261645d205b696e666f2065786973747320785d\t0\t302030\nrepeated\t736574202d6e6f636f6d706c61696e204b4545503b736574207820583b756e736574202d6e6f636f6d706c61696e202d6e6f636f6d706c61696e20783b6c697374205b696e666f20657869737473202d6e6f636f6d706c61696e5d205b696e666f2065786973747320785d\t0\t312030\nonly-quiet\t756e736574202d6e6f636f6d706c61696e\t0\t\nonly-end\t756e736574202d2d\t0\t\nquiet-end\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d6e6f636f6d706c61696e202d2d202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t0\t30\nend-quiet\t736574202d6e6f636f6d706c61696e204b4545503b756e736574202d2d202d6e6f636f6d706c61696e206d697373696e67\t1\t63616e277420756e73657420226d697373696e67223a206e6f2073756368207661726961626c65\nunknown-prefix\t756e736574202d6e6f636f6d70206d697373696e67\t1\t63616e277420756e73657420222d6e6f636f6d70223a206e6f2073756368207661726961626c65\nnul-quiet\t73657420666c6167205b62696e61727920666f726d617420482a2032643665366636333666366437303663363136393665303037343631363936635d3b756e7365742024666c6167206d697373696e67\t0\t\nnul-end\t73657420666c6167205b62696e61727920666f726d617420482a2032643264303037343631363936635d3b736574202d6e6f636f6d706c61696e204b4545503b756e7365742024666c6167202d6e6f636f6d706c61696e3b696e666f20657869737473202d6e6f636f6d706c61696e\t0\t30\n"
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_unset_options/8.4.tsv](../../../../rust/tcl-registry/tests/data/native_unset_options/8.4.tsv). SHA-256 `aae194f3d940932bd5194ac5f190436a9b45bb554d0000b19721aab46bece310`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_unset_options/8.5.tsv](../../../../rust/tcl-registry/tests/data/native_unset_options/8.5.tsv). SHA-256 `aae194f3d940932bd5194ac5f190436a9b45bb554d0000b19721aab46bece310`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_unset_options/8.6.tsv](../../../../rust/tcl-registry/tests/data/native_unset_options/8.6.tsv). SHA-256 `aae194f3d940932bd5194ac5f190436a9b45bb554d0000b19721aab46bece310`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_unset_options/9.0.tsv](../../../../rust/tcl-registry/tests/data/native_unset_options/9.0.tsv). SHA-256 `aae194f3d940932bd5194ac5f190436a9b45bb554d0000b19721aab46bece310`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-registry/tests/data/native_unset_options/9.1.tsv](../../../../rust/tcl-registry/tests/data/native_unset_options/9.1.tsv). SHA-256 `aae194f3d940932bd5194ac5f190436a9b45bb554d0000b19721aab46bece310`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-registry/tests/data/native_unset_options/jim.tsv](../../../../rust/tcl-registry/tests/data/native_unset_options/jim.tsv). SHA-256 `2008adf85c9d4ce9b85f03f88497533204efc65752ed5d0b24e40cdbf7339d42`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (input): [rust/tcl-registry/tests/data/native_unset_options/probe.py](../../../../rust/tcl-registry/tests/data/native_unset_options/probe.py). SHA-256 `b20bf78403c5da46c656858ab515596424b3b859f6b6eed286bd3590ab2c0b85`. Exact retained input/program bytes; purpose is limited to this question.
- `file-7` (observation): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7-row-8` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/0`. Original provider/capture association at its exact selected row.
- `file-7-row-9` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/1`. Original provider/capture association at its exact selected row.
- `file-7-row-10` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/2`. Original provider/capture association at its exact selected row.
- `file-7-row-11` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/3`. Original provider/capture association at its exact selected row.
- `file-7-row-12` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/4`. Original provider/capture association at its exact selected row.
- `file-7-row-13` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/5`. Original provider/capture association at its exact selected row.
- `file-7-row-14` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/6`. Original provider/capture association at its exact selected row.
- `file-7-row-15` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/7`. Original provider/capture association at its exact selected row.
- `file-7-row-16` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/8`. Original provider/capture association at its exact selected row.
- `file-7-row-17` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/9`. Original provider/capture association at its exact selected row.
- `file-7-row-18` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/10`. Original provider/capture association at its exact selected row.
- `file-7-row-19` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/11`. Original provider/capture association at its exact selected row.
- `file-7-row-20` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/12`. Original provider/capture association at its exact selected row.
- `file-7-row-21` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/13`. Original provider/capture association at its exact selected row.
- `file-7-row-22` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/14`. Original provider/capture association at its exact selected row.
- `file-7-row-23` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/15`. Original provider/capture association at its exact selected row.
- `file-7-row-24` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/16`. Original provider/capture association at its exact selected row.
- `file-7-row-25` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/17`. Original provider/capture association at its exact selected row.
- `file-7-row-26` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/18`. Original provider/capture association at its exact selected row.
- `file-7-row-27` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/19`. Original provider/capture association at its exact selected row.
- `file-7-row-28` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/20`. Original provider/capture association at its exact selected row.
- `file-7-row-29` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/21`. Original provider/capture association at its exact selected row.
- `file-7-row-30` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/22`. Original provider/capture association at its exact selected row.
- `file-7-row-31` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/23`. Original provider/capture association at its exact selected row.
- `file-7-row-32` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/24`. Original provider/capture association at its exact selected row.
- `file-7-row-33` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/25`. Original provider/capture association at its exact selected row.
- `file-7-row-34` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/26`. Original provider/capture association at its exact selected row.
- `file-7-row-35` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/27`. Original provider/capture association at its exact selected row.
- `file-7-row-36` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/28`. Original provider/capture association at its exact selected row.
- `file-7-row-37` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/29`. Original provider/capture association at its exact selected row.
- `file-7-row-38` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/30`. Original provider/capture association at its exact selected row.
- `file-7-row-39` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/31`. Original provider/capture association at its exact selected row.
- `file-7-row-40` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/32`. Original provider/capture association at its exact selected row.
- `file-7-row-41` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/33`. Original provider/capture association at its exact selected row.
- `file-7-row-42` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/34`. Original provider/capture association at its exact selected row.
- `file-7-row-43` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/35`. Original provider/capture association at its exact selected row.
- `file-7-row-44` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/36`. Original provider/capture association at its exact selected row.
- `file-7-row-45` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/37`. Original provider/capture association at its exact selected row.
- `file-7-row-46` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/38`. Original provider/capture association at its exact selected row.
- `file-7-row-47` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/39`. Original provider/capture association at its exact selected row.
- `file-7-row-48` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/40`. Original provider/capture association at its exact selected row.
- `file-7-row-49` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/41`. Original provider/capture association at its exact selected row.
- `file-7-row-50` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/42`. Original provider/capture association at its exact selected row.
- `file-7-row-51` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/43`. Original provider/capture association at its exact selected row.
- `file-7-row-52` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/44`. Original provider/capture association at its exact selected row.
- `file-7-row-53` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/45`. Original provider/capture association at its exact selected row.
- `file-7-row-54` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/46`. Original provider/capture association at its exact selected row.
- `file-7-row-55` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/47`. Original provider/capture association at its exact selected row.
- `file-7-row-56` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/48`. Original provider/capture association at its exact selected row.
- `file-7-row-57` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/49`. Original provider/capture association at its exact selected row.
- `file-7-row-58` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/50`. Original provider/capture association at its exact selected row.
- `file-7-row-59` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/51`. Original provider/capture association at its exact selected row.
- `file-7-row-60` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/52`. Original provider/capture association at its exact selected row.
- `file-7-row-61` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/53`. Original provider/capture association at its exact selected row.
- `file-7-row-62` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/54`. Original provider/capture association at its exact selected row.
- `file-7-row-63` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/55`. Original provider/capture association at its exact selected row.
- `file-7-row-64` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/56`. Original provider/capture association at its exact selected row.
- `file-7-row-65` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/57`. Original provider/capture association at its exact selected row.
- `file-7-row-66` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/58`. Original provider/capture association at its exact selected row.
- `file-7-row-67` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/59`. Original provider/capture association at its exact selected row.
- `file-7-row-68` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/60`. Original provider/capture association at its exact selected row.
- `file-7-row-69` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/61`. Original provider/capture association at its exact selected row.
- `file-7-row-70` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/62`. Original provider/capture association at its exact selected row.
- `file-7-row-71` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/63`. Original provider/capture association at its exact selected row.
- `file-7-row-72` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/64`. Original provider/capture association at its exact selected row.
- `file-7-row-73` (provider): [rust/tcl-registry/tests/data/native_unset_options/provenance.json](../../../../rust/tcl-registry/tests/data/native_unset_options/provenance.json). SHA-256 `0d8cf9133ec220060ec05dd0798fb6ce9f5bc3613ab6909766e8e6237249d907`. JSON pointer `/rows/65`. Original provider/capture association at its exact selected row.
- `reconstructed-wire-inputs` (input): [docs/design/analysis/name-resolution-proofs/reconstructed-capture-inputs.json](../../../../docs/design/analysis/name-resolution-proofs/reconstructed-capture-inputs.json). SHA-256 `8fc1af984ce6fd8d66b87a0b24a0d7cf770f53e751ebe33e8be652b6799e4012`. JSON pointer `/groups/unset-options`. Exact original complete capture inputs recovered offline from retained body hex and catch/binary-scan wrapper, with every original whole-input digest checked. This supplies no new guest execution.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
