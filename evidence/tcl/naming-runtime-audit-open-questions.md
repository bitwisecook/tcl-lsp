# Naming questions left open by the runtime consumer audit

## Question

How do original C Tcl releases treat five naming questions the runtime consumer audit left open or raised, so that the runtime and VM can be held to them?

1. A Tcl-level NUL inside a namespace, command or variable name: is the name kept whole or cut at the NUL?
2. `namespace which -variable` and `info exists` on declared-only variables, array elements and missing names, against `namespace upvar` on an element.
3. Names that outlive a deleted namespace recreated under the same name: imports, `namespace origin`, ensembles and `upvar` links.
4. Which variable operations report a link into a deleted namespace as `upvar refers to variable in deleted namespace`, and which as `no such variable`.
5. Which error code an unset reports when the name is missing, when it reaches a variable entry without a value, and when an array element is missing.

Each case runs at global level inside `catch`; NUL bytes in results are shown as `<NUL>`. The observations bind only these scripts on these releases.

## Assumptions

- The NUL cases use the backslash escape `\x00` in source, so the value holds a Tcl NUL character, not a raw byte from a C caller. A non-hex character always follows the escape, since 8.4 and 8.5 read every following hex digit into `\x`.
- `info commands` results are sorted with `lsort`; the order of a sorted list containing a NUL differs between releases because the comparison differs.
- Cases run in one interpreter in order, so a later case sees what an earlier one created; `q-upvar-elem` and `re-ensemble` need commands 8.4 lacks.
- BIG-IP and Jim were not run.

## Script

```tcl-visible
proc show {label script} {
    set code [catch {uplevel #0 $script} result]
    regsub -all {\x00} $result {<NUL>} result
    puts "$label: $code [list $result]"
}

# NUL inside names
show nul-ns-eval {namespace eval "a\x00z" {variable v 1}; namespace children :: "a*"}
show nul-ns-exists {namespace exists "::a\x00z"}
show nul-ns-tail {namespace tail "::a\x00z::c"}
show nul-ns-qualifiers {namespace qualifiers "::a\x00z::c"}
show nul-proc {proc "p\x00q" {} {return ok}; list [lsort [info commands "p*"]] [catch {"p\x00q"} r] $r}
show nul-var {set "v\x00w" 1; info exists "v\x00w"}
show nul-var-ns {set "::a\x00z::v" 2; set "::a\x00z::v"}
show nul-rename {rename "p\x00q" "r\x00s"; lsort [info commands "r*"]}
show nul-which {namespace which -command "r\x00s"}

# namespace query versus variable flags
show q-which-var {namespace eval ::q {variable s 1; array set a {k 1}}; namespace which -variable ::q::s}
show q-which-elem {namespace which -variable ::q::a(k)}
show q-which-missing {namespace which -variable ::q::missing}
show q-info-exists-elem {info exists ::q::a(k)}
show q-upvar-elem {namespace eval ::q {proc f {} {namespace upvar ::q a(k) x; set x}}; ::q::f}
show q-which-declared {namespace eval ::q {variable decl}; namespace which -variable ::q::decl}
show q-info-vars-declared {lsort [info vars ::q::*]}

# deleted and recreated namespaces under one name
show re-import {
    namespace eval ::src {proc hello {} {return old}; namespace export hello}
    namespace eval ::dst {namespace import ::src::hello}
    namespace delete ::src
    namespace eval ::src {proc hello {} {return new}; namespace export hello}
    list [info commands ::dst::*] [catch {::dst::hello} r] $r
}
show re-origin {
    namespace eval ::s2 {proc h {} {return old}; namespace export h}
    namespace eval ::d2 {namespace import ::s2::h}
    set o [namespace origin ::d2::h]
    namespace delete ::s2
    namespace eval ::s2 {proc h {} {return new}}
    list $o [catch {namespace origin ::d2::h} r] $r
}
show re-current-in-proc {
    namespace eval ::n3 {proc where {} {namespace current}}
    set w [::n3::where]
    namespace delete ::n3
    namespace eval ::n3 {}
    list $w [namespace exists ::n3] [info commands ::n3::*]
}
show re-ensemble {
    namespace eval ::en {proc a {} {return A}; namespace export a; namespace ensemble create}
    set before [::en a]
    namespace delete ::en
    namespace eval ::en {proc a {} {return B}; namespace export a}
    list $before [catch {::en a} r] $r
}
show re-variable-link {
    namespace eval ::lv {variable x 1}
    proc g {} {upvar #0 ::lv::x y; namespace delete ::lv; namespace eval ::lv {variable x 2}; list [catch {set y} r] $r $::lv::x}
    g
}
proc dead_link {op} {
    namespace eval ::lw {variable x 1}
    upvar #0 ::lw::x y
    namespace delete ::lw
    namespace eval ::lw {variable x 2}
    set ::errorCode NONE
    switch -- $op {
        read {set code [catch {set y} r]}
        write {set code [catch {set y 3} r]}
        unset {set code [catch {unset y} r]}
        incr {set code [catch {incr y} r]}
    }
    list $code $r $::errorCode $::lw::x
}
foreach op {read write unset incr} {show re-link-$op [list dead_link $op]}
proc unset_code {script} {
    set ::errorCode NONE
    list [catch {uplevel #0 $script} r] $r $::errorCode
}
show unset-missing {unset_code {unset nosuch}}
show unset-declared {unset_code {namespace eval ::u {variable z; unset z}}}
show unset-linked {unset_code {proc ul {} {upvar #0 gone g; unset g}; ul}}
show unset-local {unset_code {proc uu {} {set l 1; unset l; unset l}; uu}}
show unset-element {unset_code {array set arr {}; unset arr(k)}}
show read-element {unset_code {array set arr {}; set arr(k)}}
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: `tclsh8.4` with the script file as its argument.
- Exit status: 0
- Standard output:

```text-visible
nul-ns-eval: 0 ::a<NUL>z
nul-ns-exists: 0 1
nul-ns-tail: 0 c
nul-ns-qualifiers: 0 ::a<NUL>z
nul-proc: 0 {{package pid proc puts pwd p<NUL>q} 0 ok}
nul-var: 0 1
nul-var-ns: 0 2
nul-rename: 0 {read regexp regsub rename return r<NUL>s}
nul-which: 0 ::r<NUL>s
q-which-var: 0 ::q::s
q-which-elem: 0 {}
q-which-missing: 0 {}
q-info-exists-elem: 0 1
q-upvar-elem: 1 {bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
q-which-declared: 0 ::q::decl
q-info-vars-declared: 0 {::q::a ::q::decl ::q::s}
re-import: 0 {{} 1 {invalid command name "::dst::hello"}}
re-origin: 0 {::s2::h 1 {invalid command name "::d2::h"}}
re-current-in-proc: 0 {::n3 1 {}}
re-ensemble: 1 {bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
re-variable-link: 0 {1 {can't read "y": no such variable} 2}
re-link-read: 0 {1 {can't read "y": no such variable} NONE 2}
re-link-write: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} NONE 2}
re-link-unset: 0 {1 {can't unset "y": no such variable} NONE 2}
re-link-incr: 0 {1 {can't read "y": no such variable} NONE 2}
unset-missing: 0 {1 {can't unset "nosuch": no such variable} NONE}
unset-declared: 0 {1 {can't unset "z": no such variable} NONE}
unset-linked: 0 {1 {can't unset "g": no such variable} NONE}
unset-local: 0 {1 {can't unset "l": no such variable} NONE}
unset-element: 0 {1 {can't unset "arr(k)": no such element in array} NONE}
read-element: 0 {1 {can't read "arr(k)": no such element in array} NONE}
```

- Standard error: empty.

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: `tclsh8.5` with the script file as its argument.
- Exit status: 0
- Standard output:

```text-visible
nul-ns-eval: 0 ::a<NUL>z
nul-ns-exists: 0 1
nul-ns-tail: 0 c
nul-ns-qualifiers: 0 ::a<NUL>z
nul-proc: 0 {{package pid proc puts pwd p<NUL>q} 0 ok}
nul-var: 0 1
nul-var-ns: 0 2
nul-rename: 0 {read regexp regsub rename return r<NUL>s}
nul-which: 0 ::r<NUL>s
q-which-var: 0 ::q::s
q-which-elem: 0 {}
q-which-missing: 0 {}
q-info-exists-elem: 0 1
q-upvar-elem: 0 1
q-which-declared: 0 ::q::decl
q-info-vars-declared: 0 {::q::a ::q::decl ::q::s}
re-import: 0 {{} 1 {invalid command name "::dst::hello"}}
re-origin: 0 {::s2::h 1 {invalid command name "::d2::h"}}
re-current-in-proc: 0 {::n3 1 {}}
re-ensemble: 0 {A 1 {invalid command name "::en"}}
re-variable-link: 0 {1 {can't read "y": no such variable} 2}
re-link-read: 0 {1 {can't read "y": no such variable} NONE 2}
re-link-write: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} NONE 2}
re-link-unset: 0 {1 {can't unset "y": no such variable} NONE 2}
re-link-incr: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} NONE 2}
unset-missing: 0 {1 {can't unset "nosuch": no such variable} NONE}
unset-declared: 0 {1 {can't unset "z": no such variable} NONE}
unset-linked: 0 {1 {can't unset "g": no such variable} NONE}
unset-local: 0 {1 {can't unset "l": no such variable} NONE}
unset-element: 0 {1 {can't unset "arr(k)": no such element in array} {TCL LOOKUP ELEMENT k}}
read-element: 0 {1 {can't read "arr(k)": no such element in array} NONE}
```

- Standard error: empty.

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: `tclsh8.6` with the script file as its argument.
- Exit status: 0
- Standard output:

```text-visible
nul-ns-eval: 0 ::a<NUL>z
nul-ns-exists: 0 1
nul-ns-tail: 0 c
nul-ns-qualifiers: 0 ::a<NUL>z
nul-proc: 0 {{package pid proc puts pwd p<NUL>q} 0 ok}
nul-var: 0 1
nul-var-ns: 0 2
nul-rename: 0 {read regexp regsub rename return r<NUL>s}
nul-which: 0 ::r<NUL>s
q-which-var: 0 ::q::s
q-which-elem: 0 {}
q-which-missing: 0 {}
q-info-exists-elem: 0 1
q-upvar-elem: 0 1
q-which-declared: 0 ::q::decl
q-info-vars-declared: 0 {::q::a ::q::decl ::q::s}
re-import: 0 {{} 1 {invalid command name "::dst::hello"}}
re-origin: 0 {::s2::h 1 {invalid command name "::d2::h"}}
re-current-in-proc: 0 {::n3 1 {}}
re-ensemble: 0 {A 1 {invalid command name "::en"}}
re-variable-link: 0 {1 {can't read "y": no such variable} 2}
re-link-read: 0 {1 {can't read "y": no such variable} {TCL READ VARNAME} 2}
re-link-write: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} {TCL WRITE VARNAME} 2}
re-link-unset: 0 {1 {can't unset "y": no such variable} {TCL UNSET VARNAME} 2}
re-link-incr: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} {TCL WRITE VARNAME} 2}
unset-missing: 0 {1 {can't unset "nosuch": no such variable} {TCL LOOKUP VARNAME nosuch}}
unset-declared: 0 {1 {can't unset "z": no such variable} {TCL UNSET VARNAME}}
unset-linked: 0 {1 {can't unset "g": no such variable} {TCL UNSET VARNAME}}
unset-local: 0 {1 {can't unset "l": no such variable} {TCL UNSET VARNAME}}
unset-element: 0 {1 {can't unset "arr(k)": no such element in array} {TCL LOOKUP ELEMENT k}}
read-element: 0 {1 {can't read "arr(k)": no such element in array} {TCL READ VARNAME}}
```

- Standard error: empty.

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: `tclsh9.0` with the script file as its argument.
- Exit status: 0
- Standard output:

```text-visible
nul-ns-eval: 0 ::a<NUL>z
nul-ns-exists: 0 1
nul-ns-tail: 0 c
nul-ns-qualifiers: 0 ::a<NUL>z
nul-proc: 0 {{p<NUL>q package pid proc puts pwd} 0 ok}
nul-var: 0 1
nul-var-ns: 0 2
nul-rename: 0 {r<NUL>s read regexp regsub rename return}
nul-which: 0 ::r<NUL>s
q-which-var: 0 ::q::s
q-which-elem: 0 {}
q-which-missing: 0 {}
q-info-exists-elem: 0 1
q-upvar-elem: 0 1
q-which-declared: 0 ::q::decl
q-info-vars-declared: 0 {::q::a ::q::decl ::q::s}
re-import: 0 {{} 1 {invalid command name "::dst::hello"}}
re-origin: 0 {::s2::h 1 {invalid command name "::d2::h"}}
re-current-in-proc: 0 {::n3 1 {}}
re-ensemble: 0 {A 1 {invalid command name "::en"}}
re-variable-link: 0 {1 {can't read "y": no such variable} 2}
re-link-read: 0 {1 {can't read "y": no such variable} {TCL READ VARNAME} 2}
re-link-write: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} {TCL WRITE VARNAME} 2}
re-link-unset: 0 {1 {can't unset "y": no such variable} {TCL UNSET VARNAME} 2}
re-link-incr: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} {TCL WRITE VARNAME} 2}
unset-missing: 0 {1 {can't unset "nosuch": no such variable} {TCL LOOKUP VARNAME nosuch}}
unset-declared: 0 {1 {can't unset "z": no such variable} {TCL UNSET VARNAME}}
unset-linked: 0 {1 {can't unset "g": no such variable} {TCL UNSET VARNAME}}
unset-local: 0 {1 {can't unset "l": no such variable} {TCL UNSET VARNAME}}
unset-element: 0 {1 {can't unset "arr(k)": no such element in array} {TCL LOOKUP ELEMENT k}}
read-element: 0 {1 {can't read "arr(k)": no such element in array} {TCL READ VARNAME}}
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: `tclsh9.1` with the script file as its argument.
- Exit status: 0
- Standard output:

```text-visible
nul-ns-eval: 0 ::a<NUL>z
nul-ns-exists: 0 1
nul-ns-tail: 0 c
nul-ns-qualifiers: 0 ::a<NUL>z
nul-proc: 0 {{p<NUL>q package pid proc puts pwd} 0 ok}
nul-var: 0 1
nul-var-ns: 0 2
nul-rename: 0 {r<NUL>s read regexp regsub remquo rename return}
nul-which: 0 ::r<NUL>s
q-which-var: 0 ::q::s
q-which-elem: 0 {}
q-which-missing: 0 {}
q-info-exists-elem: 0 1
q-upvar-elem: 0 1
q-which-declared: 0 ::q::decl
q-info-vars-declared: 0 {::q::a ::q::decl ::q::s}
re-import: 0 {{} 1 {invalid command name "::dst::hello"}}
re-origin: 0 {::s2::h 1 {invalid command name "::d2::h"}}
re-current-in-proc: 0 {::n3 1 {}}
re-ensemble: 0 {A 1 {invalid command name "::en"}}
re-variable-link: 0 {1 {can't read "y": no such variable} 2}
re-link-read: 0 {1 {can't read "y": no such variable} {TCL READ VARNAME} 2}
re-link-write: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} {TCL WRITE VARNAME} 2}
re-link-unset: 0 {1 {can't unset "y": no such variable} {TCL UNSET VARNAME} 2}
re-link-incr: 0 {1 {can't set "y": upvar refers to variable in deleted namespace} {TCL WRITE VARNAME} 2}
unset-missing: 0 {1 {can't unset "nosuch": no such variable} {TCL LOOKUP VARNAME nosuch}}
unset-declared: 0 {1 {can't unset "z": no such variable} {TCL UNSET VARNAME}}
unset-linked: 0 {1 {can't unset "g": no such variable} {TCL UNSET VARNAME}}
unset-local: 0 {1 {can't unset "l": no such variable} {TCL UNSET VARNAME}}
unset-element: 0 {1 {can't unset "arr(k)": no such element in array} {TCL LOOKUP ELEMENT k}}
read-element: 0 {1 {can't read "arr(k)": no such element in array} {TCL READ VARNAME}}
```

- Standard error: empty.
