# naming.source.jim-nested-direct-entry-and-prefix-effects

Kind: `native-observation`

## Problem statement

Recursive validation of nested script syntax can accidentally suppress an outer command prefix that the selected provider really executes. Conversely, admitting a complete-looking prefix can execute commands when the provider rejects the whole script at entry. Command substitutions, quoted substitutions and variable-index substitutions require the actual selected script-entry purpose rather than a single universal parse cut.

## Question

For these seven counted nested-script sources, which outer and child prefix effects occur under public Direct evaluation on C Tcl 8.4–9.1 and the measured Jim build, and how do they relate to the separate info complete query?

## Conclusion

All six measured providers execute before=1 for the four terminal nested word-weld cases, return extra characters after close-brace, leave sink/after absent and report complete=1. Even the case containing set inner 1 leaves inner absent. In the two missing-brace cases the five C providers execute before=1; Jim leaves every observed effect absent and returns unmatched "[". For bare x{ inside a command substitution, the C providers report complete=1 and succeed through after=2, with sink string bytes 785c7b; Jim reports complete=0 and rejects entry with every effect absent. These are distinct selected script-entry outcomes for these original bytes, not a general prefix-execution rule. BIG-IP is not tested.

## Scope

Seven exact counted ASCII source values reconstructed with binary format H* inside the unchanged public probe. Each uses info complete followed by uplevel #0 and independent reads of before, inner, sink and after. Six original subprocesses exit0 with empty stderr and yield 42 case rows. Physical source LF, binary source bytes, completeness results, script catch results and reached effects are independent facts. No bytecode, compiler binding, original Native object/header, lifetime, callback, frame, Host error metadata, F5/TMM or arbitrary nested grammar equivalence is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: The exact invoked ELF and captured Tcl library inputs retain before/after pins and full canonical payload joins. Configure/compiler/link and loaded shared-library identity are unrecorded. Exact original VERSION line in this process stdout.. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.. Dialect: tcl8.4.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases report complete=0 and before=1, then error with sink/after absent. Bare x{ reports complete=1 and succeeds with before=1, after=2 and sink string bytes 785c7b. This answer covers only the seven original case values.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: The exact invoked ELF and captured Tcl library inputs retain before/after pins and full canonical payload joins. Configure/compiler/link and loaded shared-library identity are unrecorded. Exact original VERSION line in this process stdout.. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.. Dialect: tcl8.5.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases report complete=0 and before=1, then error with sink/after absent. Bare x{ reports complete=1 and succeeds with before=1, after=2 and sink string bytes 785c7b. This answer covers only the seven original case values.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: The exact invoked ELF and captured Tcl library inputs retain before/after pins and full canonical payload joins. Configure/compiler/link and loaded shared-library identity are unrecorded. Exact original VERSION line in this process stdout.. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.. Dialect: tcl8.6.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases report complete=0 and before=1, then error with sink/after absent. Bare x{ reports complete=1 and succeeds with before=1, after=2 and sink string bytes 785c7b. This answer covers only the seven original case values.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: The exact invoked ELF and captured Tcl library inputs retain before/after pins and full canonical payload joins. Configure/compiler/link and loaded shared-library identity are unrecorded. Exact original VERSION line in this process stdout.. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.. Dialect: tcl9.0.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases report complete=0 and before=1, then error with sink/after absent. Bare x{ reports complete=1 and succeeds with before=1, after=2 and sink string bytes 785c7b. This answer covers only the seven original case values.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: The exact invoked ELF and captured Tcl library inputs retain before/after pins and full canonical payload joins. Configure/compiler/link and loaded shared-library identity are unrecorded. Exact original VERSION line in this process stdout.. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.. Dialect: tcl9.1.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases report complete=0 and before=1, then error with sink/after absent. Bare x{ reports complete=1 and succeeds with before=1, after=2 and sink string bytes 785c7b. This answer covers only the seven original case values.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: The exact invoked ELF and captured Tcl library inputs retain before/after pins and full canonical payload joins. Configure/compiler/link and loaded shared-library identity are unrecorded. Exact original VERSION line in this process stdout.. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.. Dialect: jim.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases and the bare x{ case report complete=0, return unmatched "[" and leave all four effects absent. This answer covers only the seven original case values.

### bigip

Status: `not-tested`. Version: not tested. Build: No actual BIG-IP process for these exact operands. Channel: No appliance capture for this question. Dialect: F5 iRules.

No actual BIG-IP response to these exact inputs is retained. Local C Tcl/Jim outcomes establish no BIG-IP or TMM behaviour.

## Exact evidence

- `original-probe` (input): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/probe.tcl](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/probe.tcl). SHA-256 `480e296d4e54c2c7cb5114859fd891e954f5f166a7d9c528560f6b82aa7680a7`. Exact original public command and post-command observation script; preserved physical source bytes.
- `original-cases` (input): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/cases.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/cases.json). SHA-256 `de7dce9b194a2b5ad2413ce289bee7a87a4fb2e1438f78944a60e0a23055c460`. Exact original input declarations. The request-only status in the 628 declaration describes that retained request and supplies no outcome.
- `original-run` (provider): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/run.py](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/run.py). SHA-256 `63e7e231854f497fa47213b7223e5f4d51583779fdfa4a1364fcb995c6ac9658`. Exact original invocation, environment, per-provider isolated file and before/after byte pin capture protocol. Its retained bytes do not supply build or loaded-library closure.
- `original-results` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/results.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/results.json). SHA-256 `786a330e7d50a8c393f597dda7ee0c8310d0beeb69860cef6667e5c16121030a`. Exact six independently closed original subprocess/result associations. Script catch codes remain distinct from the process exit.
- `tcl8.4-receipt-json` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20/receipt.json). SHA-256 `573c0fcd67329814502c02264d814bc9489dc948a04c55019f989f3bb930ee40`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.4-stdout` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20/stdout](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20/stdout). SHA-256 `2528d1c7807116dd4aeb5a9c5f9b8ec9749cff7a1412f874745fbe3e4ccc5c90`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.4-stderr` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20/stderr](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.5-receipt-json` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19/receipt.json). SHA-256 `21c10b1c08ded9eb9f06747e02cbabd8b164740c291f0f12ea2e4bcb8a379f20`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.5-stdout` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19/stdout](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19/stdout). SHA-256 `3bf2dec40e197dbe5699de2ce85d500c265b6c51456ffbb43cba20a725269f03`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.5-stderr` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19/stderr](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.6-receipt-json` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18/receipt.json). SHA-256 `e4e203f75e071dc162d97cdeaa284d3cba2d702097d30b520a8bb54a5c76f7e9`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.6-stdout` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18/stdout](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18/stdout). SHA-256 `5d762605e7e716362d163f401ffea2fa2ff25f7b763fed2361bf76132d51ded8`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl8.6-stderr` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18/stderr](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl9.0-receipt-json` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4/receipt.json). SHA-256 `a4769a664d2ff1d9ca3c412485ddcbc29d7fd930d22ccde4a2bc4a227eb579b1`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl9.0-stdout` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4/stdout](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4/stdout). SHA-256 `5659049736fbd1e45c0af4f330d51d8a0b2d2775a12c38d36c726191179d55e2`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl9.0-stderr` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4/stderr](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl9.1-receipt-json` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0/receipt.json). SHA-256 `dd648785bfe36e0aabd9cbb25b73ac56428560ff8d0fea1bbe2012decdfc6f34`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl9.1-stdout` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0/stdout](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0/stdout). SHA-256 `23769e55db64ee959dc5a58c6ec020e7201d5ad460799a5c00769eaaf91720d2`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `tcl9.1-stderr` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0/stderr](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `jim-receipt-json` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim/receipt.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim/receipt.json). SHA-256 `b90fe1a5adee90b5421126d89e616aa13920fd7562b5afd67463cef31b1d7377`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `jim-stdout` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim/stdout](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim/stdout). SHA-256 `623215ab0012099acbfa8d3bafa451a93e3df9df0839105eac8bf54df64f19a7`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `jim-stderr` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim/stderr](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed subprocess receipt or stream for this provider and these exact inputs; no independent worker/header/lifetime inference.
- `exact-provider-input-retention` (provider): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/provider-retention.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/provider-retention.json). SHA-256 `f00da0043cbde42fb19513f70aaa16cd5a9876b93c35ec10def529e63e18429a`. Each original before/after executable/probe/library pin joins exact existing full payload bytes. Version-source provenance remains explicit; no C/Jim source or loaded-library closure is implied.
- `independent-projection` (observation): [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/interpretation.json](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/interpretation.json). SHA-256 `74382a47fa271409eabed761a9e3f7438098882078d60e3a0fd987d9397d4c78`. Exact counted raw-line byte joins and separate public result/effect fields, reviewed without executing a provider.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/run.py"
]
```

The exact original runner refuses to replace results.json and expects its original absolute provider/library paths. Restore and verify the declared payload bytes, use a separate fresh output root, and retain any new process evidence separately. For 628 supply a distinct owned disposable R2286_IO_PROBE_FILE for each provider; it is deleted by the unchanged probe. Version attribution must retain its explicit capture source. Full provider build and loaded-library replay are unrecorded. The preserved runner has no implementation-source build closure or original Native object authority.

## Exact counted sources

Every source below is ASCII. Braces, quotes, brackets, dollar signs and semicolons are literal source bytes. The hexadecimal column determines the exact bytes; none of these values contains a physical source newline.

| Case | Written source | Counted source hex |
| --- | --- | --- |
| `terminal_child` | `set before 1; set sink [list {a}b]; set after 2` | `736574206265666f726520313b207365742073696e6b205b6c697374207b617d625d3b207365742061667465722032` |
| `terminal_child_prefix` | `set before 1; set sink [set inner 1; list {a}b]; set after 2` | `736574206265666f726520313b207365742073696e6b205b73657420696e6e657220313b206c697374207b617d625d3b207365742061667465722032` |
| `terminal_quoted_child` | `set before 1; set sink "[list {a}b]"; set after 2` | `736574206265666f726520313b207365742073696e6b20225b6c697374207b617d625d223b207365742061667465722032` |
| `terminal_index_child` | `set before 1; set sink $arr([list {a}b]); set after 2` | `736574206265666f726520313b207365742073696e6b2024617272285b6c697374207b617d625d293b207365742061667465722032` |
| `missing_child_brace` | `set before 1; set sink [list {a]; set after 2` | `736574206265666f726520313b207365742073696e6b205b6c697374207b615d3b207365742061667465722032` |
| `missing_child_variable_brace` | `set before 1; set sink [list ${a]; set after 2` | `736574206265666f726520313b207365742073696e6b205b6c69737420247b615d3b207365742061667465722032` |
| `bare_child_open_brace` | `set before 1; set sink [list x{]; set after 2` | `736574206265666f726520313b207365742073696e6b205b6c69737420787b5d3b207365742061667465722032` |
