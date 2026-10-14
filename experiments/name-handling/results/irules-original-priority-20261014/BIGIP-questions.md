# Original iRules event and priority observations

Status: unanswered. The source reporter uses Registry-authored conditional metadata;
it does not establish BIG-IP loader acceptance, handler registration, firing or order.
These cases require a BIG-IP oracle. C Tcl and JimTcl lack the hosted handlers.

Run each case as an independent temporary rule. Record the complete original source,
TMOS/TMM version, loader acceptance or complete error, any observed registered priority,
virtual-server profiles, event stimulus and actual firing order. Restore the appliance
configuration after each case. Return raw outputs and answer documents on `work`.
Do not substitute a mock `when` or `priority` command. A rejected rule establishes only
its observed loader boundary; it does not establish a runtime lookup result.

| Case | Original source | Question |
| --- | --- | --- |
| default | `when HTTP_REQUEST { log local0. default }` | Which priority is registered when omitted? |
| inherited | `priority 200; when HTTP_REQUEST { log local0. inherited }` | Is top-level priority accepted and inherited? |
| inline | `priority 200; when HTTP_REQUEST priority 300 { log local0. inline }; when HTTP_RESPONSE { log local0. after }` | Does the inline priority leave later inheritance at 200? |
| dynamic setter | `set p 200; priority $p; when HTTP_REQUEST { log local0. dynamic }` | Which loader/value boundary is observed? |
| expanded setter | `set p {200}; priority {*}$p; when HTTP_REQUEST { log local0. expanded }` | Is expansion supported at this source position? |
| quoted computed head | `set tail en; "wh${tail}" HTTP_REQUEST { log local0. computed }` | Does the loader accept and select this head? |
| replaced handler | `proc when args {}; when HTTP_REQUEST { log local0. replacement }` | Is replacement accepted and which declaration is registered? |
| replaced setter | `proc priority args {}; priority 200; when HTTP_REQUEST { log local0. replacement }` | Does this create an event priority or only a procedure call? |
| root head | `::when HTTP_REQUEST { log local0. rooted }` | Does the loader treat the explicit root head as an event declaration? |
| inert body | `set stored {priority 10; when HTTP_RESPONSE { log local0. inert }}; when HTTP_REQUEST { log local0. live }` | Is stored body text excluded from registration and inheritance? |
| local shadow | `proc helper {} {proc priority args {}; priority 10}; when HTTP_REQUEST { log local0. live }` | Does an uncalled body leave top-level priority unchanged? |
| expanded inline | `set p {300}; priority 200; when HTTP_REQUEST priority {*}$p { log local0. expanded }; when HTTP_RESPONSE { log local0. after }` | What is accepted, and does later inheritance remain 200? |

Actual observations can support version-scoped conformance tests after answers arrive.
The Rust controls accompanying the source migration exercise source correspondence,
selection refusal and report boundaries; they are not substitute oracle results.
