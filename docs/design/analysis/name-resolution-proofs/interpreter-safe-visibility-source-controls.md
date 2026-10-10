# naming.interpreter.safe-visibility-source-controls

Kind: `native-observation`

## Problem statement

An info commands card, a written colon name, a hidden-token spelling and a callable child command are distinct observations. Safe interpreters can report a card without admitting the tested calls, and a visible redefinition can coexist with a separately hidden command.

## Question

For the eight fixed original child-interpreter source controls, what do the captured providers report for safe source cards and qualified calls, literal/scoped colon names, hide/expose tokens and collision errors, and the source-constructed counted-zero command?

## Conclusion

C8.4/8.5 report source in the tested safe info commands query, while calls to source, ::source and ::::source each fail. C8.6–9.1 omit that card and return the same three call errors. All five C captures distinguish literal :source/:held from a colon namespace and plain names; hide of ::held with a separate token succeeds, qualified default tokens and nonglobal source commands return their exact errors, and hidden/visible collisions preserve REDEFINED versus ORIGINAL results. The one source-constructed held-zero-x command hides and invokes with COUNTED. Current Jim rejects child-interpreter entry in all eight cases with wrong # args: should be "interp"; no child visibility, hide/expose or counted-name answer is established for Jim.

An additional marked source definition retains the original path word and typed visibility subject for source, ::source and ::::source in the conditional C child body. Each subject preserves six independent applicability obligations and refuses foreign source; Jim source analysis supplies none of that C surface.

## Scope

One exact ASCII LF source-file CLI probe per C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim. Each fixed case owns and deletes its child; caught inner codes/results remain distinct from the outer OBSERVATION and process status. The binary-produced counted-zero case is one exact source construction, not an arbitrary raw-input protocol. These results establish finite guest cards/calls/errors only: no child activation receipt, physical table/token/object/header/cache identity, compiler admission, deferred callback, Normal/effect closure or BIG-IP answer. Conditional Compiler source visibility advice remains an independent implementation purpose.

The original Native safe-visibility rows remain independent observations. This control tests source ownership and possible edges only; it supplies no actual child allocation/interpreter entry, command token, handler/frame, Normal or edit permission, and no assertion result is attached.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; exact SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 8.4.20
OBSERVATION safe_colon_and_namespace 0 30207b736f75726365203a736f75726365204c49544552414c5f434f4c4f4e20434f4c4f4e5f4e414d4553504143457d
OBSERVATION safe_global_qualification 0 31207b696e76616c696420636f6d6d616e64206e616d652022736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a3a3a736f75726365227d
OBSERVATION hide_distinct_token_and_colon_exposure 0 30207b746f6b656e2031207b696e76616c696420636f6d6d616e64206e616d65202268656c64227d204f524947494e414c207b7d7d
OBSERVATION hide_default_qualified_token 0 31207b63616e6e6f7420757365206e616d657370616365207175616c69666965727320696e2068696464656e20636f6d6d616e6420746f6b656e202872656e616d65297d204f524947494e414c
OBSERVATION hide_namespace_source 0 31207b63616e206f6e6c79206869646520676c6f62616c206e616d65737061636520636f6d6d616e647320287573652072656e616d65207468656e2068696465297d204e414d45535041434544
OBSERVATION hidden_and_new_visible_allocations 0 31207b68696464656e20636f6d6d616e64206e616d65642022746f6b656e2220616c7265616479206578697374737d2031207b6578706f73656420636f6d6d616e64202268656c642220616c7265616479206578697374737d205245444546494e4544204f524947494e414c
OBSERVATION hide_literal_colon 0 31207b696e76616c696420636f6d6d616e64206e616d6520223a68656c64227d20504c41494e204c49544552414c5f434f4c4f4e
OBSERVATION counted_binary_command_boundary 0 30207b7d203020434f554e544544
```

Decoded fixed rows:

```text
safe_colon_and_namespace 0 0 {source :source LITERAL_COLON COLON_NAMESPACE}
safe_global_qualification 0 1 {invalid command name "source"} 1 {invalid command name "::source"} 1 {invalid command name "::::source"}
hide_distinct_token_and_colon_exposure 0 0 {token 1 {invalid command name "held"} ORIGINAL {}}
hide_default_qualified_token 0 1 {cannot use namespace qualifiers in hidden command token (rename)} ORIGINAL
hide_namespace_source 0 1 {can only hide global namespace commands (use rename then hide)} NAMESPACED
hidden_and_new_visible_allocations 0 1 {hidden command named "token" already exists} 1 {exposed command "held" already exists} REDEFINED ORIGINAL
hide_literal_colon 0 1 {invalid command name ":held"} PLAIN LITERAL_COLON
counted_binary_command_boundary 0 0 {} 0 COUNTED
```

Outer process status 0 with empty stderr. These finite guest results supply no physical child/table/token identity or source implementation authority.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; exact SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 8.5.19
OBSERVATION safe_colon_and_namespace 0 30207b736f75726365203a736f75726365204c49544552414c5f434f4c4f4e20434f4c4f4e5f4e414d4553504143457d
OBSERVATION safe_global_qualification 0 31207b696e76616c696420636f6d6d616e64206e616d652022736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a3a3a736f75726365227d
OBSERVATION hide_distinct_token_and_colon_exposure 0 30207b746f6b656e2031207b696e76616c696420636f6d6d616e64206e616d65202268656c64227d204f524947494e414c207b7d7d
OBSERVATION hide_default_qualified_token 0 31207b63616e6e6f7420757365206e616d657370616365207175616c69666965727320696e2068696464656e20636f6d6d616e6420746f6b656e202872656e616d65297d204f524947494e414c
OBSERVATION hide_namespace_source 0 31207b63616e206f6e6c79206869646520676c6f62616c206e616d65737061636520636f6d6d616e647320287573652072656e616d65207468656e2068696465297d204e414d45535041434544
OBSERVATION hidden_and_new_visible_allocations 0 31207b68696464656e20636f6d6d616e64206e616d65642022746f6b656e2220616c7265616479206578697374737d2031207b6578706f73656420636f6d6d616e64202268656c642220616c7265616479206578697374737d205245444546494e4544204f524947494e414c
OBSERVATION hide_literal_colon 0 31207b696e76616c696420636f6d6d616e64206e616d6520223a68656c64227d20504c41494e204c49544552414c5f434f4c4f4e
OBSERVATION counted_binary_command_boundary 0 30207b7d203020434f554e544544
```

Decoded fixed rows:

```text
safe_colon_and_namespace 0 0 {source :source LITERAL_COLON COLON_NAMESPACE}
safe_global_qualification 0 1 {invalid command name "source"} 1 {invalid command name "::source"} 1 {invalid command name "::::source"}
hide_distinct_token_and_colon_exposure 0 0 {token 1 {invalid command name "held"} ORIGINAL {}}
hide_default_qualified_token 0 1 {cannot use namespace qualifiers in hidden command token (rename)} ORIGINAL
hide_namespace_source 0 1 {can only hide global namespace commands (use rename then hide)} NAMESPACED
hidden_and_new_visible_allocations 0 1 {hidden command named "token" already exists} 1 {exposed command "held" already exists} REDEFINED ORIGINAL
hide_literal_colon 0 1 {invalid command name ":held"} PLAIN LITERAL_COLON
counted_binary_command_boundary 0 0 {} 0 COUNTED
```

Outer process status 0 with empty stderr. These finite guest results supply no physical child/table/token identity or source implementation authority.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; exact SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 8.6.18
OBSERVATION safe_colon_and_namespace 0 30207b7b7d203a736f75726365204c49544552414c5f434f4c4f4e20434f4c4f4e5f4e414d4553504143457d
OBSERVATION safe_global_qualification 0 31207b696e76616c696420636f6d6d616e64206e616d652022736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a3a3a736f75726365227d
OBSERVATION hide_distinct_token_and_colon_exposure 0 30207b746f6b656e2031207b696e76616c696420636f6d6d616e64206e616d65202268656c64227d204f524947494e414c207b7d7d
OBSERVATION hide_default_qualified_token 0 31207b63616e6e6f7420757365206e616d657370616365207175616c69666965727320696e2068696464656e20636f6d6d616e6420746f6b656e202872656e616d65297d204f524947494e414c
OBSERVATION hide_namespace_source 0 31207b63616e206f6e6c79206869646520676c6f62616c206e616d65737061636520636f6d6d616e647320287573652072656e616d65207468656e2068696465297d204e414d45535041434544
OBSERVATION hidden_and_new_visible_allocations 0 31207b68696464656e20636f6d6d616e64206e616d65642022746f6b656e2220616c7265616479206578697374737d2031207b6578706f73656420636f6d6d616e64202268656c642220616c7265616479206578697374737d205245444546494e4544204f524947494e414c
OBSERVATION hide_literal_colon 0 31207b696e76616c696420636f6d6d616e64206e616d6520223a68656c64227d20504c41494e204c49544552414c5f434f4c4f4e
OBSERVATION counted_binary_command_boundary 0 30207b7d203020434f554e544544
```

Decoded fixed rows:

```text
safe_colon_and_namespace 0 0 {{} :source LITERAL_COLON COLON_NAMESPACE}
safe_global_qualification 0 1 {invalid command name "source"} 1 {invalid command name "::source"} 1 {invalid command name "::::source"}
hide_distinct_token_and_colon_exposure 0 0 {token 1 {invalid command name "held"} ORIGINAL {}}
hide_default_qualified_token 0 1 {cannot use namespace qualifiers in hidden command token (rename)} ORIGINAL
hide_namespace_source 0 1 {can only hide global namespace commands (use rename then hide)} NAMESPACED
hidden_and_new_visible_allocations 0 1 {hidden command named "token" already exists} 1 {exposed command "held" already exists} REDEFINED ORIGINAL
hide_literal_colon 0 1 {invalid command name ":held"} PLAIN LITERAL_COLON
counted_binary_command_boundary 0 0 {} 0 COUNTED
```

Outer process status 0 with empty stderr. These finite guest results supply no physical child/table/token identity or source implementation authority.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; exact SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 9.0.4
OBSERVATION safe_colon_and_namespace 0 30207b7b7d203a736f75726365204c49544552414c5f434f4c4f4e20434f4c4f4e5f4e414d4553504143457d
OBSERVATION safe_global_qualification 0 31207b696e76616c696420636f6d6d616e64206e616d652022736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a3a3a736f75726365227d
OBSERVATION hide_distinct_token_and_colon_exposure 0 30207b746f6b656e2031207b696e76616c696420636f6d6d616e64206e616d65202268656c64227d204f524947494e414c207b7d7d
OBSERVATION hide_default_qualified_token 0 31207b63616e6e6f7420757365206e616d657370616365207175616c69666965727320696e2068696464656e20636f6d6d616e6420746f6b656e202872656e616d65297d204f524947494e414c
OBSERVATION hide_namespace_source 0 31207b63616e206f6e6c79206869646520676c6f62616c206e616d65737061636520636f6d6d616e647320287573652072656e616d65207468656e2068696465297d204e414d45535041434544
OBSERVATION hidden_and_new_visible_allocations 0 31207b68696464656e20636f6d6d616e64206e616d65642022746f6b656e2220616c7265616479206578697374737d2031207b6578706f73656420636f6d6d616e64202268656c642220616c7265616479206578697374737d205245444546494e4544204f524947494e414c
OBSERVATION hide_literal_colon 0 31207b696e76616c696420636f6d6d616e64206e616d6520223a68656c64227d20504c41494e204c49544552414c5f434f4c4f4e
OBSERVATION counted_binary_command_boundary 0 30207b7d203020434f554e544544
```

Decoded fixed rows:

```text
safe_colon_and_namespace 0 0 {{} :source LITERAL_COLON COLON_NAMESPACE}
safe_global_qualification 0 1 {invalid command name "source"} 1 {invalid command name "::source"} 1 {invalid command name "::::source"}
hide_distinct_token_and_colon_exposure 0 0 {token 1 {invalid command name "held"} ORIGINAL {}}
hide_default_qualified_token 0 1 {cannot use namespace qualifiers in hidden command token (rename)} ORIGINAL
hide_namespace_source 0 1 {can only hide global namespace commands (use rename then hide)} NAMESPACED
hidden_and_new_visible_allocations 0 1 {hidden command named "token" already exists} 1 {exposed command "held" already exists} REDEFINED ORIGINAL
hide_literal_colon 0 1 {invalid command name ":held"} PLAIN LITERAL_COLON
counted_binary_command_boundary 0 0 {} 0 COUNTED
```

Outer process status 0 with empty stderr. These finite guest results supply no physical child/table/token identity or source implementation authority.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; exact SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 9.1.0
OBSERVATION safe_colon_and_namespace 0 30207b7b7d203a736f75726365204c49544552414c5f434f4c4f4e20434f4c4f4e5f4e414d4553504143457d
OBSERVATION safe_global_qualification 0 31207b696e76616c696420636f6d6d616e64206e616d652022736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a736f75726365227d2031207b696e76616c696420636f6d6d616e64206e616d6520223a3a3a3a736f75726365227d
OBSERVATION hide_distinct_token_and_colon_exposure 0 30207b746f6b656e2031207b696e76616c696420636f6d6d616e64206e616d65202268656c64227d204f524947494e414c207b7d7d
OBSERVATION hide_default_qualified_token 0 31207b63616e6e6f7420757365206e616d657370616365207175616c69666965727320696e2068696464656e20636f6d6d616e6420746f6b656e202872656e616d65297d204f524947494e414c
OBSERVATION hide_namespace_source 0 31207b63616e206f6e6c79206869646520676c6f62616c206e616d65737061636520636f6d6d616e647320287573652072656e616d65207468656e2068696465297d204e414d45535041434544
OBSERVATION hidden_and_new_visible_allocations 0 31207b68696464656e20636f6d6d616e64206e616d65642022746f6b656e2220616c7265616479206578697374737d2031207b6578706f73656420636f6d6d616e64202268656c642220616c7265616479206578697374737d205245444546494e4544204f524947494e414c
OBSERVATION hide_literal_colon 0 31207b696e76616c696420636f6d6d616e64206e616d6520223a68656c64227d20504c41494e204c49544552414c5f434f4c4f4e
OBSERVATION counted_binary_command_boundary 0 30207b7d203020434f554e544544
```

Decoded fixed rows:

```text
safe_colon_and_namespace 0 0 {{} :source LITERAL_COLON COLON_NAMESPACE}
safe_global_qualification 0 1 {invalid command name "source"} 1 {invalid command name "::source"} 1 {invalid command name "::::source"}
hide_distinct_token_and_colon_exposure 0 0 {token 1 {invalid command name "held"} ORIGINAL {}}
hide_default_qualified_token 0 1 {cannot use namespace qualifiers in hidden command token (rename)} ORIGINAL
hide_namespace_source 0 1 {can only hide global namespace commands (use rename then hide)} NAMESPACED
hidden_and_new_visible_allocations 0 1 {hidden command named "token" already exists} 1 {exposed command "held" already exists} REDEFINED ORIGINAL
hide_literal_colon 0 1 {invalid command name ":held"} PLAIN LITERAL_COLON
counted_binary_command_boundary 0 0 {} 0 COUNTED
```

Outer process status 0 with empty stderr. These finite guest results supply no physical child/table/token identity or source implementation authority.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; exact SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.. Dialect: Jim Tcl.

Exact original stdout:

```text
PATCHLEVEL 0.84-9-g5bac7c9
OBSERVATION safe_colon_and_namespace 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
OBSERVATION safe_global_qualification 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
OBSERVATION hide_distinct_token_and_colon_exposure 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
OBSERVATION hide_default_qualified_token 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
OBSERVATION hide_namespace_source 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
OBSERVATION hidden_and_new_visible_allocations 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
OBSERVATION hide_literal_colon 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
OBSERVATION counted_binary_command_boundary 1 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
```

Decoded fixed rows:

```text
safe_colon_and_namespace 1 wrong # args: should be "interp"
safe_global_qualification 1 wrong # args: should be "interp"
hide_distinct_token_and_colon_exposure 1 wrong # args: should be "interp"
hide_default_qualified_token 1 wrong # args: should be "interp"
hide_namespace_source 1 wrong # args: should be "interp"
hidden_and_new_visible_allocations 1 wrong # args: should be "interp"
hide_literal_colon 1 wrong # args: should be "interp"
counted_binary_command_boundary 1 wrong # args: should be "interp"
```

Child entry is unsupported in every case; no child visibility or hidden-token answer is inferred.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_safe_visibility_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/probe.tcl). SHA-256 `aa9cc598b853d1dd72bc20bc8c1f0fb6b48e99fc554a295366d909114c48b96f`. Exact original complete ASCII LF source containing eight independently owned child controls.
- `runner` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/capture.py](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/capture.py). SHA-256 `936eb374440ed5fa52dc9622a512869074c8690e5ee3d8de0253297103a7d3a9`. Exact original Root runner; process argv, queue dependency and output capture protocol retained.
- `queue` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/queue.json](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Original selected executable/SDK/header/library/source/environment queue.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20/receipt.json). SHA-256 `26373f8b6df5c54fa4b82fb523936773dcc36c4e758270368dd9c7be0ebb1c2a`. Original process argv/status, queried version, source/runner/executable/SDK/environment and raw stream digests. No child activation receipt is issued.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20/stdout). SHA-256 `fb39dba0ddabe333845bd73b6c450299c02b6ef651571dacffe3d891cf435a7f`. Exact original complete stdout bytes; inner catch codes remain separate from process status.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; inner catch codes remain separate from process status.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19/receipt.json). SHA-256 `36f195028ab002d5291ef190bc84774f25a83b97bc76e0226b1fd0cafa1b16f8`. Original process argv/status, queried version, source/runner/executable/SDK/environment and raw stream digests. No child activation receipt is issued.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19/stdout). SHA-256 `6fb6792b5e1eadfb7b93f4b17d854f4e2e29fdc1a8e3f740400f76f84475a847`. Exact original complete stdout bytes; inner catch codes remain separate from process status.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; inner catch codes remain separate from process status.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18/receipt.json). SHA-256 `b4766d45b88ca1a0b2d3f817afcae56fb8d40f0d8806452a7e2fc12604d5618e`. Original process argv/status, queried version, source/runner/executable/SDK/environment and raw stream digests. No child activation receipt is issued.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18/stdout). SHA-256 `71e797afe2205b4e662228bbc45885940ef9597c7368b5568b48dfd32fe3839c`. Exact original complete stdout bytes; inner catch codes remain separate from process status.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; inner catch codes remain separate from process status.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4/receipt.json). SHA-256 `9b921b067ebe3101a739353a8a7b11c3c8ac968fb64596ebafff5cb6fe430242`. Original process argv/status, queried version, source/runner/executable/SDK/environment and raw stream digests. No child activation receipt is issued.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4/stdout). SHA-256 `9aa4606e3932788406bdcde373dae6ccd5c43ef8b67dcf4d3df91d347afc5594`. Exact original complete stdout bytes; inner catch codes remain separate from process status.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; inner catch codes remain separate from process status.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0/receipt.json). SHA-256 `cc1a58d03d8e3fe98579859c7845b40ea804f9ceae35746a0a6907a742c3bb8f`. Original process argv/status, queried version, source/runner/executable/SDK/environment and raw stream digests. No child activation receipt is issued.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0/stdout). SHA-256 `c969bc08602a28d20af21cf446d019c67a63bf412fd050fed6945d29ed26c3a9`. Exact original complete stdout bytes; inner catch codes remain separate from process status.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; inner catch codes remain separate from process status.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_safe_visibility_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/jim/receipt.json). SHA-256 `fb5e344f6957aa8e91161d5dd38643c36afdf71980cc627a44721f93156cd31d`. Original process argv/status, queried version, source/runner/executable/SDK/environment and raw stream digests. No child activation receipt is issued.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/jim/stdout). SHA-256 `60afe32d5c2364b5971bd56d09b19110ab33786bfb24fd5784a5b20875b6c917`. Exact original complete stdout bytes; inner catch codes remain separate from process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_safe_visibility_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; inner catch codes remain separate from process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `Analyser::original_interp_visibility_body`: Require the genuine selected child source visibility/body purpose with complete original input and words; reporting spellings cannot supply the independent child allocation or entry.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `analyser::interp_visibility::tests::original_child_visibility_is_scoped_conditional_and_retains_possible_source_edges` (linked): Conditional C child source loads retain three original source-qualified forms, their exact path word and current source subject with six independent obligations; foreign source refuses, and Jim does not inherit the C child surface.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "/workspace/.proofs/native-safe-visibility157/capture.py"
]
```

The original runner retains capture-machine paths and its provider queue dependency. Replay requires independently verified matching executable/SDK/source/environment pins and fresh output paths. This review launches no native process or Rust tests. Jim child creation is unsupported; its outer catches cannot supply any hidden-command or child lookup answer.
