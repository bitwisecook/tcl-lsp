# BIG-IP cross-context naming: unicode bytes reverse

## Question

Do precomposed and decomposed variable names produced from binary bytes remain distinct when written in the reverse order? This affects Unicode name resolution in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-20](fixtures/bigip-tmm-traffic-event-20.md)

- run: `r2286p_unicodebytes_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{lengths 37 38} {raw_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735fc3a9 utf8_conversion 1 {invalid command name "encoding"}} {raw_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735f65cc81 utf8_conversion 1 {invalid command name "encoding"}} {write_decomposed 0 DECOMPOSED} {write_pre 0 PRECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_decomposed 0 {}} {remaining_pre 0 PRECOMPOSED}⟦EOF⟧
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
{lengths 37 38} {raw_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735fc3a9 utf8_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735fc383c2a9} {raw_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735f65cc81 utf8_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735f65c38cc281} {write_decomposed 0 DECOMPOSED} {write_pre 0 PRECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_decomposed 0 {}} {remaining_pre 0 PRECOMPOSED}⟦EOF⟧
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
{lengths 37 38} {raw_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735fc3a9 utf8_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735fc383c2a9} {raw_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735f65cc81 utf8_conversion 0 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646562797465735f65c38cc281} {write_decomposed 0 DECOMPOSED} {write_pre 0 PRECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_decomposed 0 {}} {remaining_pre 0 PRECOMPOSED}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
