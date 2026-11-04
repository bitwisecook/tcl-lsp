# Tcl execution evidence

## Question

> Literal leading roots in class names and two-word procedures require actual public factory/body and object-call controls. Independent alias versus procedure publication recipes cannot establish distinct live comparison slots from stored spellings alone.

> Does the original rooted Jim class factory leave an earlier constructor procedure body inspectable, change an earlier get body, and admit the measured early and later constructor calls?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Original fixed ASCII LF source file with two rooted authored class/procedure names, public caught class and object calls, exact info-body comparison and one later constructor replacement. The earlier constructor observation list is separate from the call result code; the guest error/body contents and generated object values are not printed. No NUL, non-ASCII or multiline/control escape is exercised. Exact public outcomes are independent of conditional Compiler source barriers and pure name publication/comparison recipes. Required executable/library/header/build/source/package pins retain associations, not a compiled-source causal proof.

## Script

```tcl-visible
puts [list version [info patchlevel] initial_class [info commands class]]
set package_code [catch {package require oo} package_result]
puts [list oo_package $package_code $package_result class_command [info commands class]]
if {[llength [info commands class]] == 0} {
    puts [list class_initialiser_scope not_applicable]
    exit 0
}
proc {::RootConstructor constructor} {args} {lappend ::seen_root_constructor $args}
set original_constructor_body [info body {::RootConstructor constructor}]
set create_code [catch {class ::RootConstructor {}} create_result]
set body_code [catch {info body {::RootConstructor constructor}} current_constructor_body]
puts [list rooted_early_constructor $create_code $body_code [expr {$original_constructor_body eq $current_constructor_body}]]
set seen_root_constructor {}
set object_code [catch {::RootConstructor new FIRST} object_result]
puts [list rooted_early_constructor_call $object_code $seen_root_constructor]
proc {::RootGet get} {} {return ORIGINAL_GET}
set original_get_body [info body {::RootGet get}]
set create_code [catch {class ::RootGet {}} create_result]
set body_code [catch {info body {::RootGet get}} current_get_body]
puts [list rooted_early_get $create_code $body_code [expr {$original_get_body eq $current_get_body}]]
proc {::RootConstructor constructor} {args} {lappend ::seen_root_constructor [linsert $args 0 REPLACEMENT]}
set seen_root_constructor {}
set object_code [catch {::RootConstructor new SECOND} object_result]
puts [list rooted_later_constructor_call $object_code $seen_root_constructor]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 initial_class {}
oo_package 1 {can't find package oo} class_command {}
class_initialiser_scope not_applicable
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 initial_class {}
oo_package 1 {can't find package oo} class_command {}
class_initialiser_scope not_applicable
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 initial_class {}
oo_package 1 {can't find package oo} class_command {}
class_initialiser_scope not_applicable
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 initial_class {}
oo_package 1 {can't find package oo} class_command {}
class_initialiser_scope not_applicable
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 initial_class {}
oo_package 1 {can't find package oo} class_command {}
class_initialiser_scope not_applicable
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9 initial_class class
oo_package 0 1.0 class_command class
rooted_early_constructor 0 1 0
rooted_early_constructor_call 1 {}
rooted_early_get 0 0 0
rooted_later_constructor_call 0 {{REPLACEMENT SECOND}}
```

- Standard error: empty.
