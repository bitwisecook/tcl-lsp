# BIG-IP follow-up: expressions

## Question

How do BIG-IP Tcl contexts evaluate matches and starts_with, including their precedence relative to not and or for the tested operands? This constrains expression parsing and evaluation in the LSP, compiler and VM.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

```tcl-visible
set rows {}
foreach expression {
    {"abcd" matches "a*"}
    {"a*" matches "a*"}
    {"a*" matches "abcd"}
    {"abcd" matches "a.*"}
    {"abcd" matches "bc"}
    {"abcd" matches "abcd"}
    {1 or 0 matches 0}
    {not "abc" starts_with "a"}
    {not ("abc" starts_with "a")}
} {
    set rc [catch {expr $expression} result]
    lappend rows [list $expression $rc $result]
}
set rows
```

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{{"abcd" matches "a*"} 0 1} {{"a*" matches "a*"} 0 1} {{"a*" matches "abcd"} 0 0} {{"abcd" matches "a.*"} 0 0} {{"abcd" matches "bc"} 0 0} {{"abcd" matches "abcd"} 0 1} {{1 or 0 matches 0} 0 1} {{not "abc" starts_with "a"} 1 {can't use non-numeric string as operand of "!"}} {{not ("abc" starts_with "a")} 0 0}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.

### TmshCliScript; IAppImplementation

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TmshCliScript; IAppImplementation.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{{"abcd" matches "a*"} 0 1} {{"a*" matches "a*"} 0 1} {{"a*" matches "abcd"} 0 0} {{"abcd" matches "a.*"} 0 0} {{"abcd" matches "bc"} 0 0} {{"abcd" matches "abcd"} 0 1} {{1 or 0 matches 0} 0 1} {{not "abc" starts_with "a"} 1 {can't use non-numeric string as operand of "!"}} {{not ("abc" starts_with "a")} 0 0}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.

### ICallScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: ICallScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{{"abcd" matches "a*"} 0 1} {{"a*" matches "a*"} 0 1} {{"a*" matches "abcd"} 0 0} {{"abcd" matches "a.*"} 0 0} {{"abcd" matches "bc"} 0 0} {{"abcd" matches "abcd"} 0 1} {{1 or 0 matches 0} 0 1} {{not "abc" starts_with "a"} 1 {can't use non-numeric string as operand of "!"}} {{not ("abc" starts_with "a")} 0 0}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
