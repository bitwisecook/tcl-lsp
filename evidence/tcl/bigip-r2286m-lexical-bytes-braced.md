# BIG-IP follow-up: lexical bytes braced

## Question

How does Tcl parse a braced variable token containing name bytes produced by binary format? This affects lexical name recognition in the LSP and compiler.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix __tcl_lsp_2286_r2286m_lex_bytes_
set pre $prefix
append pre [binary format H* c3a9]
set decomposed $prefix
append decomposed [binary format H* 65cc81]
set rows {}
foreach pair [list [list pre $pre PRE_VALUE] [list decomposed $decomposed DECOMPOSED_VALUE]] {
    set label [lindex $pair 0]
    set name [lindex $pair 1]
    set value [lindex $pair 2]
    set write_rc [catch {set $name $value} write_result]
    set script "set \${"; append script $name; append script "}"
    binary scan $script H* script_hex
    set rc [catch {eval $script} result]
    binary scan $result H* result_hex
    lappend rows [list $label write $write_rc $write_result script_hex $script_hex eval $rc $result_hex]
}
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
{pre write 0 PRE_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735fc3a97d eval 1 63616e2774207265616420225052455f56414c5545223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65cc817d eval 1 63616e2774207265616420224445434f4d504f5345445f56414c5545223a206e6f2073756368207661726961626c65}⟦EOF⟧
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
{pre write 0 PRE_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735fc3a97d eval 1 63616e2774207265616420225052455f56414c5545223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65cc817d eval 1 63616e2774207265616420224445434f4d504f5345445f56414c5545223a206e6f2073756368207661726961626c65}⟦EOF⟧
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
{pre write 0 PRE_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735fc3a97d eval 1 63616e2774207265616420225052455f56414c5545223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65cc817d eval 1 63616e2774207265616420224445434f4d504f5345445f56414c5545223a206e6f2073756368207661726961626c65}⟦EOF⟧
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
{pre write 0 PRE_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735fc3a97d eval 1 63616e2774207265616420225052455f56414c5545223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420247b5f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65cc817d eval 1 63616e2774207265616420224445434f4d504f5345445f56414c5545223a206e6f2073756368207661726961626c65}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.
