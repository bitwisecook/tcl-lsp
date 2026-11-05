# BIG-IP cross-context naming: commands bytes

## Question

How are precomposed and decomposed command names produced from binary bytes created and resolved? This affects command-name modeling in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-10](fixtures/bigip-tmm-traffic-event-10.md)

- run: `r2286p_command_bytes_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f62797465735fc3a9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f62797465735f65cc81 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### IAppImplementation; ICallScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: IAppImplementation; ICallScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f62797465735fc3a9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f62797465735f65cc81 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
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
{namespace_create pre 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f62797465735fc3a9 0 {}} {proc_create pre 0 {}} {lookup_original pre 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity} {call_original pre 0 PRE_COMMAND} {rename pre 0 {}} {lookup_old_after_rename pre 0 {}} {lookup_new_after_rename pre 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity_renamed} {call_renamed pre 0 PRE_COMMAND} {namespace_create decomposed 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f62797465735f65cc81 0 {}} {proc_create decomposed 0 {}} {lookup_original decomposed 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity} {call_original decomposed 0 DECOMPOSED_COMMAND} {rename decomposed 0 {}} {lookup_old_after_rename decomposed 0 {}} {lookup_new_after_rename decomposed 0 ::__tcl_lsp_2286_r2286p_command_bytes_é::identity_renamed} {call_renamed decomposed 0 DECOMPOSED_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
