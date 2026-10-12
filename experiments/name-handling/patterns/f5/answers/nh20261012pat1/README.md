# Corpus and Wiki measurements: nh20261012pat1

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 285 separate
records: 145 observed completions, 82 unreached operations and 58 unavailable
body/callback results. Context, unit, wrapper, original source and error-field
availability are explicit in each record.

Hosted corpus cases rename owned namespaces/procedures and split setup from
the final command. They are adaptations. RULE_INIT and rule-procedure contexts
each reached 11 cases on all four recorded units. CLI, iApp implementation
and iCall each reached nine cases; their other cases failed setup.

Stock corpus records capture the whole original program. An earlier error
does not establish final-command reachability; those whole-program completions
are separately scoped in the records. Stock Wiki output reports 21 PASS/9 SKIP
on Tcl 8.4.13 and 22 PASS/8 SKIP on Tcl 8.5.13. No selected-body return markers
were captured, so body-result bytes/error options are unavailable despite PASS.

The traffic wrapper was rejected because its string exceeded the MCP message's
65,535-character limit. It supplies no traffic lookup or semantic unsupported
answer. APL callback execution remains unreached.

[reviewed-proofs.json](reviewed-proofs.json) proposes narrow central records,
retaining original provider observations separately. `MANIFEST.sha256` seals
the retained files; its digest list does not certify unmeasured execution.
