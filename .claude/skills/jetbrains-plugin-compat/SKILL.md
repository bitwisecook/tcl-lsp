---
name: jetbrains-plugin-compat
description: >
  Keep the JetBrains plugin binary-compatible across supported IDE versions.
  Use for Marketplace or local verifier failures, platform API changes,
  NoSuchMethodError / NoSuchFieldError reports, or plugin release verification.
allowed-tools: Bash, Read, Edit, Grep, Glob, WebFetch
---

# JetBrains plugin compatibility

The plugin under `editors/jetbrains` compiles once against IntelliJ IDEA
Ultimate 2025.3 (`sinceBuild = 253`) and uses an open `untilBuild`. The same
bytecode must link against every supported IDE's platform and LSP4J libraries.
Keep the compile SDK at the support floor.

## Verification surfaces

- **Local:** `make test-jetbrains` runs the Kotlin suite.
  `make verify-editor-jetbrains` runs the IntelliJ Plugin Verifier against
  `pluginVerification.ides` in `editors/jetbrains/build.gradle.kts`.
- **Marketplace:** JetBrains verifies each uploaded plugin against its IDE
  matrix. `.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh`
  reads the results using the maintainer token resolved by
  `scripts/release/jetbrains_token.sh`. Never print the token.

Keep the support floor, a stable IDEA target, a 263 EAP target, and CLion's
bundled Web Browser layout in the local matrix. A >=2026.1 target covers the
`LspClient` API; a 263 target covers LSP4J 1.0 diagnostic messages. IDE downloads
are multi-GB and cached by Gradle.

## Verdicts

`OK` and `WARNINGS` are compatible. Deprecated, experimental and internal API
usages are warnings. Keep APIs needed by the support floor when their
replacements are unavailable there.

`CRITICAL`, `PROBLEMS` and `INVALID_PLUGIN` are failures. Resolve the reported
binary incompatibilities before shipping. A missing method, field or class
can throw at runtime even when compilation against the floor succeeds.
`NON_DOWNLOADABLE` and `UNABLE_TO_VERIFY` do not establish compatibility.

## Read Marketplace results

```bash
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh results --channel stable --version 2.2.5 --full
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh results --channel eap --full
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh products --channel stable
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh schedule --channel stable --ide IU-263.7031.23
```

The helper's `--full` output identifies the exact unresolved symbol and plugin
call sites. Scheduling verification changes the Marketplace verification queue;
use it when verification of an uploaded build is requested.

The Marketplace REST API uses plugin ID `31801`, XML ID
`com.tcllsp.jetbrains`, and `verificationType=INTELLIJ_COMPATIBILITY`:

| Purpose | Endpoint on `https://plugins.jetbrains.com` |
|---|---|
| Updates in a channel | `GET /api/plugins/31801/updates?channel={stable\|eap}&size=50` |
| Verdicts | `GET /api/verifications/update/{updateId}?verificationType=INTELLIJ_COMPATIBILITY` |
| Detailed report | `GET {result.fullVerificationResultUrl}` |
| Available IDE builds | `GET /api/verifications/update/{updateId}/products` |
| Schedule verification | `POST /api/verifications/update/{updateId}?ideVersion=…&verificationType=INTELLIJ_COMPATIBILITY` |

Verification endpoints require the maintainer token, including
`READ_VERIFICATION_RESULTS` for reading reports.

## Inspect the platform API

`inspect_sdk.sh` downloads individual module jars and runs `javap`, avoiding
an entire IDE download:

```bash
.claude/skills/jetbrains-plugin-compat/inspect_sdk.sh --list lsp
.claude/skills/jetbrains-plugin-compat/inspect_sdk.sh 253.28294.334 com.intellij.platform.lsp.api.LspServer
.claude/skills/jetbrains-plugin-compat/inspect_sdk.sh 263.7031.23 com.intellij.platform.lsp.api.LspClient -c
```

Its optional third argument selects a module other than `lsp`; `-c` includes
method bytecode. It tries both release and snapshot repositories and caches
jars under `tmp/idea-sdk-jars/` or `SDK_CACHE`.

Compare the floor and failing build's exact JVM signatures, including return
types and declaring owners. Inspect the compiled plugin class with
`javap -p -c` to confirm its references resolve in both SDKs.

## Compatibility rules

- **Request timeouts:** pass `LspServer.DEFAULT_REQUEST_TIMEOUT_MS` explicitly
  to every `sendRequestSync` call. It inlines as `10_000`. Omitting it emits
  a static `LspServer.sendRequestSync$default` call, whose owner is not stable
  across supported IDEs. A direct interface method call resolves through the
  platform's interface hierarchy.
- **Diagnostic messages:** use `super.getMessage(diagnostic)` inside
  `LspDiagnosticsSupport` message and tooltip customisation. Its `String`
  return type is stable. LSP4J 1.0 uses
  `Either<String, MarkupContent>` for `Diagnostic.getMessage()`, while the
  floor uses `String`; direct calls cannot link across both versions.
- **Interface defaults:** keep Kotlin's
  `jvmDefault = JvmDefaultMode.NO_COMPATIBILITY`. Supported IDEs provide JVM
  default methods. Inheriting them directly avoids generated forwarding calls
  to deprecated or experimental tool-window and status-bar defaults.
  `TclPlatformCompatibilityTest` checks the relevant inherited methods.
- **Logging:** the server descriptor uses `DESCRIPTOR_LOG`, its own logger.
  A local logger named `LOG` is shadowed by the inherited deprecated
  `LspServerDescriptor.LOG` field inside descriptor methods.
- **LSP lifecycle:** retain the `LspServer*` descriptor, manager and provider
  APIs while supporting 2025.3. Their `LspClient*` replacements are absent
  from that SDK.
- **Opening editors:** retain floor-compatible editor-opening calls. Verify
  exact signatures before using asynchronous alternatives; identical method
  names do not guarantee compatible return types.
- **Other renamed, removed or retyped APIs:** prefer a stable platform API
  that handles the installed SDK. Use reflection or an IDE-version adapter
  only when no direct call satisfies both the floor and newer IDEs.

## Release checks

1. Run `make test-jetbrains` and `make verify-editor-jetbrains`.
2. Confirm zero binary compatibility failures across the configured matrix.
3. Confirm the compile SDK and `sinceBuild` still match the support floor.
4. After an authorised upload, check Marketplace results for that exact new
   version. Verify a newer EAP when the Marketplace matrix does not cover it.

Publishing credentials remain behind the protected release environment; this
skill does not authorise a plugin upload or release by itself.
