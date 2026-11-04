# Tcl execution evidence

## Question

> Treating an empty TclOO constructor body as an installed zero-command method can validate unused formals or reject arguments that the selected runtime accepts. Removing an own constructor also need not remove an inherited constructor.

> Does an empty TclOO constructor definition remove its own constructor before formal validation, while space/comment bodies retain arity and an inherited constructor remains selected after own removal?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Six fresh selected executable processes, fixed ASCII LF source-file input, caught definition outcomes, own-constructor reflection and new-call acceptance for four exact argument counts. No package loading, external BIG-IP, destructor control, original native object/header/cache getter, opaque name, frame recipe, compiler admission, arbitrary callback effects or general Normal proof is measured. Generated object names in the retained stdout are results of these exact runs, not portable identity assertions.

## Script

```tcl-visible
# Original ASCII source-script lifecycle controls. No installed native object
# header, binary-produced operand or BIG-IP context is observed by this script.
puts [list META patchlevel [info patchlevel] tcloo_command [llength [info commands ::oo::class]]]
if {![llength [info commands ::oo::class]]} {
    puts [list UNAVAILABLE reason no_original_oo_class_command]
} else {
    foreach {label params body} {
        empty {a b} {}
        space {a b} { }
        comment {a b} {# body}
        empty_malformed {{a b c}} {}
        space_malformed {{a b c}} { }
    } {
        set name ::original_lifecycle_$label
        set code [catch [list ::oo::class create $name [list constructor $params $body]] result]
        puts [list DECL label $label code $code result $result]
        if {$code == 0} {
            set ownCode [catch [list info class constructor $name] ownResult]
            puts [list OWN label $label code $ownCode constructor $ownResult]
            foreach count {0 1 2 3} {
                set argv [list $name new]
                for {set i 0} {$i < $count} {incr i} {lappend argv V$i}
                set code [catch $argv result]
                set ec {}
                if {$code != 0 && [info exists ::errorCode]} {set ec $::errorCode}
                puts [list CALL label $label argc $count code $code result $result errorCode $ec]
                if {$code == 0} {$result destroy}
            }
            $name destroy
        }
    }
    ::oo::class create ::original_lifecycle_parent {constructor {a b} { }}
    ::oo::class create ::original_lifecycle_child {
        superclass ::original_lifecycle_parent
        constructor {{a b c}} {}
    }
    set ownCode [catch {info class constructor ::original_lifecycle_child} ownResult]
    puts [list OWN label inherited_after_empty_removal code $ownCode constructor $ownResult]
    foreach count {0 1 2 3} {
        set argv [list ::original_lifecycle_child new]
        for {set i 0} {$i < $count} {incr i} {lappend argv V$i}
        set code [catch $argv result]
        set ec {}
        if {$code != 0 && [info exists ::errorCode]} {set ec $::errorCode}
        puts [list CALL label inherited_after_empty_removal argc $count code $code result $result errorCode $ec]
        if {$code == 0} {$result destroy}
    }
    ::original_lifecycle_child destroy
    ::original_lifecycle_parent destroy
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.4.20 tcloo_command 0
UNAVAILABLE reason no_original_oo_class_command
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.5.19 tcloo_command 0
UNAVAILABLE reason no_original_oo_class_command
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.6.18 tcloo_command 1
DECL label empty code 0 result ::original_lifecycle_empty
OWN label empty code 0 constructor {}
CALL label empty argc 0 code 0 result ::oo::Obj12 errorCode {}
CALL label empty argc 1 code 0 result ::oo::Obj13 errorCode {}
CALL label empty argc 2 code 0 result ::oo::Obj14 errorCode {}
CALL label empty argc 3 code 0 result ::oo::Obj15 errorCode {}
DECL label space code 0 result ::original_lifecycle_space
OWN label space code 0 constructor {{a b} { }}
CALL label space argc 0 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
CALL label space argc 1 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
CALL label space argc 2 code 0 result ::oo::Obj19 errorCode {}
CALL label space argc 3 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
DECL label comment code 0 result ::original_lifecycle_comment
OWN label comment code 0 constructor {{a b} {# body}}
CALL label comment argc 0 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
CALL label comment argc 1 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
CALL label comment argc 2 code 0 result ::oo::Obj24 errorCode {}
CALL label comment argc 3 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
DECL label empty_malformed code 0 result ::original_lifecycle_empty_malformed
OWN label empty_malformed code 0 constructor {}
CALL label empty_malformed argc 0 code 0 result ::oo::Obj27 errorCode {}
CALL label empty_malformed argc 1 code 0 result ::oo::Obj28 errorCode {}
CALL label empty_malformed argc 2 code 0 result ::oo::Obj29 errorCode {}
CALL label empty_malformed argc 3 code 0 result ::oo::Obj30 errorCode {}
DECL label space_malformed code 1 result {too many fields in argument specifier "a b c"}
OWN label inherited_after_empty_removal code 0 constructor {}
CALL label inherited_after_empty_removal argc 0 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
CALL label inherited_after_empty_removal argc 1 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
CALL label inherited_after_empty_removal argc 2 code 0 result ::oo::Obj36 errorCode {}
CALL label inherited_after_empty_removal argc 3 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 9.0.4 tcloo_command 1
DECL label empty code 0 result ::original_lifecycle_empty
OWN label empty code 0 constructor {}
CALL label empty argc 0 code 0 result ::oo::Obj22 errorCode {}
CALL label empty argc 1 code 0 result ::oo::Obj23 errorCode {}
CALL label empty argc 2 code 0 result ::oo::Obj24 errorCode {}
CALL label empty argc 3 code 0 result ::oo::Obj25 errorCode {}
DECL label space code 0 result ::original_lifecycle_space
OWN label space code 0 constructor {{a b} { }}
CALL label space argc 0 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
CALL label space argc 1 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
CALL label space argc 2 code 0 result ::oo::Obj30 errorCode {}
CALL label space argc 3 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
DECL label comment code 0 result ::original_lifecycle_comment
OWN label comment code 0 constructor {{a b} {# body}}
CALL label comment argc 0 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
CALL label comment argc 1 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
CALL label comment argc 2 code 0 result ::oo::Obj36 errorCode {}
CALL label comment argc 3 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
DECL label empty_malformed code 0 result ::original_lifecycle_empty_malformed
OWN label empty_malformed code 0 constructor {}
CALL label empty_malformed argc 0 code 0 result ::oo::Obj40 errorCode {}
CALL label empty_malformed argc 1 code 0 result ::oo::Obj41 errorCode {}
CALL label empty_malformed argc 2 code 0 result ::oo::Obj42 errorCode {}
CALL label empty_malformed argc 3 code 0 result ::oo::Obj43 errorCode {}
DECL label space_malformed code 1 result {too many fields in argument specifier "a b c"}
OWN label inherited_after_empty_removal code 0 constructor {}
CALL label inherited_after_empty_removal argc 0 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
CALL label inherited_after_empty_removal argc 1 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
CALL label inherited_after_empty_removal argc 2 code 0 result ::oo::Obj52 errorCode {}
CALL label inherited_after_empty_removal argc 3 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 9.1.0 tcloo_command 1
DECL label empty code 0 result ::original_lifecycle_empty
OWN label empty code 0 constructor {}
CALL label empty argc 0 code 0 result ::oo::Obj22 errorCode {}
CALL label empty argc 1 code 0 result ::oo::Obj23 errorCode {}
CALL label empty argc 2 code 0 result ::oo::Obj24 errorCode {}
CALL label empty argc 3 code 0 result ::oo::Obj25 errorCode {}
DECL label space code 0 result ::original_lifecycle_space
OWN label space code 0 constructor {{a b} { }}
CALL label space argc 0 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
CALL label space argc 1 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
CALL label space argc 2 code 0 result ::oo::Obj30 errorCode {}
CALL label space argc 3 code 1 result {wrong # args: should be "::original_lifecycle_space new a b"} errorCode {TCL WRONGARGS}
DECL label comment code 0 result ::original_lifecycle_comment
OWN label comment code 0 constructor {{a b} {# body}}
CALL label comment argc 0 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
CALL label comment argc 1 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
CALL label comment argc 2 code 0 result ::oo::Obj36 errorCode {}
CALL label comment argc 3 code 1 result {wrong # args: should be "::original_lifecycle_comment new a b"} errorCode {TCL WRONGARGS}
DECL label empty_malformed code 0 result ::original_lifecycle_empty_malformed
OWN label empty_malformed code 0 constructor {}
CALL label empty_malformed argc 0 code 0 result ::oo::Obj40 errorCode {}
CALL label empty_malformed argc 1 code 0 result ::oo::Obj41 errorCode {}
CALL label empty_malformed argc 2 code 0 result ::oo::Obj42 errorCode {}
CALL label empty_malformed argc 3 code 0 result ::oo::Obj43 errorCode {}
DECL label space_malformed code 1 result {too many fields in argument specifier "a b c"}
OWN label inherited_after_empty_removal code 0 constructor {}
CALL label inherited_after_empty_removal argc 0 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
CALL label inherited_after_empty_removal argc 1 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
CALL label inherited_after_empty_removal argc 2 code 0 result ::oo::Obj52 errorCode {}
CALL label inherited_after_empty_removal argc 3 code 1 result {wrong # args: should be "::original_lifecycle_child new a b"} errorCode {TCL WRONGARGS}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 0.84-9-g5bac7c9 tcloo_command 0
UNAVAILABLE reason no_original_oo_class_command
```

- Standard error: empty.
