# Hosted F5 dictionary-body questions

Run these questions in the actual BIG-IP Tcl host available to the answering chat. Record the BIG-IP release, Tcl release if exposed, hosting surface, event/frame, relevant configuration, original script bytes and hash, exact output/errors, and whether each probe was actually reached. System tclsh on the appliance is a separate provider and cannot answer an iRule-hosted question.

1. Are `dict with`, `dict update`, empty variable names and literal-dollar names available in the hosted event? Run the retained empty-name, literal-dollar and comment-only probes in a disposable event/frame and report both dictionary contents and mapped variables.
2. Does an empty body still write mapped variables? If hosted `trace` is available, run the retained trace-alias probe independently and report callback count, output, errors and process/event completion. If trace is unavailable or prohibited, report that refusal; do not substitute a C Tcl answer.
3. Can a nonempty dictionary body define or change command names in this host? Run the retained body-command-mutation probe in an isolated disposable context and report actual availability and scope. Record any policy refusal before body entry.
4. What occurs for malformed dictionaries, missing dictionaries, array-element dictionary operands and unknown keysets? Report entry/body/writeback separately where observable. Does `dict update` use a scripted helper whose closure must remain intact, or an opaque native implementation? An unavailable helper identity remains unanswered.
5. Do mapped empty names, literal sigils, qualified names, array references and Unicode NUL name values follow this host's actual variable policy? Extend each retained script separately, preserving bytes and results. Do not infer Native byte-string semantics from Unicode values.

Store answers and raw scripts/results beside these questions in the same branch. Each answer must distinguish an executed observation, an unsupported/refused operation and an unanswered question. No hosted result is available in the current C/Jim captures.
