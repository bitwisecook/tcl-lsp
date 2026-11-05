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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls. Version: tcl8.4 8.4.20.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls. Version: tcl9.1 9.1.0.

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

Status: `unsupported`. Channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls. Version: jim 0.84-9-g5bac7c9.

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

Status: `not-tested`.

No native interpreter observation establishes this Rust source-advice invariant.

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `Analyser::original_interp_visibility_body`: Require the genuine selected child source visibility/body purpose with complete original input and words; reporting spellings cannot supply the independent child allocation or entry.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `analyser::interp_visibility::tests::original_child_visibility_is_scoped_conditional_and_retains_possible_source_edges` (linked): Conditional C child source loads retain three original source-qualified forms, their exact path word and current source subject with six independent obligations; foreign source refuses, and Jim does not inherit the C child surface.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_safe_visibility_original](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original)
- [rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.4.20)
- [rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.5.19)
- [rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/8.6.18)
- [rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.0.4)
- [rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/9.1.0)
- [rust/tcl-registry/tests/data/native_safe_visibility_original/jim](../../../../rust/tcl-registry/tests/data/native_safe_visibility_original/jim)
