# BIG-IP follow-up: commands cross producer

## Question

Do command names produced by format %c and binary format resolve to the same or different commands for the tested values? This affects command-name modeling in the LSP, compiler and VM.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix ::__tcl_lsp_2286_r2286m_command_cross_
set format_pre $prefix
append format_pre [format %c 233]
set bytes_pre $prefix
append bytes_pre [binary format H* c3a9]
set format_decomposed $prefix
append format_decomposed e [format %c 769]
set bytes_decomposed $prefix
append bytes_decomposed [binary format H* 65cc81]
set rows {}
foreach group [list [list pre $format_pre $bytes_pre] [list decomposed $format_decomposed $bytes_decomposed]] {
    set label [lindex $group 0]
    set format_ns [lindex $group 1]
    set bytes_ns [lindex $group 2]
    set format_proc $format_ns
    append format_proc ::identity
    set bytes_proc $bytes_ns
    append bytes_proc ::identity
    set renamed $bytes_proc
    append renamed _renamed
    foreach pair [list [list format $format_ns] [list bytes $bytes_ns]] {
        set ns [lindex $pair 1]
        binary scan $ns H* raw
        set rc [catch {namespace eval $ns {}} result]
        lappend rows [list namespace_create $label [lindex $pair 0] [string length $ns] $raw $rc $result]
    }
    set rc [catch {eval [list proc $format_proc {} [list return FORMAT_COMMAND]]} result]
    lappend rows [list proc_create $label format $rc $result]
    set rc [catch {eval [list proc $bytes_proc {} [list return BYTES_COMMAND]]} result]
    lappend rows [list proc_create $label bytes $rc $result]
    foreach pair [list [list format $format_proc] [list bytes $bytes_proc]] {
        set name [lindex $pair 1]
        set lookup_rc [catch {namespace which -command $name} lookup]
        set call_rc [catch {eval [list $name]} value]
        lappend rows [list before_rename $label [lindex $pair 0] $lookup_rc $lookup $call_rc $value]
    }
    set rc [catch {rename $bytes_proc $renamed} result]
    lappend rows [list rename_bytes $label $rc $result]
    foreach pair [list [list format $format_proc] [list bytes $bytes_proc] [list renamed $renamed]] {
        set name [lindex $pair 1]
        set lookup_rc [catch {namespace which -command $name} lookup]
        set call_rc [catch {eval [list $name]} value]
        lappend rows [list after_rename $label [lindex $pair 0] $lookup_rc $lookup $call_rc $value]
    }
}
foreach ns [list $format_pre $bytes_pre $format_decomposed $bytes_decomposed] {catch {namespace delete $ns}}
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
{namespace_create pre format 39 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fe9 0 {}} {namespace_create pre bytes 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fc3a9 0 {}} {proc_create pre format 0 {}} {proc_create pre bytes 0 {}} {before_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {before_rename pre bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes pre 0 {}} {after_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {after_rename pre bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename pre renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND} {namespace_create decomposed format 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f6501 0 {}} {namespace_create decomposed bytes 41 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f65cc81 0 {}} {proc_create decomposed format 0 {}} {proc_create decomposed bytes 0 {}} {before_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {before_rename decomposed bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes decomposed 0 {}} {after_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {after_rename decomposed bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename decomposed renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND}⟦EOF⟧
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
{namespace_create pre format 39 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fe9 0 {}} {namespace_create pre bytes 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fc3a9 0 {}} {proc_create pre format 0 {}} {proc_create pre bytes 0 {}} {before_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {before_rename pre bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes pre 0 {}} {after_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {after_rename pre bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename pre renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND} {namespace_create decomposed format 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f6501 0 {}} {namespace_create decomposed bytes 41 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f65cc81 0 {}} {proc_create decomposed format 0 {}} {proc_create decomposed bytes 0 {}} {before_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {before_rename decomposed bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes decomposed 0 {}} {after_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {after_rename decomposed bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename decomposed renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND}⟦EOF⟧
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
{namespace_create pre format 39 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fe9 0 {}} {namespace_create pre bytes 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fc3a9 0 {}} {proc_create pre format 0 {}} {proc_create pre bytes 0 {}} {before_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {before_rename pre bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes pre 0 {}} {after_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {after_rename pre bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename pre renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND} {namespace_create decomposed format 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f6501 0 {}} {namespace_create decomposed bytes 41 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f65cc81 0 {}} {proc_create decomposed format 0 {}} {proc_create decomposed bytes 0 {}} {before_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {before_rename decomposed bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes decomposed 0 {}} {after_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {after_rename decomposed bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename decomposed renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND}⟦EOF⟧
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
{namespace_create pre format 39 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fe9 0 {}} {namespace_create pre bytes 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735fc3a9 0 {}} {proc_create pre format 0 {}} {proc_create pre bytes 0 {}} {before_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {before_rename pre bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes pre 0 {}} {after_rename pre format 0 ::__tcl_lsp_2286_r2286m_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {after_rename pre bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename pre renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND} {namespace_create decomposed format 40 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f6501 0 {}} {namespace_create decomposed bytes 41 3a3a5f5f74636c5f6c73705f323238365f72323238366d5f636f6d6d616e645f63726f73735f65cc81 0 {}} {proc_create decomposed format 0 {}} {proc_create decomposed bytes 0 {}} {before_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {before_rename decomposed bytes 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes decomposed 0 {}} {after_rename decomposed format 0 ::__tcl_lsp_2286_r2286m_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {after_rename decomposed bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286m_command_cross_é::identity"}} {after_rename decomposed renamed 0 ::__tcl_lsp_2286_r2286m_command_cross_é::identity_renamed 0 BYTES_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.
