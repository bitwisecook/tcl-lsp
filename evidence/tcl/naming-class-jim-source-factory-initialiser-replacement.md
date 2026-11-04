# Tcl execution evidence

## Question

> A current Jim class factory can change a previously authored two-word procedure slot. A conditional source member ledger needs independent public evidence of the exact stock slots and the conditional baseclass case; later authored overrides need their own chronological control.

> Which pre-existing two-word procedure slots are replaced by the current Jim class factory, is baseclass replacement conditional on nonempty BASES, and can a later raw procedure replace a stock member?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Original fixed ASCII LF CLI file with twelve distinct Init_<member> class names, explicit public info-body comparison, two separate baseclass cases, one current-target call and one later object call. Body contents and caught error text are not printed by these controls: the recorded integer codes and equality flags are their exact public observations. No raw/counted NUL, non-ASCII or continuation/control-escape specimen is executed. Source/package/build associations are retained pins, not an independently proved compiled-source explanation. The bounded results concern the captured Jim oo package; all C class branches are unexecuted after package/class discovery, separate from TclOO. No Native object allocation, argv/header/cache/frame identity, source-model admission, general inheritance or edit permission follows.

## Script

```tcl-visible
puts [list version [info patchlevel] initial_class [info commands class]]
set package_code [catch {package require oo} package_result]
puts [list oo_package $package_code $package_result class_command [info commands class]]
if {[llength [info commands class]] == 0} {
    puts [list jim_initialiser_controls not_applicable no_class_command]
} else {
    foreach member {new finalize method vars classvars classname methods defaultconstructor constructor destroy get eval} {
        set classname Init_$member
        set command "$classname $member"
        proc $command args {return STALE}
        set before [info body $command]
        set factory_code [catch {class $classname {}} factory_result]
        set body_code [catch {info body $command} after]
        puts [list initialiser $member $factory_code $body_code [expr {$body_code == 0 && $after eq $before}]]
    }
    proc {WithoutBase baseclass} args {return STALE}
    set before [info body {WithoutBase baseclass}]
    set code [catch {class WithoutBase {}} result]
    set body_code [catch {info body {WithoutBase baseclass}} after]
    puts [list baseclass_without_bases $code $body_code [expr {$body_code == 0 && $after eq $before}]]
    class ActualBase {}
    proc {WithBase baseclass} args {return STALE}
    set before [info body {WithBase baseclass}]
    set code [catch {class WithBase {ActualBase} {}} result]
    set body_code [catch {info body {WithBase baseclass}} after]
    puts [list baseclass_with_base $code $body_code [expr {$body_code == 0 && $after eq $before}]]
    set code [catch {{WithBase baseclass}} result]
    puts [list baseclass_current_target $code $result]
    class Later {}
    proc {Later get} args {return CUSTOM}
    set code [catch {set obj [Later new]; $obj get anything} result]
    puts [list custom_after_factory $code $result]
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF CLI file; explicit public info body equality/catch outcomes and one caught later object call. Body contents are compared, not printed. No native header/object/frame/cache or successful compiler admission claim. C no class command is explicitly not applicable.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 initial_class {}
oo_package 1 {can't find package oo} class_command {}
jim_initialiser_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF CLI file; explicit public info body equality/catch outcomes and one caught later object call. Body contents are compared, not printed. No native header/object/frame/cache or successful compiler admission claim. C no class command is explicitly not applicable.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 initial_class {}
oo_package 1 {can't find package oo} class_command {}
jim_initialiser_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF CLI file; explicit public info body equality/catch outcomes and one caught later object call. Body contents are compared, not printed. No native header/object/frame/cache or successful compiler admission claim. C no class command is explicitly not applicable.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 initial_class {}
oo_package 1 {can't find package oo} class_command {}
jim_initialiser_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF CLI file; explicit public info body equality/catch outcomes and one caught later object call. Body contents are compared, not printed. No native header/object/frame/cache or successful compiler admission claim. C no class command is explicitly not applicable.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 initial_class {}
oo_package 1 {can't find package oo} class_command {}
jim_initialiser_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF CLI file; explicit public info body equality/catch outcomes and one caught later object call. Body contents are compared, not printed. No native header/object/frame/cache or successful compiler admission claim. C no class command is explicitly not applicable.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 initial_class {}
oo_package 1 {can't find package oo} class_command {}
jim_initialiser_controls not_applicable no_class_command
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF CLI file; explicit public info body equality/catch outcomes and one caught later object call. Body contents are compared, not printed. No native header/object/frame/cache or successful compiler admission claim. C no class command is explicitly not applicable.
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9 initial_class class
oo_package 0 1.0 class_command class
initialiser new 0 0 0
initialiser finalize 0 0 0
initialiser method 0 0 0
initialiser vars 0 0 0
initialiser classvars 0 0 0
initialiser classname 0 0 0
initialiser methods 0 0 0
initialiser defaultconstructor 0 0 0
initialiser constructor 0 1 0
initialiser destroy 0 0 0
initialiser get 0 0 0
initialiser eval 0 0 0
baseclass_without_bases 0 0 1
baseclass_with_base 0 0 0
baseclass_current_target 0 ActualBase
custom_after_factory 0 CUSTOM
```

- Standard error: empty.
