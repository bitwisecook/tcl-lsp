# BIG-IP follow-up: unicode format reverse

## Question

Do precomposed and decomposed variable names produced by format %c remain distinct when written in the reverse order? This affects Unicode name resolution in the LSP, compiler and VM.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix __tcl_lsp_2286_r2286m_unicode_
set pre $prefix
append pre [format %c 233]
set decomposed $prefix
append decomposed e [format %c 769]
set rows [list [list lengths [string length $pre] [string length $decomposed]]]
foreach name [list $pre $decomposed] {
    set raw_rc [catch {binary scan $name H* raw} raw_result]
    if {$raw_rc != 0} {set raw $raw_result}
    set rc [catch {encoding convertto utf-8 $name} encoded]
    if {$rc == 0} {binary scan $encoded H* encoded}
    lappend rows [list raw_conversion $raw_rc $raw utf8_conversion $rc $encoded]
}
set rc [catch {set $decomposed DECOMPOSED} result]
lappend rows [list write_decomposed $rc $result]
set rc [catch {set $pre PRECOMPOSED} result]
lappend rows [list write_pre $rc $result]
foreach name [list $pre $decomposed] {
    set rc [catch {set $name} result]
    lappend rows [list read $rc $result]
}
set rc [catch {unset $decomposed} result]
lappend rows [list unset_decomposed $rc $result]
set rc [catch {set $pre} result]
lappend rows [list remaining_pre $rc $result]
foreach name [list $pre $decomposed] {catch {unset $name}}
set rows
```

## Results

### TMM 0:0, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{lengths 31 32} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655fe9 utf8_conversion 1 {invalid command name "encoding"}} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655f6501 utf8_conversion 1 {invalid command name "encoding"}} {write_decomposed 0 DECOMPOSED} {write_pre 0 PRECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_decomposed 0 {}} {remaining_pre 0 PRECOMPOSED}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{lengths 31 32} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655fe9 utf8_conversion 1 {invalid command name "encoding"}} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655f6501 utf8_conversion 1 {invalid command name "encoding"}} {write_decomposed 0 DECOMPOSED} {write_pre 0 PRECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_decomposed 0 {}} {remaining_pre 0 PRECOMPOSED}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{lengths 31 32} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655fe9 utf8_conversion 1 {invalid command name "encoding"}} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655f6501 utf8_conversion 1 {invalid command name "encoding"}} {write_decomposed 0 DECOMPOSED} {write_pre 0 PRECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_decomposed 0 {}} {remaining_pre 0 PRECOMPOSED}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{lengths 31 32} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655fe9 utf8_conversion 1 {invalid command name "encoding"}} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655f6501 utf8_conversion 1 {invalid command name "encoding"}} {write_decomposed 0 DECOMPOSED} {write_pre 0 PRECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_decomposed 0 {}} {remaining_pre 0 PRECOMPOSED}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.
