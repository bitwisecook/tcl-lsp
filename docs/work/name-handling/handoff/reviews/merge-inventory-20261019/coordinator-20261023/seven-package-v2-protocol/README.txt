Seven-package all-target typecheck protocol archive

The lossless archive is evidence.tar.gz; its SHA-256 is recorded in evidence.tar.gz.sha256. It contains the frozen v2 suite, launch helper, exact plan, runner-input and evidence manifests, source18 manifest, no-GO refusal, readiness record, VM static peer review, and v1 supersession provenance plus the v1 files still available at capture. It contains no .rs or .c files. archive-input-manifest.json inside the tar records SHA-256 for each archived input file except the archive manifest itself.

V2 review pins: plan 427f4587e8138e2994a97df5f68ac8b00794bdc30851b83bbf1e300ed7e842d7; suite 7c70a7678ea19abf03555e5bac7b6980032a9a119e961248ea1c60dce0475bd1; launcher 21393272f47d687cbd8e935ed32c584e9d60c6c549d380fa2fac512bc3602c27; runner manifest 7b2f2ffa7eb203ae5992bb7e3042059ac7f364414232cfe6b5e4dd5e0aeb72c4; executor 88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811. VM bounded static review is CLEAR; no Cargo/runner/provider/lock was run by the reviewer. The only invocation was launch.py's absent-GO refusal, which started neither runner nor Cargo.

This is preparation evidence only. Launch remains blocked until root integrates exact18 and produces matching root GO. It provides no typecheck result, test result, runtime evidence, or release acceptance.
