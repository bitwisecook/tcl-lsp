# naming.variable.unset-execution-frontiers

Kind: `native-observation`

## Problem statement

Unset can call a trace for an undefined target, ignore a callback error or stop before evaluating a later array index. A broad successful-unset assumption would miss those execution frontiers.

## Question

Which trace events, completion/result bytes and later operand evaluations occur in the six original unset scenarios?

## Conclusion

The six-engine controls separate undefined traced unset, ignored trace error, quiet missing namespaces and sequential compiled targets. The failing first receiver stops the later index. Jim trace absence is retained as an actual guest error, with no object/reference-state inference.

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
  "case": "undefined-trace",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20757d7d207b7d"
}
```

`provenance.json` (`file-7-row-9`):

```json
{
  "engine": "8.4",
  "case": "undefined-trace-quiet",
  "exit": 0,
  "stdout": "0\t30207b7d207b7b67686f7374207b7d20757d7d207b7d"
}
```

`provenance.json` (`file-7-row-10`):

```json
{
  "engine": "8.4",
  "case": "quiet-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-11`):

```json
{
  "engine": "8.4",
  "case": "quiet-array-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-12`):

```json
{
  "engine": "8.4",
  "case": "sequential-quiet",
  "exit": 0,
  "stdout": "0\t7b7b4f4e4520317d207b54574f20317d7d20302030"
}
```

`provenance.json` (`file-7-row-13`):

```json
{
  "engine": "8.4",
  "case": "element-order",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d205345434f4e44"
}
```

`8.4.tsv` (`file-0`):

```json
{
  "original_table": "undefined-trace\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b207472616365207661726961626c652067686f737420752063623b207365742063205b6361746368207b756e7365742067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b74726163652076696e666f2067686f73745d\t0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20757d7d207b7d\nundefined-trace-quiet\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b207472616365207661726961626c652067686f737420752063623b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e2067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b74726163652076696e666f2067686f73745d\t0\t30207b7d207b7b67686f7374207b7d20757d7d207b7d\nquiet-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a767d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nquiet-array-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a61286b297d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nsequential-quiet\t736574203a3a666972737420313b20736574203a3a7365636f6e6420313b20736574203a3a6576656e7473207b7d3b2070726f63206e616d65207b77686963687d207b6c617070656e64203a3a6576656e7473205b6c69737420247768696368205b696e666f20657869737473203a3a66697273745d5d3b2072657475726e207b7d7d3b2070726f632070207b7d207b756e736574202d6e6f636f6d706c61696e203a3a66697273745b6e616d65204f4e455d203a3a7365636f6e645b6e616d652054574f5d7d3b20703b206c69737420246576656e7473205b696e666f20657869737473203a3a66697273745d205b696e666f20657869737473203a3a7365636f6e645d\t0\t7b7b4f4e4520317d207b54574f20317d7d20302030\nelement-order\t736574203a3a7365656e204245464f52453b2070726f6320696478207b76616c75657d207b736574203a3a7365656e202476616c75653b2072657475726e202476616c75657d3b2070726f632070207b7d207b756e7365742061285b6964782046495253545d292061285b696478205345434f4e445d297d3b207365742063205b6361746368207b707d206d5d3b206c69737420246320246d20243a3a7365656e\t0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d205345434f4e44\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-14`):

```json
{
  "engine": "8.5",
  "case": "undefined-trace",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-15`):

```json
{
  "engine": "8.5",
  "case": "undefined-trace-quiet",
  "exit": 0,
  "stdout": "0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-16`):

```json
{
  "engine": "8.5",
  "case": "quiet-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-17`):

```json
{
  "engine": "8.5",
  "case": "quiet-array-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-18`):

```json
{
  "engine": "8.5",
  "case": "sequential-quiet",
  "exit": 0,
  "stdout": "0\t7b7b4f4e4520317d207b54574f20317d7d20302030"
}
```

`provenance.json` (`file-7-row-19`):

```json
{
  "engine": "8.5",
  "case": "element-order",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d205345434f4e44"
}
```

`8.5.tsv` (`file-1`):

```json
{
  "original_table": "undefined-trace\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e7365742067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d\nundefined-trace-quiet\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e2067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d\nquiet-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a767d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nquiet-array-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a61286b297d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nsequential-quiet\t736574203a3a666972737420313b20736574203a3a7365636f6e6420313b20736574203a3a6576656e7473207b7d3b2070726f63206e616d65207b77686963687d207b6c617070656e64203a3a6576656e7473205b6c69737420247768696368205b696e666f20657869737473203a3a66697273745d5d3b2072657475726e207b7d7d3b2070726f632070207b7d207b756e736574202d6e6f636f6d706c61696e203a3a66697273745b6e616d65204f4e455d203a3a7365636f6e645b6e616d652054574f5d7d3b20703b206c69737420246576656e7473205b696e666f20657869737473203a3a66697273745d205b696e666f20657869737473203a3a7365636f6e645d\t0\t7b7b4f4e4520317d207b54574f20317d7d20302030\nelement-order\t736574203a3a7365656e204245464f52453b2070726f6320696478207b76616c75657d207b736574203a3a7365656e202476616c75653b2072657475726e202476616c75657d3b2070726f632070207b7d207b756e7365742061285b6964782046495253545d292061285b696478205345434f4e445d297d3b207365742063205b6361746368207b707d206d5d3b206c69737420246320246d20243a3a7365656e\t0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d205345434f4e44\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-20`):

```json
{
  "engine": "8.6",
  "case": "undefined-trace",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-21`):

```json
{
  "engine": "8.6",
  "case": "undefined-trace-quiet",
  "exit": 0,
  "stdout": "0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-22`):

```json
{
  "engine": "8.6",
  "case": "quiet-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-23`):

```json
{
  "engine": "8.6",
  "case": "quiet-array-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-24`):

```json
{
  "engine": "8.6",
  "case": "sequential-quiet",
  "exit": 0,
  "stdout": "0\t7b7b4f4e4520317d207b54574f20307d7d20302030"
}
```

`provenance.json` (`file-7-row-25`):

```json
{
  "engine": "8.6",
  "case": "element-order",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354"
}
```

`8.6.tsv` (`file-2`):

```json
{
  "original_table": "undefined-trace\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e7365742067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d\nundefined-trace-quiet\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e2067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d\nquiet-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a767d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nquiet-array-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a61286b297d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nsequential-quiet\t736574203a3a666972737420313b20736574203a3a7365636f6e6420313b20736574203a3a6576656e7473207b7d3b2070726f63206e616d65207b77686963687d207b6c617070656e64203a3a6576656e7473205b6c69737420247768696368205b696e666f20657869737473203a3a66697273745d5d3b2072657475726e207b7d7d3b2070726f632070207b7d207b756e736574202d6e6f636f6d706c61696e203a3a66697273745b6e616d65204f4e455d203a3a7365636f6e645b6e616d652054574f5d7d3b20703b206c69737420246576656e7473205b696e666f20657869737473203a3a66697273745d205b696e666f20657869737473203a3a7365636f6e645d\t0\t7b7b4f4e4520317d207b54574f20307d7d20302030\nelement-order\t736574203a3a7365656e204245464f52453b2070726f6320696478207b76616c75657d207b736574203a3a7365656e202476616c75653b2072657475726e202476616c75657d3b2070726f632070207b7d207b756e7365742061285b6964782046495253545d292061285b696478205345434f4e445d297d3b207365742063205b6361746368207b707d206d5d3b206c69737420246320246d20243a3a7365656e\t0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-26`):

```json
{
  "engine": "9.0",
  "case": "undefined-trace",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-27`):

```json
{
  "engine": "9.0",
  "case": "undefined-trace-quiet",
  "exit": 0,
  "stdout": "0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-28`):

```json
{
  "engine": "9.0",
  "case": "quiet-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-29`):

```json
{
  "engine": "9.0",
  "case": "quiet-array-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-30`):

```json
{
  "engine": "9.0",
  "case": "sequential-quiet",
  "exit": 0,
  "stdout": "0\t7b7b4f4e4520317d207b54574f20307d7d20302030"
}
```

`provenance.json` (`file-7-row-31`):

```json
{
  "engine": "9.0",
  "case": "element-order",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354"
}
```

`9.0.tsv` (`file-3`):

```json
{
  "original_table": "undefined-trace\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e7365742067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d\nundefined-trace-quiet\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e2067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d\nquiet-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a767d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nquiet-array-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a61286b297d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nsequential-quiet\t736574203a3a666972737420313b20736574203a3a7365636f6e6420313b20736574203a3a6576656e7473207b7d3b2070726f63206e616d65207b77686963687d207b6c617070656e64203a3a6576656e7473205b6c69737420247768696368205b696e666f20657869737473203a3a66697273745d5d3b2072657475726e207b7d7d3b2070726f632070207b7d207b756e736574202d6e6f636f6d706c61696e203a3a66697273745b6e616d65204f4e455d203a3a7365636f6e645b6e616d652054574f5d7d3b20703b206c69737420246576656e7473205b696e666f20657869737473203a3a66697273745d205b696e666f20657869737473203a3a7365636f6e645d\t0\t7b7b4f4e4520317d207b54574f20307d7d20302030\nelement-order\t736574203a3a7365656e204245464f52453b2070726f6320696478207b76616c75657d207b736574203a3a7365656e202476616c75653b2072657475726e202476616c75657d3b2070726f632070207b7d207b756e7365742061285b6964782046495253545d292061285b696478205345434f4e445d297d3b207365742063205b6361746368207b707d206d5d3b206c69737420246320246d20243a3a7365656e\t0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-32`):

```json
{
  "engine": "9.1",
  "case": "undefined-trace",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-33`):

```json
{
  "engine": "9.1",
  "case": "undefined-trace-quiet",
  "exit": 0,
  "stdout": "0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d"
}
```

`provenance.json` (`file-7-row-34`):

```json
{
  "engine": "9.1",
  "case": "quiet-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-35`):

```json
{
  "engine": "9.1",
  "case": "quiet-array-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204f524947494e414c"
}
```

`provenance.json` (`file-7-row-36`):

```json
{
  "engine": "9.1",
  "case": "sequential-quiet",
  "exit": 0,
  "stdout": "0\t7b7b4f4e4520317d207b54574f20307d7d20302030"
}
```

`provenance.json` (`file-7-row-37`):

```json
{
  "engine": "9.1",
  "case": "element-order",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354"
}
```

`9.1.tsv` (`file-4`):

```json
{
  "original_table": "undefined-trace\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e7365742067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t31207b63616e277420756e736574202267686f7374223a206e6f2073756368207661726961626c657d207b7b67686f7374207b7d20756e7365747d7d207b7d\nundefined-trace-quiet\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e2067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t0\t30207b7d207b7b67686f7374207b7d20756e7365747d7d207b7d\nquiet-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a767d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nquiet-array-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a61286b297d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204f524947494e414c\nsequential-quiet\t736574203a3a666972737420313b20736574203a3a7365636f6e6420313b20736574203a3a6576656e7473207b7d3b2070726f63206e616d65207b77686963687d207b6c617070656e64203a3a6576656e7473205b6c69737420247768696368205b696e666f20657869737473203a3a66697273745d5d3b2072657475726e207b7d7d3b2070726f632070207b7d207b756e736574202d6e6f636f6d706c61696e203a3a66697273745b6e616d65204f4e455d203a3a7365636f6e645b6e616d652054574f5d7d3b20703b206c69737420246576656e7473205b696e666f20657869737473203a3a66697273745d205b696e666f20657869737473203a3a7365636f6e645d\t0\t7b7b4f4e4520317d207b54574f20307d7d20302030\nelement-order\t736574203a3a7365656e204245464f52453b2070726f6320696478207b76616c75657d207b736574203a3a7365656e202476616c75653b2072657475726e202476616c75657d3b2070726f632070207b7d207b756e7365742061285b6964782046495253545d292061285b696478205345434f4e445d297d3b207365742063205b6361746368207b707d206d5d3b206c69737420246320246d20243a3a7365656e\t0\t31207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354\n"
}
```

### jim

Status: `observed`. Version: jim. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Native CLI file input. The original argv names the script file; its complete catch/binary-scan bytes match the recorded whole-input SHA.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-7-row-38`):

```json
{
  "engine": "jim",
  "case": "undefined-trace",
  "exit": 0,
  "stdout": "1\t696e76616c696420636f6d6d616e64206e616d652022747261636522"
}
```

`provenance.json` (`file-7-row-39`):

```json
{
  "engine": "jim",
  "case": "undefined-trace-quiet",
  "exit": 0,
  "stdout": "1\t696e76616c696420636f6d6d616e64206e616d652022747261636522"
}
```

`provenance.json` (`file-7-row-40`):

```json
{
  "engine": "jim",
  "case": "quiet-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204e4f4e45"
}
```

`provenance.json` (`file-7-row-41`):

```json
{
  "engine": "jim",
  "case": "quiet-array-namespace",
  "exit": 0,
  "stdout": "0\t30207b7d204e4f4e45"
}
```

`provenance.json` (`file-7-row-42`):

```json
{
  "engine": "jim",
  "case": "sequential-quiet",
  "exit": 0,
  "stdout": "0\t7b7b4f4e4520317d207b54574f20317d7d20302030"
}
```

`provenance.json` (`file-7-row-43`):

```json
{
  "engine": "jim",
  "case": "element-order",
  "exit": 0,
  "stdout": "0\t31207b63616e277420756e73657420226128464952535429223a207661726961626c652069736e27742061727261797d205345434f4e44"
}
```

`jim.tsv` (`file-5`):

```json
{
  "original_table": "undefined-trace\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e7365742067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t1\t696e76616c696420636f6d6d616e64206e616d652022747261636522\nundefined-trace-quiet\t736574206576656e7473207b7d3b2070726f63206362207b6e2069206f707d207b6c617070656e64203a3a6576656e7473205b6c69737420246e20246920246f705d3b2072657475726e202d636f6465206572726f722054524143457d3b20747261636520616464207661726961626c652067686f737420756e7365742063623b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e2067686f73747d206d5d3b206c69737420246320246d20246576656e7473205b747261636520696e666f207661726961626c652067686f73745d\t1\t696e76616c696420636f6d6d616e64206e616d652022747261636522\nquiet-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a767d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204e4f4e45\nquiet-array-namespace\t736574206572726f72436f6465204f524947494e414c3b207365742063205b6361746368207b756e736574202d6e6f636f6d706c61696e203a3a6d697373696e673a3a61286b297d206d5d3b206c69737420246320246d20246572726f72436f6465\t0\t30207b7d204e4f4e45\nsequential-quiet\t736574203a3a666972737420313b20736574203a3a7365636f6e6420313b20736574203a3a6576656e7473207b7d3b2070726f63206e616d65207b77686963687d207b6c617070656e64203a3a6576656e7473205b6c69737420247768696368205b696e666f20657869737473203a3a66697273745d5d3b2072657475726e207b7d7d3b2070726f632070207b7d207b756e736574202d6e6f636f6d706c61696e203a3a66697273745b6e616d65204f4e455d203a3a7365636f6e645b6e616d652054574f5d7d3b20703b206c69737420246576656e7473205b696e666f20657869737473203a3a66697273745d205b696e666f20657869737473203a3a7365636f6e645d\t0\t7b7b4f4e4520317d207b54574f20317d7d20302030\nelement-order\t736574203a3a7365656e204245464f52453b2070726f6320696478207b76616c75657d207b736574203a3a7365656e202476616c75653b2072657475726e202476616c75657d3b2070726f632070207b7d207b756e7365742061285b6964782046495253545d292061285b696478205345434f4e445d297d3b207365742063205b6361746368207b707d206d5d3b206c69737420246320246d20243a3a7365656e\t0\t31207b63616e277420756e73657420226128464952535429223a207661726961626c652069736e27742061727261797d205345434f4e44\n"
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [runtime/rust/tests/data/native_unset_execution/8.4.tsv](../../../../runtime/rust/tests/data/native_unset_execution/8.4.tsv). SHA-256 `311412c58a5f69924a19ed2dba06b94478acce0889f3b19980b02852701c79b9`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [runtime/rust/tests/data/native_unset_execution/8.5.tsv](../../../../runtime/rust/tests/data/native_unset_execution/8.5.tsv). SHA-256 `8e753ac494d3fe678f7dffa7e2adae01d2ffc8da695d8a91b65a2b3012b28ce4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [runtime/rust/tests/data/native_unset_execution/8.6.tsv](../../../../runtime/rust/tests/data/native_unset_execution/8.6.tsv). SHA-256 `f0de24c5e4ad44aee55b521fe21cceb58d0028f86f0f7b598cba36b4086c06b4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [runtime/rust/tests/data/native_unset_execution/9.0.tsv](../../../../runtime/rust/tests/data/native_unset_execution/9.0.tsv). SHA-256 `f0de24c5e4ad44aee55b521fe21cceb58d0028f86f0f7b598cba36b4086c06b4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [runtime/rust/tests/data/native_unset_execution/9.1.tsv](../../../../runtime/rust/tests/data/native_unset_execution/9.1.tsv). SHA-256 `f0de24c5e4ad44aee55b521fe21cceb58d0028f86f0f7b598cba36b4086c06b4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [runtime/rust/tests/data/native_unset_execution/jim.tsv](../../../../runtime/rust/tests/data/native_unset_execution/jim.tsv). SHA-256 `340e9d0d2a2edfdaeea96be995211fa1d9388ca6ef0c68e7f26bee46ba6cb7f5`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (input): [runtime/rust/tests/data/native_unset_execution/probe.py](../../../../runtime/rust/tests/data/native_unset_execution/probe.py). SHA-256 `438b12613fae568c81c208f39717f3416a3ff23da705af04d2c35f249435a3de`. Exact retained input/program bytes; purpose is limited to this question.
- `file-7` (observation): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7-row-8` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/0`. Original provider/capture association at its exact selected row.
- `file-7-row-9` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/1`. Original provider/capture association at its exact selected row.
- `file-7-row-10` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/2`. Original provider/capture association at its exact selected row.
- `file-7-row-11` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/3`. Original provider/capture association at its exact selected row.
- `file-7-row-12` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/4`. Original provider/capture association at its exact selected row.
- `file-7-row-13` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/5`. Original provider/capture association at its exact selected row.
- `file-7-row-14` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/6`. Original provider/capture association at its exact selected row.
- `file-7-row-15` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/7`. Original provider/capture association at its exact selected row.
- `file-7-row-16` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/8`. Original provider/capture association at its exact selected row.
- `file-7-row-17` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/9`. Original provider/capture association at its exact selected row.
- `file-7-row-18` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/10`. Original provider/capture association at its exact selected row.
- `file-7-row-19` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/11`. Original provider/capture association at its exact selected row.
- `file-7-row-20` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/12`. Original provider/capture association at its exact selected row.
- `file-7-row-21` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/13`. Original provider/capture association at its exact selected row.
- `file-7-row-22` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/14`. Original provider/capture association at its exact selected row.
- `file-7-row-23` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/15`. Original provider/capture association at its exact selected row.
- `file-7-row-24` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/16`. Original provider/capture association at its exact selected row.
- `file-7-row-25` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/17`. Original provider/capture association at its exact selected row.
- `file-7-row-26` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/18`. Original provider/capture association at its exact selected row.
- `file-7-row-27` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/19`. Original provider/capture association at its exact selected row.
- `file-7-row-28` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/20`. Original provider/capture association at its exact selected row.
- `file-7-row-29` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/21`. Original provider/capture association at its exact selected row.
- `file-7-row-30` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/22`. Original provider/capture association at its exact selected row.
- `file-7-row-31` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/23`. Original provider/capture association at its exact selected row.
- `file-7-row-32` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/24`. Original provider/capture association at its exact selected row.
- `file-7-row-33` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/25`. Original provider/capture association at its exact selected row.
- `file-7-row-34` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/26`. Original provider/capture association at its exact selected row.
- `file-7-row-35` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/27`. Original provider/capture association at its exact selected row.
- `file-7-row-36` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/28`. Original provider/capture association at its exact selected row.
- `file-7-row-37` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/29`. Original provider/capture association at its exact selected row.
- `file-7-row-38` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/30`. Original provider/capture association at its exact selected row.
- `file-7-row-39` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/31`. Original provider/capture association at its exact selected row.
- `file-7-row-40` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/32`. Original provider/capture association at its exact selected row.
- `file-7-row-41` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/33`. Original provider/capture association at its exact selected row.
- `file-7-row-42` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/34`. Original provider/capture association at its exact selected row.
- `file-7-row-43` (provider): [runtime/rust/tests/data/native_unset_execution/provenance.json](../../../../runtime/rust/tests/data/native_unset_execution/provenance.json). SHA-256 `9c32a3b69ef549630217ca05d1d6eccd463e07f5f4c17f4ff3497c836c170f24`. JSON pointer `/rows/35`. Original provider/capture association at its exact selected row.
- `reconstructed-wire-inputs` (input): [docs/design/analysis/name-resolution-proofs/reconstructed-capture-inputs.json](../../../../docs/design/analysis/name-resolution-proofs/reconstructed-capture-inputs.json). SHA-256 `8fc1af984ce6fd8d66b87a0b24a0d7cf770f53e751ebe33e8be652b6799e4012`. JSON pointer `/groups/unset-execution`. Exact original complete capture inputs recovered offline from retained body hex and catch/binary-scan wrapper, with every original whole-input digest checked. This supplies no new guest execution.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
