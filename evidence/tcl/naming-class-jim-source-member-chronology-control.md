# Tcl execution evidence

## Question

> Public class-package chronology can distinguish sequential external methods, genuine two-word procedures before or after a factory, and BASES references to current, moved, command-aliased, deleted or replaced dispatchers. Conditional Compiler class-reference syntax cannot answer these native execution questions.

> What public Jim class-package results follow sequential external method declarations, two-word procedures before/after a class factory, and BASES references to current, moved, command-aliased, deleted or nonclass command names?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Original fixed ASCII LF CLI program with no raw/counting NUL, non-ASCII, continuation or control escape. One fresh process per provider shares package discovery and eight sequential controls using distinct authored class names; errors retain their partial public effects and do not establish an unexecuted branch. Six exact source/executable/artifact/stream joins are checked independently. Jim controls concern its oo package and ordinary alias command; C package/class unavailability is scoped only to these captured stock installations and is separate from TclOO. Source package/build pins establish retained associations, not an independently proved compiled-source/object/header explanation. No Rust assertion result, Native identity, successful source-carrier construction, allocation, navigation or edit permission follows.

## Script

```tcl-visible
puts [list version [info patchlevel] initial_class [info commands class] alias_command [info commands alias]]
set package_code [catch {package require oo} package_result]
puts [list oo_package $package_code $package_result class_command [info commands class]]
if {[llength [info commands class]] == 0} {
    puts [list jim_class_controls not_applicable no_class_command]
} else {
    proc observe {label script} {
        set code [catch {uplevel 1 $script} result]
        puts [list $label $code $result]
    }
    observe sequential_members {
        class Accum {x 1}
        Accum method first {} {return FIRST}
        Accum method second {} {return SECOND}
        set obj [Accum new]
        list [$obj first] [$obj second] [$obj get x]
    }
    observe two_word_before_after {
        proc {Chrono before} {} {return BEFORE}
        class Chrono {x 1}
        proc {Chrono after} {} {return AFTER}
        set obj [Chrono new]
        list [$obj before] [$obj after]
    }
    observe moved_earlier_two_word {
        proc {MovedProc before} {} {return BEFORE}
        rename {MovedProc before} held_before
        class MovedProc {}
        set obj [MovedProc new]
        $obj before
    }
    observe bases_current {
        class Base {base 11}
        Base method inherited {} {return BASE}
        class Child {Base} {own 22}
        set obj [Child new]
        list [$obj inherited] [$obj get base] [$obj get own]
    }
    observe bases_moved_factory {
        class MoveBase {base 31}
        MoveBase method inherited {} {return MOVE_BASE}
        rename MoveBase HeldBase
        class MoveChild {HeldBase} {own 32}
        set obj [MoveChild new]
        list [$obj inherited] [$obj get base] [$obj get own]
    }
    observe bases_command_alias {
        class AliasBase {base 41}
        AliasBase method inherited {} {return ALIAS_BASE}
        alias AliasName AliasBase
        class AliasChild {AliasName} {own 42}
        set obj [AliasChild new]
        list [$obj inherited] [$obj get base] [$obj get own]
    }
    observe bases_known_deleted {
        class DeletedBase {base 51}
        rename DeletedBase {}
        class DeletedChild {DeletedBase} {}
    }
    observe bases_known_nonclass {
        class ShadowedBase {base 61}
        proc ShadowedBase args {return NON_CLASS}
        class ShadowedChild {ShadowedBase} {}
    }
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF CLI file; package require oo availability, caught public class/object results. Source contains no raw or counted NUL, non-ASCII, line continuations or control escapes. No internal object/header/native argv/cache/frame/currentness claim. C stock class-command absence is not applicable, separate from TclOO availability.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF CLI file; package require oo availability, caught public class/object results. Source contains no raw or counted NUL, non-ASCII, line continuations or control escapes. No internal object/header/native argv/cache/frame/currentness claim. C stock class-command absence is not applicable, separate from TclOO availability.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF CLI file; package require oo availability, caught public class/object results. Source contains no raw or counted NUL, non-ASCII, line continuations or control escapes. No internal object/header/native argv/cache/frame/currentness claim. C stock class-command absence is not applicable, separate from TclOO availability.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF CLI file; package require oo availability, caught public class/object results. Source contains no raw or counted NUL, non-ASCII, line continuations or control escapes. No internal object/header/native argv/cache/frame/currentness claim. C stock class-command absence is not applicable, separate from TclOO availability.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF CLI file; package require oo availability, caught public class/object results. Source contains no raw or counted NUL, non-ASCII, line continuations or control escapes. No internal object/header/native argv/cache/frame/currentness claim. C stock class-command absence is not applicable, separate from TclOO availability.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF CLI file; package require oo availability, caught public class/object results. Source contains no raw or counted NUL, non-ASCII, line continuations or control escapes. No internal object/header/native argv/cache/frame/currentness claim. C stock class-command absence is not applicable, separate from TclOO availability.
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9 initial_class class alias_command alias
oo_package 0 1.0 class_command class
sequential_members 0 {FIRST SECOND 1}
two_word_before_after 0 {BEFORE AFTER}
moved_earlier_two_word 1 {MovedProc, unknown method "before": should be classname, classvars, constructor, defaultconstructor, destroy, eval, finalize, get, method, methods, new, vars}
bases_current 0 {BASE 11 22}
bases_moved_factory 1 {invalid command name "HeldBase inherited"}
bases_command_alias 1 {invalid command name "AliasName inherited"}
bases_known_deleted 1 {invalid command name "DeletedBase"}
bases_known_nonclass 1 {missing value to go with key}
```

- Standard error: empty.
