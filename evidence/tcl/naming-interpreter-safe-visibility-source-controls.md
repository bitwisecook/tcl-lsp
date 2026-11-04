# Tcl execution evidence

## Question

> An info commands card, a written colon name, a hidden-token spelling and a callable child command are distinct observations. Safe interpreters can report a card without admitting the tested calls, and a visible redefinition can coexist with a separately hidden command.

> For the eight fixed original child-interpreter source controls, what do the captured providers report for safe source cards and qualified calls, literal/scoped colon names, hide/expose tokens and collision errors, and the source-constructed counted-zero command?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One exact ASCII LF source-file CLI probe per C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim. Each fixed case owns and deletes its child; caught inner codes/results remain distinct from the outer OBSERVATION and process status. The binary-produced counted-zero case is one exact source construction, not an arbitrary raw-input protocol. These results establish finite guest cards/calls/errors only: no child activation receipt, physical table/token/object/header/cache identity, compiler admission, deferred callback, Normal/effect closure or BIG-IP answer. Conditional Compiler source visibility advice remains an independent implementation purpose. The original Native safe-visibility rows remain independent observations. This control tests source ownership and possible edges only; it supplies no actual child allocation/interpreter entry, command token, handler/frame, Normal or edit permission, and no assertion result is attached.

## Script

```tcl-visible
# ASCII source. Each case owns and deletes its child; outputs retain exact result bytes.
puts "PATCHLEVEL [info patchlevel]"
proc r2286_observe {label script} {
    set code [catch {uplevel 1 $script} result]
    binary scan $result H* hex
    puts "OBSERVATION $label $code $hex"
}
r2286_observe safe_colon_and_namespace {
    interp create -safe r2286_s
    set code [catch {
        interp eval r2286_s {
            proc :source {args} {return LITERAL_COLON}
            namespace eval : {proc source {args} {return COLON_NAMESPACE}}
            list [info commands source] [info commands :source] [:source] [namespace eval : {source}]
        }
    } result]
    interp delete r2286_s
    list $code $result
}
r2286_observe safe_global_qualification {
    interp create -safe r2286_s
    set out {}
    foreach script {{source absent.tcl} {::source absent.tcl} {::::source absent.tcl}} {
        set code [catch {interp eval r2286_s $script} result]
        lappend out $code $result
    }
    interp delete r2286_s
    set out
}
r2286_observe hide_distinct_token_and_colon_exposure {
    interp create r2286_s
    set code [catch {
        interp eval r2286_s {proc held {} {return ORIGINAL}}
        interp hide r2286_s ::held token
        set hidden [interp hidden r2286_s]
        set miss [catch {interp eval r2286_s {held}} missing]
        interp expose r2286_s token :visible
        list $hidden $miss $missing [interp eval r2286_s {:visible}] [interp eval r2286_s {info commands token}]
    } result]
    interp delete r2286_s
    list $code $result
}
r2286_observe hide_default_qualified_token {
    interp create r2286_s
    interp eval r2286_s {proc held {} {return ORIGINAL}}
    set code [catch {interp hide r2286_s ::held} result]
    set visible [interp eval r2286_s {held}]
    interp delete r2286_s
    list $code $result $visible
}
r2286_observe hide_namespace_source {
    interp create r2286_s
    interp eval r2286_s {namespace eval n {proc held {} {return NAMESPACED}}}
    set code [catch {interp hide r2286_s ::n::held token} result]
    set visible [interp eval r2286_s {n::held}]
    interp delete r2286_s
    list $code $result $visible
}
r2286_observe hidden_and_new_visible_allocations {
    interp create r2286_s
    interp eval r2286_s {proc held {} {return ORIGINAL}}
    interp hide r2286_s held token
    interp eval r2286_s {proc held {} {return REDEFINED}}
    set hidecode [catch {interp hide r2286_s held token} hidemessage]
    set exposecode [catch {interp expose r2286_s token held} exposemessage]
    set visible [interp eval r2286_s {held}]
    set original [interp invokehidden r2286_s token]
    interp delete r2286_s
    list $hidecode $hidemessage $exposecode $exposemessage $visible $original
}
r2286_observe hide_literal_colon {
    interp create r2286_s
    interp eval r2286_s {proc :held {} {return LITERAL_COLON}; proc held {} {return PLAIN}}
    interp hide r2286_s :held token
    set coloncode [catch {interp eval r2286_s {:held}} colonmessage]
    set visible [interp eval r2286_s {held}]
    set original [interp invokehidden r2286_s token]
    interp delete r2286_s
    list $coloncode $colonmessage $visible $original
}
r2286_observe counted_binary_command_boundary {
    interp create r2286_s
    set name [binary format c* {104 101 108 100 0 120}]
    interp eval r2286_s [list proc $name {} {return COUNTED}]
    set hidecode [catch {interp hide r2286_s $name token} hidemessage]
    set original [catch {interp invokehidden r2286_s token} originalmessage]
    interp delete r2286_s
    list $hidecode $hidemessage $original $originalmessage
}
rename r2286_observe {}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.
- Exit status: 0
- Standard output:

```text-visible
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

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.
- Exit status: 0
- Standard output:

```text-visible
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

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.
- Exit status: 0
- Standard output:

```text-visible
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

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.
- Exit status: 0
- Standard output:

```text-visible
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

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.
- Exit status: 0
- Standard output:

```text-visible
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

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Exact complete original ASCII LF source-file CLI; eight independent child entry controls.
- Exit status: 0
- Standard output:

```text-visible
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

- Standard error: empty.
