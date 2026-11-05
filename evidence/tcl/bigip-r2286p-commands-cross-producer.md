# BIG-IP cross-context naming: commands cross producer

## Question

Do command names produced by format %c and binary format resolve to the same or different commands for the tested values? This affects command-name modeling in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-11](fixtures/bigip-tmm-traffic-event-11.md)

- run: `r2286p_command_cross_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create pre format 39 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735fe9 0 {}} {namespace_create pre bytes 40 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735fc3a9 0 {}} {proc_create pre format 0 {}} {proc_create pre bytes 0 {}} {before_rename pre format 0 ::__tcl_lsp_2286_r2286p_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {before_rename pre bytes 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes pre 0 {}} {after_rename pre format 0 ::__tcl_lsp_2286_r2286p_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {after_rename pre bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286p_command_cross_é::identity"}} {after_rename pre renamed 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity_renamed 0 BYTES_COMMAND} {namespace_create decomposed format 40 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735f6501 0 {}} {namespace_create decomposed bytes 41 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735f65cc81 0 {}} {proc_create decomposed format 0 {}} {proc_create decomposed bytes 0 {}} {before_rename decomposed format 0 ::__tcl_lsp_2286_r2286p_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {before_rename decomposed bytes 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes decomposed 0 {}} {after_rename decomposed format 0 ::__tcl_lsp_2286_r2286p_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {after_rename decomposed bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286p_command_cross_é::identity"}} {after_rename decomposed renamed 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity_renamed 0 BYTES_COMMAND}⟦EOF⟧
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
{namespace_create pre format 39 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735fe9 0 {}} {namespace_create pre bytes 40 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735fc3a9 0 {}} {proc_create pre format 0 {}} {proc_create pre bytes 0 {}} {before_rename pre format 0 ::__tcl_lsp_2286_r2286p_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {before_rename pre bytes 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes pre 0 {}} {after_rename pre format 0 ::__tcl_lsp_2286_r2286p_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {after_rename pre bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286p_command_cross_é::identity"}} {after_rename pre renamed 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity_renamed 0 BYTES_COMMAND} {namespace_create decomposed format 40 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735f6501 0 {}} {namespace_create decomposed bytes 41 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735f65cc81 0 {}} {proc_create decomposed format 0 {}} {proc_create decomposed bytes 0 {}} {before_rename decomposed format 0 ::__tcl_lsp_2286_r2286p_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {before_rename decomposed bytes 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes decomposed 0 {}} {after_rename decomposed format 0 ::__tcl_lsp_2286_r2286p_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {after_rename decomposed bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286p_command_cross_é::identity"}} {after_rename decomposed renamed 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity_renamed 0 BYTES_COMMAND}⟦EOF⟧
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
{namespace_create pre format 39 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735fe9 0 {}} {namespace_create pre bytes 40 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735fc3a9 0 {}} {proc_create pre format 0 {}} {proc_create pre bytes 0 {}} {before_rename pre format 0 ::__tcl_lsp_2286_r2286p_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {before_rename pre bytes 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes pre 0 {}} {after_rename pre format 0 ::__tcl_lsp_2286_r2286p_command_cross_⟦BYTE:E9⟧::identity 0 FORMAT_COMMAND} {after_rename pre bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286p_command_cross_é::identity"}} {after_rename pre renamed 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity_renamed 0 BYTES_COMMAND} {namespace_create decomposed format 40 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735f6501 0 {}} {namespace_create decomposed bytes 41 3a3a5f5f74636c5f6c73705f323238365f7232323836705f636f6d6d616e645f63726f73735f65cc81 0 {}} {proc_create decomposed format 0 {}} {proc_create decomposed bytes 0 {}} {before_rename decomposed format 0 ::__tcl_lsp_2286_r2286p_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {before_rename decomposed bytes 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity 0 BYTES_COMMAND} {rename_bytes decomposed 0 {}} {after_rename decomposed format 0 ::__tcl_lsp_2286_r2286p_command_cross_e⟦U+0001⟧::identity 0 FORMAT_COMMAND} {after_rename decomposed bytes 0 {} 1 {invalid command name "::__tcl_lsp_2286_r2286p_command_cross_é::identity"}} {after_rename decomposed renamed 0 ::__tcl_lsp_2286_r2286p_command_cross_é::identity_renamed 0 BYTES_COMMAND}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
