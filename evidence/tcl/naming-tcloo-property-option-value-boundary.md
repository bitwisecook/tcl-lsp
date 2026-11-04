# Tcl execution evidence

## Question

> An option-looking script value can be confused with another property option, while quoted/abbreviated selectors, multiple property names and repeated getter options require exact width and stored-script observations. Introspection refusal cannot establish that a default property method is absent.

> Which post-keyword words are property names, getter/setter script values and actual options when a script value is itself -get/-set, an option is quoted or abbreviated, names follow option-value pairs, or a getter option repeats?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Original fixed ASCII LF CLI source, separate fresh process per provider, exact configurable-command discovery, stored info-class definitions and caught configurable reads/writes/declaration failures. Public case labels, whole definitions/results and exact errorCode lists are retained. The custom method scripts are inspectable; failure to inspect the default q getter says nothing about absence. Native input has no raw/counting NUL, non-ASCII, continuation or control escape. Three named Rust source bindings are independently selected layout/geometry coverage only: their dynamic/Unicode/opaque/raw-zero cases are not measured by this ASCII native program and assert no current installed method or entered frame. Complete executable/source/library/header/build associations are checked independently; excerpts do not prove compiled-source/object equivalence.

## Script

```tcl-visible
# Original ASCII source-file property layout controls. These observations concern
# stock Tcl commands, stored method scripts and configure calls, not Native
# source-carrier admission, method headers, compiler activation or BIG-IP.
puts [list META patchlevel [info patchlevel] configurable_command [llength [info commands ::oo::configurable]]]
if {![llength [info commands ::oo::configurable]]} {
    puts [list UNAVAILABLE reason no_original_oo_configurable_command]
} else {
    proc ::-set {} {return getter_option_value}
    ::oo::configurable create ::property_width_p {
        property p -get {-set} "-set" {set ::property_width_seen $value}
    }
    foreach method {<ReadProp-p> <WriteProp-p>} {
        set code [catch [list info class definition ::property_width_p $method] result]
        puts [list DEFINITION label option_like_getter method $method code $code definition $result]
    }
    set instance [::property_width_p new]
    set code [catch {$instance configure -p} result]
    puts [list CALL label option_like_getter operation read code $code result $result]
    set ::property_width_seen not_observed
    set code [catch {$instance configure -p setter_value} result]
    puts [list CALL label option_like_getter operation write code $code result $result seen $::property_width_seen]
    $instance destroy
    ::property_width_p destroy
    rename ::-set {}

    ::oo::configurable create ::property_width_multi {
        property p -g {-set} q "-s" {set ::property_width_q $value}
    }
    foreach method {<ReadProp-p> <ReadProp-q> <WriteProp-q>} {
        set code [catch [list info class definition ::property_width_multi $method] result]
        puts [list DEFINITION label multiple_names_abbreviations method $method code $code definition $result]
    }
    ::property_width_multi destroy

    ::oo::configurable create ::property_width_repeat {
        property p -get {return superseded} "-get" {-set} -set {set ::property_width_repeat_seen $value}
    }
    foreach method {<ReadProp-p> <WriteProp-p>} {
        set code [catch [list info class definition ::property_width_repeat $method] result]
        puts [list DEFINITION label repeated_getter method $method code $code definition $result]
    }
    ::property_width_repeat destroy

    foreach {label definition} {
        missing_value {property p -get}
        unknown_option {property p -unknown {return hidden}}
        ambiguous_option {property p - {return hidden}}
        invalid_kind {property p -kind r -get {return hidden}}
    } {
        set code [catch [list ::oo::configurable create ::property_width_invalid $definition] result]
        set ec {}
        if {$code != 0 && [info exists ::errorCode]} {set ec $::errorCode}
        puts [list DECL label $label code $code result $result errorCode $ec]
        if {$code == 0} {::property_width_invalid destroy}
    }
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.4.20 configurable_command 0
UNAVAILABLE reason no_original_oo_configurable_command
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.5.19 configurable_command 0
UNAVAILABLE reason no_original_oo_configurable_command
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.6.18 configurable_command 0
UNAVAILABLE reason no_original_oo_configurable_command
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 9.0.4 configurable_command 1
DEFINITION label option_like_getter method <ReadProp-p> code 0 definition {{} -set}
DEFINITION label option_like_getter method <WriteProp-p> code 0 definition {value {set ::property_width_seen $value}}
CALL label option_like_getter operation read code 0 result getter_option_value
CALL label option_like_getter operation write code 0 result {} seen setter_value
DEFINITION label multiple_names_abbreviations method <ReadProp-p> code 0 definition {{} -set}
DEFINITION label multiple_names_abbreviations method <ReadProp-q> code 1 definition {definition not available for this kind of method}
DEFINITION label multiple_names_abbreviations method <WriteProp-q> code 0 definition {value {set ::property_width_q $value}}
DEFINITION label repeated_getter method <ReadProp-p> code 0 definition {{} -set}
DEFINITION label repeated_getter method <WriteProp-p> code 0 definition {value {set ::property_width_repeat_seen $value}}
DECL label missing_value code 1 result {missing body to go with -get option} errorCode {TCL WRONGARGS}
DECL label unknown_option code 1 result {bad option "-unknown": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -unknown}
DECL label ambiguous_option code 1 result {ambiguous option "-": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -}
DECL label invalid_kind code 1 result {ambiguous kind "r": must be readable, readwrite, or writable} errorCode {TCL LOOKUP INDEX kind r}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 9.1.0 configurable_command 1
DEFINITION label option_like_getter method <ReadProp-p> code 0 definition {{} -set}
DEFINITION label option_like_getter method <WriteProp-p> code 0 definition {value {set ::property_width_seen $value}}
CALL label option_like_getter operation read code 0 result getter_option_value
CALL label option_like_getter operation write code 0 result {} seen setter_value
DEFINITION label multiple_names_abbreviations method <ReadProp-p> code 0 definition {{} -set}
DEFINITION label multiple_names_abbreviations method <ReadProp-q> code 1 definition {definition not available for this kind of method}
DEFINITION label multiple_names_abbreviations method <WriteProp-q> code 0 definition {value {set ::property_width_q $value}}
DEFINITION label repeated_getter method <ReadProp-p> code 0 definition {{} -set}
DEFINITION label repeated_getter method <WriteProp-p> code 0 definition {value {set ::property_width_repeat_seen $value}}
DECL label missing_value code 1 result {missing body to go with -get option} errorCode {TCL WRONGARGS}
DECL label unknown_option code 1 result {bad option "-unknown": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -unknown}
DECL label ambiguous_option code 1 result {ambiguous option "-": must be -get, -kind, or -set} errorCode {TCL LOOKUP INDEX option -}
DECL label invalid_kind code 1 result {ambiguous kind "r": must be readable, readwrite, or writable} errorCode {TCL LOOKUP INDEX kind r}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF source-file CLI; stock configurable availability, stored info class definition scripts and caught configure calls. No original Native source-carrier/frame/header or BIG-IP authority is measured.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 0.84-9-g5bac7c9 configurable_command 0
UNAVAILABLE reason no_original_oo_configurable_command
```

- Standard error: empty.
