# naming.variable.empty-array-root-lexical-reference

Kind: `native-observation`

## Problem statement

Removing an array variable root from a source reference produces $(k). C Tcl can interpret this as an element of an empty-named array, while Jim can select expression sugar. Treating both engines identically can reject a valid C edit or emit a different Jim program. These ASCII source controls distinguish lexical, braced, scalar and inert forms; they do not establish arbitrary name edit authority.

## Question

After set {(k)} VALUE, what do $(k), ${(k)}, ${} and inert {$(k)} return on the six selected engines?

## Conclusion

C Tcl 8.4.20 through 9.1.0 reads VALUE through $(k) and ${(k)}; ${} errors because the empty root is an array. Jim 0.84-9-g5bac7c9 treats $(k) as expression syntax and errors, reads VALUE through ${(k)}, and reads its empty-root dictionary as k VALUE through ${}. The inert form returns literal $(k) on all six. This is the tested ASCII lexical/value distinction, with no command existence, edit, Native header or compiler admission grant.

## Scope

Five fixed caught ASCII source controls passed as exact file bytes to shell stdin; actual info patchlevel output, ELF/header/source-owner/Makefile hashes and process stdout/stderr retained. BIG-IP not tested. No raw zero, opaque non-UTF8, source channel conversion or original object representation claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual shell executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; independent header/source-owner/Makefile SHA retained in provider receipt. No native compilation was performed by this capture.. Channel: ASCII file bytes supplied directly to native shell stdin. Dialect: Tcl.

EMPTY_LEXICAL=VALUE; EMPTY_BRACED=VALUE; EMPTY_SCALAR=error variable is array; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual shell executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; independent header/source-owner/Makefile SHA retained in provider receipt. No native compilation was performed by this capture.. Channel: ASCII file bytes supplied directly to native shell stdin. Dialect: Tcl.

EMPTY_LEXICAL=VALUE; EMPTY_BRACED=VALUE; EMPTY_SCALAR=error variable is array; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual shell executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; independent header/source-owner/Makefile SHA retained in provider receipt. No native compilation was performed by this capture.. Channel: ASCII file bytes supplied directly to native shell stdin. Dialect: Tcl.

EMPTY_LEXICAL=VALUE; EMPTY_BRACED=VALUE; EMPTY_SCALAR=error variable is array; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual shell executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; independent header/source-owner/Makefile SHA retained in provider receipt. No native compilation was performed by this capture.. Channel: ASCII file bytes supplied directly to native shell stdin. Dialect: Tcl.

EMPTY_LEXICAL=VALUE; EMPTY_BRACED=VALUE; EMPTY_SCALAR=error variable is array; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual shell executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; independent header/source-owner/Makefile SHA retained in provider receipt. No native compilation was performed by this capture.. Channel: ASCII file bytes supplied directly to native shell stdin. Dialect: Tcl.

EMPTY_LEXICAL=VALUE; EMPTY_BRACED=VALUE; EMPTY_SCALAR=error variable is array; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual shell executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; independent header/source-owner/Makefile SHA retained in provider receipt. No native compilation was performed by this capture.. Channel: ASCII file bytes supplied directly to native shell stdin. Dialect: Jim Tcl.

EMPTY_LEXICAL=error syntax expression (k); EMPTY_BRACED=VALUE; EMPTY_SCALAR=k VALUE; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `input` (input): [rust/tcl-lexer/tests/data/native_empty_array_root/cases.tcl](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/cases.tcl). SHA-256 `285a98b19dbea8bf2dc322c166cbe6f7f2bd65fd37c327c5cdd9c92387b562c5`. Exact ASCII source including five caught controls and actual info patchlevel query.
- `capture` (provider): [rust/tcl-lexer/tests/data/native_empty_array_root/receipt.json](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/receipt.json). SHA-256 `14426c9b222cf7bba323d9204dcf6c3ca240670159f6dcef6cb68b1a00683fa5`. Original six process/build/input/stream receipts; executable inputs are hashes, not bundled ELF objects.
- `tcl8.4-receipt` (provider): [rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20/receipt.json](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20/receipt.json). SHA-256 `61223eb4d4ae837a259e21640f0af97236e6e29c13ba024e771627fc58cc9035`. Original actual launched executable/hash and reported-version/process/output receipt.
- `tcl8.4-stdout` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20/stdout](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20/stdout). SHA-256 `c14cc5b4c67a8b01fb0c248393981512bcf26b89fa82bf247b3eab842fcae631`. Full original actual version and five case code/result hex rows.
- `tcl8.4-stderr` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20/stderr](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original empty process stderr; caught guest errors appear in stdout rows.
- `tcl8.5-receipt` (provider): [rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19/receipt.json](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19/receipt.json). SHA-256 `8d74ea489d179c2681cb47f2027b27a5df0b6caecf2c2b92a5211e38b4ee24c1`. Original actual launched executable/hash and reported-version/process/output receipt.
- `tcl8.5-stdout` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19/stdout](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19/stdout). SHA-256 `12e84c7eb4fe19b16d7624b6280a3e004378626ade282530746c217edb93fa60`. Full original actual version and five case code/result hex rows.
- `tcl8.5-stderr` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19/stderr](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original empty process stderr; caught guest errors appear in stdout rows.
- `tcl8.6-receipt` (provider): [rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18/receipt.json](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18/receipt.json). SHA-256 `f8f671b080a0831a5f171d01dcfd8c65059a8a3925d28f09fd9623db88605a31`. Original actual launched executable/hash and reported-version/process/output receipt.
- `tcl8.6-stdout` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18/stdout](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18/stdout). SHA-256 `c65dfedaf5900e78d35cb0d826b0774aac31e45e41fd7d7e499d006e279ae946`. Full original actual version and five case code/result hex rows.
- `tcl8.6-stderr` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18/stderr](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original empty process stderr; caught guest errors appear in stdout rows.
- `tcl9.0-receipt` (provider): [rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4/receipt.json](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4/receipt.json). SHA-256 `5668366f75a7f1db65523442e59d2f40323ab703104331a11eb6bcb5eaa9dcf8`. Original actual launched executable/hash and reported-version/process/output receipt.
- `tcl9.0-stdout` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4/stdout](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4/stdout). SHA-256 `bbb1c8d590cbc5072ae6c0025cb24d1c3dea5599c63e420f00e7a802e9e41096`. Full original actual version and five case code/result hex rows.
- `tcl9.0-stderr` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4/stderr](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original empty process stderr; caught guest errors appear in stdout rows.
- `tcl9.1-receipt` (provider): [rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0/receipt.json](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0/receipt.json). SHA-256 `50486eb7b609fdead61c4e2cd650b06cb25271551b7dc4297bdcfd3ae523bab7`. Original actual launched executable/hash and reported-version/process/output receipt.
- `tcl9.1-stdout` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0/stdout](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0/stdout). SHA-256 `10be8e237916587e5a14a980892375da05e40c0aa142fd0034a2b24c8cf5a7b4`. Full original actual version and five case code/result hex rows.
- `tcl9.1-stderr` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0/stderr](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original empty process stderr; caught guest errors appear in stdout rows.
- `jim-receipt` (provider): [rust/tcl-lexer/tests/data/native_empty_array_root/jim/receipt.json](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/jim/receipt.json). SHA-256 `12fbcc761f347c81d95ad900c17a28485352e2bdcedc5b72700718bb430a32b4`. Original actual launched executable/hash and reported-version/process/output receipt.
- `jim-stdout` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/jim/stdout](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/jim/stdout). SHA-256 `f7b8fd568a648c7b693cfdd387ed696708b4f39628a1dd30baf7470885a6c46c`. Full original actual version and five case code/result hex rows.
- `jim-stderr` (observation): [rust/tcl-lexer/tests/data/native_empty_array_root/jim/stderr](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original empty process stderr; caught guest errors appear in stdout rows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `decompose_spanned`: Retain the selected C array-reference or Jim expression component geometry without inferring a runtime value or array cell.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::template_policy_tests::jim_expression_components_retain_parentheses_and_native_token_sites` (linked): The shared lexer retains Jim expression parentheses while C selects an array reference with an empty root; this is lexical geometry only, separate from observed native values.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-lexer/tests/data/native_empty_array_root/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-empty-array-root-reconfirmation"
]
```

Requires exact recorded shell ELF/header/source-owner/Makefile inputs. Compares complete stdout/stderr and process status; caught guest errors are data. --verify-only launches no engine. This protocol neither compiles nor executes Rust.
