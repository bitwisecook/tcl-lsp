# BIG-IP APL publication questions

Goal: observe how actual APL presentation fields become Tcl variable operands in the iApp implementation context. The code currently has one explicit dotted-name/double-underscore convention and shared source grammar; it does not claim appliance variable publication, live cell or frame identity from that convention.

Use an isolated iApp fixture in the appliance's actual supported template workflow. Retain the exact APL, implementation, supplied values, BIG-IP release/hotfix, actual Tcl patchlevel, interpreter/context and fixture identity. Keep original commands, return status, stdout/stderr, and source files. Save answer documents and raw inputs/results under `experiments/name-handling/apl-source-variable-geometry/f5-answers/` on `work`. For an unavailable case, record the actual declaration or execution refusal, including whether the lookup was reached; do not substitute a desktop interpreter result.

| Case | APL declaration | Questions for the actual implementation |
| --- | --- | --- |
| Simple | `section basic { string addr }` | With value ROOT, what exact original variable operand reads it? Compare `$::basic__addr`, `$:::basic__addr`, `${:basic__addr}` and `$basic__addr` in their actual frame. Record status separately for every form. |
| Nested table | `section ssl { table members { string port } }` | What variables and scalar/array roots are published? Observe `$::ssl__members__port`, its braced spelling and documented table-index forms rather than assuming a flat scalar. |
| Existing underscores | Fields `member_addr`, `member__addr`, and any permitted nested name that generates the same convention label | Does the template loader reject a collision, publish alternatives, or replace one? Keep every declaration and supplied value, even if reports collide. |
| Punctuation | Accepted or rejected identifiers containing literal dot, lone colon, colon run, hyphen and non-ASCII text | Which declarations are accepted? For accepted fields, retain exact source operands that read the supplied distinct value. A diagnostic report is not an original input operand. |
| Repeated declaration | Two accepted fields/declarations with the same domain coordinate | Does loader order matter? Observe actual export/redefinition status and values without inferring an identity from the report. |
| Namespace context | The simple field read from original root implementation and an independently entered namespace/procedure if supported | Is the export a root namespace variable, frame-local binding, loader alias or another object? Record original declarations and successful context-specific reads independently. |
| Source roles | Comments, `list {$::basic__addr}`, `list \$::basic__addr`, `list "$::basic__addr"`, `expr {$::basic__addr eq "ROOT"}`, and a documented literal procedure/control body | Which sources actually read the field? Preserve complete source/configuration and actual body entry. Literal data does not establish a substitution read. |

For the simple/source-role cases, `source-variable-geometry.tcl` provides the exact desktop source forms as a comparison script. Use only the forms appropriate to the selected appliance context and preserve the complete actual source. Appliance results answer iApp publication/entry questions independently of the C/Jim source observations.

If the appliance context exposes tcltest, use its original interpreter-version `tcltest.tcl`/`init.tcl`, retain their source provenance and versions, and record package availability/refusal. Direct bounded Tcl scripts are sufficient for these questions and do not require a substituted test package.

Answer each row with the original producer, original lookup input, actual context, reached/refused status, observed value/error and scope of the conclusion. Follow successful observations with the matching negative control: absent field, inert literal, wrong namespace/context, colliding declaration or invalid grammar as appropriate. Implementation tests must cite these actual results; unsupported rows stay explicit.
