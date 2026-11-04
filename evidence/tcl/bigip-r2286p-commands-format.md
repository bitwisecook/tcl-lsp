# BIG-IP cross-context naming: commands format

## Question

How are precomposed and decomposed command names produced by format %c created and resolved? This affects command-name modeling in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix ::__tcl_lsp_2286_r2286p_command_format_
set pre $prefix
append pre [format %c 233]
set decomposed $prefix
append decomposed e [format %c 769]
set rows {}
foreach pair [list [list pre $pre PRE_COMMAND] [list decomposed $decomposed DECOMPOSED_COMMAND]] {
    set label [lindex $pair 0]
    set ns [lindex $pair 1]
    set value [lindex $pair 2]
    set procname $ns
    append procname ::identity
    set renamed $procname
    append renamed _renamed
    binary scan $ns H* ns_hex
    set rc [catch {namespace eval $ns {}} result]
    lappend rows [list namespace_create $label $ns_hex $rc $result]
    set body [list return $value]
    set rc [catch {eval [list proc $procname {} $body]} result]
    lappend rows [list proc_create $label $rc $result]
    set rc [catch {namespace which -command $procname} result]
    lappend rows [list lookup_original $label $rc $result]
    set rc [catch {eval [list $procname]} result]
    lappend rows [list call_original $label $rc $result]
    set rc [catch {rename $procname $renamed} result]
    lappend rows [list rename $label $rc $result]
    set rc [catch {namespace which -command $procname} result]
    lappend rows [list lookup_old_after_rename $label $rc $result]
    set rc [catch {namespace which -command $renamed} result]
    lappend rows [list lookup_new_after_rename $label $rc $result]
    set rc [catch {eval [list $renamed]} result]
    lappend rows [list call_renamed $label $rc $result]
}
foreach ns [list $pre $decomposed] {catch {namespace delete $ns}}
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
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745fe9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745f6501 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745fe9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745f6501 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745fe9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745f6501 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745fe9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745f6501 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### IAppImplementation

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: IAppImplementation.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745fe9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745f6501 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
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
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745fe9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745f6501 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.

### TmshCliScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TmshCliScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745fe9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_format_⟦BYTE:E9⟧::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f666f726d61745f6501 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_format_e⟦U+0001⟧::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
