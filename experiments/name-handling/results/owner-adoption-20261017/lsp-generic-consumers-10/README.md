# Direct LSP capture on resumed source

The resumed session rebuilt the six library test targets from `2a3928bc6` (zero warnings) and ran all 1,085 planned direct-LSP selectors with the unchanged replay runner, 180-second test and 30-second listing budgets: **838 PASS, 247 FAIL including 3 timeouts**. Source (114,247 files), six providers and six test ELFs were identical before and after. The 790 selectors shared with the paused 35r2 run have identical verdicts.

[capture.json](capture.json) pins the archive and member index. The archive holds the build receipts, every raw test log and result, and the re-rooted helper scripts. No binaries or source trees. Not compiler, integration, server-process or editor-host acceptance.
