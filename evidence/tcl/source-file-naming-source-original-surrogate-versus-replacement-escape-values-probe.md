# Tcl source-file execution evidence

## Question

> Lossy Unicode presentation can replace an unrepresentable logical source value with U+FFFD. If the original source escape is observably distinct from the replacement escape, that presentation cannot supply source identity or equality facts.

> Do the original source escapes \uD800 and \uFFFD have the same public string value, and what do the exact adjacent, self, valid Unicode and braced literal controls report?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> The source is ASCII LF text with no raw NUL or direct non-ASCII input. Quoted \u escapes are evaluated by the CLI; braced words retain literal escape spelling. Public commands emit boolean and length rows, not private string bytes or object identity. Compiler version and unlisted inherited environment were not recorded. BIG-IP was not tested.

Other assumptions were not separately recorded.

## Script

```tcl-visible
puts "SOURCE_SURROGATE_REPLACEMENT|[string equal "\uD800" "\uFFFD"]|[string length "\uD800"]|[string length "\uFFFD"]"
puts "SOURCE_ADJACENT_SURROGATES|[string equal "\uD800" "\uD801"]|[string length "\uD801"]"
puts "SOURCE_REPLACEMENT_SELF|[string equal "\uFFFD" "\uFFFD"]"
puts "SOURCE_VALID_UNICODE|[string equal "caf\u00e9" "cafe"]|[string length "caf\u00e9"]"
puts "SOURCE_BRACED_ESCAPE|[string equal {\uD800} "\uD800"]|[string length {\uD800}]"
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 initial_class {} alias_command {}
oo_package 1 {can't find package oo} class_command {}
jim_class_controls not_applicable no_class_command
```

- Standard error: empty.

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: source file passed to the interpreter.
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
