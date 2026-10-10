# How do Tcl_ReadChars and counted Tcl_EvalEx differ for literal NUL, U+1F600, CR/CRLF, numeric escapes and an unknown backslash escape in the exact retained source?

Proof ID: `naming.source.document-native-utf-ingress`

## Problem statement

A literal NUL or supplementary Unicode character in a document can have a different native spelling from identical bytes supplied as a counted Tcl string. Using a Unicode display or a shared byte buffer for both inputs can select the wrong command, namespace, variable or property key. This check is limited to explicit UTF-8 character-channel ingress with automatic newline translation and counted public C evaluation.

## Question

How do Tcl_ReadChars and counted Tcl_EvalEx differ for literal NUL, U+1F600, CR/CRLF, numeric escapes and an unknown backslash escape in the exact retained source?

## Scope

C8.4.20, C8.5.19, C8.6.18, C9.0.4 and C9.1.0; public Tcl_ReadChars on -encoding utf-8/-translation auto versus Tcl_EvalEx with explicit source length. Unicode Document and counted NativeValue are separate inputs.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | 17 rows, compile0/run0. ReadChars byte length 139; native source units and evaluated values retained exactly. |
| tcl8.5 8.5.19 | observed | 17 rows, compile0/run0. ReadChars byte length 139; native source units and evaluated values retained exactly. |
| tcl8.6 8.6.18 | observed | 17 rows, compile0/run0. ReadChars byte length 133; native source units and evaluated values retained exactly. |
| tcl9.0 9.0.4 | observed | 17 rows, compile0/run0. ReadChars byte length 127; native source units and evaluated values retained exactly. |
| tcl9.1 9.1.0 | observed | 17 rows, compile0/run0. ReadChars byte length 127; native source units and evaluated values retained exactly. |
| jim not recorded | not-tested | No observation for this precise question. |
| bigip not recorded | not-tested | No observation for this precise question. |

## Conclusion

Counted evaluation preserves raw NUL, four-byte supplementary source bytes and CR/CRLF literals. UTF-8 ReadChars produces modified NUL and translated LF on every tested C release; supplementary source produces Latin-1 units per input byte on C8.4/8.5, UTF-16 surrogate units on C8.6, and a scalar four-byte sequence on C9. Escape evaluation consumes the produced native source. These observations do not determine Jim file ingress, BIG-IP configuration ingress or arbitrary malformed external bytes.

## Evidence

- `probe`: `rust/tcl-syntax/tests/data/native_source_ingress/probe.c`, SHA-256 `3a937804542861da9882bc62253838f2ded0cfb8bc2fcba98d9ffffd297edf0e`. Public C API probe and original source-to-value operations.
- `source`: `rust/tcl-syntax/tests/data/native_source_ingress/source.tcl`, SHA-256 `82a59529ab061c1ae09226fbeb6ba25896e3452df8f20e64575128384c6587e2`. Exact126 source bytes, including three literal zero bytes and three actual U+1F600 characters.
- `input-description`: `rust/tcl-syntax/tests/data/native_source_ingress/input.json`, SHA-256 `89c28e8b61be9cefb1dbd611cdd8f1c761b5112503d89a423b810199abd85909`. Exact source/probe hashes and source hex.
- `fixture`: `rust/tcl-syntax/tests/data/native_source_ingress/observations.tsv`, SHA-256 `96aadc06971add2e655eddaaed129075f2d1ffeead61aaa86b818ba39d7fda20`. Derived byte rows used by Rust tests; exact native stdout remains separate.
- `receipt-8.4.20`: `rust/tcl-syntax/tests/data/native_source_ingress/8.4.20/receipt.json`, SHA-256 `07899e942d4823708d075fe07f2111d2343ed8f38db8dbda7e80a798d25b947a`. Actual compile/run commands, library/header/executable/input/output hashes and native rows.
- `stdout-8.4.20`: `rust/tcl-syntax/tests/data/native_source_ingress/8.4.20/stdout.jsonl`, SHA-256 `287335627533cab5962b6e1e7b4df12c1edb533d10cf5ee78e8fa3fa85aa3ee8`. Actual seventeen native JSON rows including startup patchlevel.
- `stderr-8.4.20`: `rust/tcl-syntax/tests/data/native_source_ingress/8.4.20/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty native stderr; process exit recorded in receipt.
- `receipt-8.5.19`: `rust/tcl-syntax/tests/data/native_source_ingress/8.5.19/receipt.json`, SHA-256 `b0703a1c326c03d952d40b0567242c257cc1a0cec8e28ddf901444995f1e0582`. Actual compile/run commands, library/header/executable/input/output hashes and native rows.
- `stdout-8.5.19`: `rust/tcl-syntax/tests/data/native_source_ingress/8.5.19/stdout.jsonl`, SHA-256 `53688c5e7a112e0a3fa103e376c88a3882f9fdbbff94d9045b5f1c8c4d43e62a`. Actual seventeen native JSON rows including startup patchlevel.
- `stderr-8.5.19`: `rust/tcl-syntax/tests/data/native_source_ingress/8.5.19/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty native stderr; process exit recorded in receipt.
- `receipt-8.6.18`: `rust/tcl-syntax/tests/data/native_source_ingress/8.6.18/receipt.json`, SHA-256 `895e8984d008bce84d9f912804111230b1f226854dc3156637420b0d9a5c3a0d`. Actual compile/run commands, library/header/executable/input/output hashes and native rows.
- `stdout-8.6.18`: `rust/tcl-syntax/tests/data/native_source_ingress/8.6.18/stdout.jsonl`, SHA-256 `52219fe181ff146de94b7c338e0e34d9a9c92ffb8ae847923987bde9988f09c2`. Actual seventeen native JSON rows including startup patchlevel.
- `stderr-8.6.18`: `rust/tcl-syntax/tests/data/native_source_ingress/8.6.18/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty native stderr; process exit recorded in receipt.
- `receipt-9.0.4`: `rust/tcl-syntax/tests/data/native_source_ingress/9.0.4/receipt.json`, SHA-256 `7999508c1cd3f5bbeab3330883f5e8da0d10dc3564ee91227cb393a7ab4411fb`. Actual compile/run commands, library/header/executable/input/output hashes and native rows.
- `stdout-9.0.4`: `rust/tcl-syntax/tests/data/native_source_ingress/9.0.4/stdout.jsonl`, SHA-256 `2eabbb1f12882abc141c0ba1109beb5d05821fd4485498ff862b6afb9e084e50`. Actual seventeen native JSON rows including startup patchlevel.
- `stderr-9.0.4`: `rust/tcl-syntax/tests/data/native_source_ingress/9.0.4/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty native stderr; process exit recorded in receipt.
- `receipt-9.1.0`: `rust/tcl-syntax/tests/data/native_source_ingress/9.1.0/receipt.json`, SHA-256 `4f9d77997b6b2f0efc2d00487d25b0cf02a961620a73f59eddc4ab8b1369bb53`. Actual compile/run commands, library/header/executable/input/output hashes and native rows.
- `stdout-9.1.0`: `rust/tcl-syntax/tests/data/native_source_ingress/9.1.0/stdout.jsonl`, SHA-256 `f5d8e2cc9829710c68d328cd9e11718408dac832baa592f6a80b2f0b70966561`. Actual seventeen native JSON rows including startup patchlevel.
- `stderr-9.1.0`: `rust/tcl-syntax/tests/data/native_source_ingress/9.1.0/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty native stderr; process exit recorded in receipt.
- `source-8.4.20-UtfToUtfProc`: `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.4.20-UtfToUtfProc.c`, SHA-256 `a35b1c6d11a81e442c0435d3cc16f94095ffaaa4476cdb898e9be50fa87931a7`. Inspected source excerpt; explains the operation separately from observed output.
- `source-8.6.18-UtfToUtfProc`: `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.6.18-UtfToUtfProc.c`, SHA-256 `31d0d860a607787deb987674bb08321b56d9caabb506e308af451d5a7c7f7236`. Inspected source excerpt; explains the operation separately from observed output.
- `source-9.1.0-UtfToUtfProc`: `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/9.1.0-UtfToUtfProc.c`, SHA-256 `ddadfc554be09ab05d8e614524dcdfbce843c98c4ae047cad03ff25f07b8b9e4`. Inspected source excerpt; explains the operation separately from observed output.
- `source-8.4.20-Tcl_ReadChars`: `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.4.20-Tcl_ReadChars.c`, SHA-256 `87342b998371dfb4729109202735e4f0a0b670d387a581250c0e44610faabcae`. Inspected source excerpt; explains the operation separately from observed output.
- `source-8.6.18-Tcl_ReadChars`: `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.6.18-Tcl_ReadChars.c`, SHA-256 `52c78cf85daf2ac4b9108a18f756bbaed3579c282cbfeca4c71bd9cecb69714d`. Inspected source excerpt; explains the operation separately from observed output.
- `source-9.1.0-Tcl_ReadChars`: `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/9.1.0-Tcl_ReadChars.c`, SHA-256 `9c3eef631d123b88b4a0db853779be7774a654dab9f5498bac507b225bd1744e`. Inspected source excerpt; explains the operation separately from observed output.

## Source anchors

- tcl8.4 8.4.20 (`core-8-4-20`), `generic/tclEncoding.c`, `UtfToUtfProc`, lines [2055, 2138]. Exact source SHA `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; retained excerpt `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.4.20-UtfToUtfProc.c`, SHA `a35b1c6d11a81e442c0435d3cc16f94095ffaaa4476cdb898e9be50fa87931a7`.
- tcl8.6 8.6.18 (`core-8-6-18`), `generic/tclEncoding.c`, `UtfToUtfProc`, lines [2328, 2445]. Exact source SHA `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; retained excerpt `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.6.18-UtfToUtfProc.c`, SHA `31d0d860a607787deb987674bb08321b56d9caabb506e308af451d5a7c7f7236`.
- tcl9.1 9.1.0 (`core-9-1-0`), `generic/tclEncoding.c`, `UtfToUtfProc`, lines [2965, 3153]. Exact source SHA `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; retained excerpt `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/9.1.0-UtfToUtfProc.c`, SHA `ddadfc554be09ab05d8e614524dcdfbce843c98c4ae047cad03ff25f07b8b9e4`.
- tcl8.4 8.4.20 (`core-8-4-20`), `generic/tclIO.c`, `Tcl_ReadChars`, lines [4554, 4630]. Exact source SHA `cffb29befe39d56eb1e2de552124ec67380318d3359bf189b72a27a02f504858`; retained excerpt `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.4.20-Tcl_ReadChars.c`, SHA `87342b998371dfb4729109202735e4f0a0b670d387a581250c0e44610faabcae`.
- tcl8.6 8.6.18 (`core-8-6-18`), `generic/tclIO.c`, `Tcl_ReadChars`, lines [5780, 5840]. Exact source SHA `06e174e8ec55ffc301fe87f2be0298587e3946ca66d81ddefedfa814658cac0e`; retained excerpt `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/8.6.18-Tcl_ReadChars.c`, SHA `52c78cf85daf2ac4b9108a18f756bbaed3579c282cbfeca4c71bd9cecb69714d`.
- tcl9.1 9.1.0 (`core-9-1-0`), `generic/tclIO.c`, `Tcl_ReadChars`, lines [5897, 5970]. Exact source SHA `6b4b84f18f65c635335db6b0a667f109bbbde08b91f59dd90aefc0a704f20c08`; retained excerpt `rust/tcl-syntax/tests/data/native_source_ingress/source-anchors/9.1.0-Tcl_ReadChars.c`, SHA `9c3eef631d123b88b4a0db853779be7774a654dab9f5498bac507b225bd1744e`.

## Shared owners and tests

- `rust/tcl-syntax/src/backslash.rs`: `native_source_literal_bytes` — Native source-channel value production.
- `rust/tcl-syntax/src/backslash.rs`: `native_source_braced_word_bytes` — Source braced value production.
- `rust/tcl-syntax/src/backslash.rs`: `native_source_escape_channel_in` — Original source/native escape extent correspondence.
- `rust/tcl-registry/src/native_compiler_words.rs`: `NativeCompilerWords::capture` — Original compiler word/static byte values.
- `rust/tcl-syntax/src/backslash.rs`: `backslash::tests::document_native_ingress_matches_readchars_and_preserves_counted_source`. Compare both input channels and escape/braced values to captured C rows; refuse invalid Unicode Document and keep original escape extent.
- `rust/tcl-registry/src/native_compiler_words.rs`: `native_compiler_words::tests::original_document_words_match_native_character_channel_values`. Every original set operand agrees with actual native evaluation while retaining original image/channel and independent string protocol.
- `rust/tcl-registry/src/native_compiler_word_projection.rs`: `native_compiler_word_projection::tests::document_expansion_preserves_source_spans_and_native_produced_units`. Static expansion retains original member source spans while the selected channel value matches the native unit fixture.

Native output is evidence for the interpreter operation. Rust tests must independently pass to establish implementation correspondence.

## Reconfirmation

```text
python3 rust/tcl-syntax/tests/data/native_source_ingress/replay.py --tcl-source-root tmp --output /tmp/native-source-ingress-replay
```

Requires the exact named Tcl source releases with built unix/libtcl*.a and a C compiler. Runner checks launched patchlevel and all native rows; build hashes are recorded independently. Compile/file/channel/read failures are harness failures; guest completion codes are compared as data. No Jim or BIG-IP execution occurs.
