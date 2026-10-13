# ParseFormatArgs active-call pool repair

The Runtime ParseFormatArgs adapter now retains its shared literal pool for the active call. Numeric getter callbacks can delete all three implemented worker registrations while success/default-list or conflict-result publication still receives the same live pool literals. Registration ownership and deletion timing stay unchanged.

The [clean Runtime rebuild and five focused controls](validation-02/validation.json) passed. Both new conflict/success ABI callback tests, the existing GetDateFields active-call deletion test and both prior ParseFormatArgs pool-retirement tests each ran once from the same fresh binary. Post-call result references match their actual receiving roles. The [14-package source envelope](validation-02/scope.json) stayed unchanged across the build and controls.

The [first build attempt](validation-01/validation.json) failed with ENOSPC and executed zero controls. The root's selected-package cleanup and observed disk capacity are recorded in [rebuild-basis.json](validation-02/rebuild-basis.json), including the explicitly labelled transcription of its clean command output.

- [Question and control scope](question.md)
- [Source readiness](source-ready.json)
- [Exact source patch](source/repair.patch)
- [Original question seal verification](question-seal-integrity-after-validation02.json)

These are software Runtime lifetime controls. No new C oracle run, C-native callback chronology, seven-worker pool lifetime, C9/Jim/F5 behavior or private-layout conformance is claimed.
