# BIG-IP cross-context naming: lexical format unbraced

## Question

How does Tcl parse an unbraced variable token containing name characters produced by format %c? This affects lexical name recognition in the LSP and compiler.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-16](fixtures/bigip-tmm-traffic-event-16.md)

- run: `r2286p_lex_format_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{pre write 0 PRE_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745fe9 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745f223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745f6501 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745f65223a206e6f2073756368207661726961626c65}⟦EOF⟧
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
{pre write 0 PRE_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745fe9 eval 1 63616e2774207265616420225052455f56414c5545223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745f6501 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745f65223a206e6f2073756368207661726961626c65}⟦EOF⟧
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
{pre write 0 PRE_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745fe9 eval 1 63616e2774207265616420225052455f56414c5545223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745f6501 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f7232323836705f6c65785f666f726d61745f65223a206e6f2073756368207661726961626c65}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
