---
name: jetbrains-plugin-compat
description: >
  Keep the JetBrains/IntelliJ plugin (editors/jetbrains) binary-compatible
  across IDE versions. Use when the JetBrains plugin fails Marketplace
  verification or the local IntelliJ Plugin Verifier, when a user reports a
  NoSuchMethodError / NoSuchFieldError / "unresolved method …$default" from the
  plugin, when a new IntelliJ major (e.g. 2026.1+) breaks the plugin, when the
  LspServer/LspServerManager/LspServerDescriptor (or any platform) API is
  deprecated or moved, or before/after publishing a plugin build. Covers
  running the verifier, reading Marketplace verdicts via the REST API,
  root-causing moved-API / Kotlin `$default` breakage by disassembling SDK
  module jars, and the fix patterns that survive every supported IDE.
allowed-tools: Bash, Read, Edit, Grep, Glob, WebFetch
---

# JetBrains plugin compatibility

The plugin under `editors/jetbrains` is compiled **once** against the
`sinceBuild` floor (IntelliJ IDEA Ultimate 2024.1) and must load in **every**
IDE from that floor to the newest release, with an open `untilBuild`. Nothing
recompiles per IDE — the *same* bytecode is linked against whatever platform
the user is running. So a plugin breaks when the platform API it was linked
against changes shape underneath it: a method is removed, renamed, retyped, or
**moved to a super-interface**, or an API is deprecated then deleted.

There are two verification surfaces. Keep both green:

1. **Pre-publish, local** — the IntelliJ Plugin Verifier via Gradle, over the
   IDE list in `build.gradle.kts` (`pluginVerification.ides`).
   `make verify-editor-jetbrains`.
2. **Post-publish, Marketplace** — JetBrains re-runs the verifier against a
   wide matrix of released IDEs after every upload and shows the verdicts on
   `…/edit/versions/{stable,eap}`. Read them over the REST API with
   `marketplace_verify.sh`. This is the only surface that covers IDE builds
   **newer than anything in our local `ides` list** — which is exactly how the
   `LspServer.sendRequestSync$default` breakage first surfaced (a 2.1.x build
   flagged CRITICAL on 2026.1/2026.2 that no local target covered).

Helper scripts live next to this file:

- `marketplace_verify.sh` — read/schedule the Marketplace verdicts (needs the
  maintainer token; resolved via `scripts/release/jetbrains_token.sh`).
- `inspect_sdk.sh` — disassemble any platform API class at any IDE version
  **without** downloading a multi-GB IDE (fetches the small per-module jar from
  the JetBrains intellij-repository and runs `javap`).

## Verdict grammar

The verifier assigns each (plugin, IDE) pair one verdict:

- `OK` / `WARNINGS` — **compatible**. Warnings are informational only:
  *deprecated API*, *experimental API*, *internal API* usages. These do **not**
  fail the build (they are not in the verifier's default `failureLevel`) and
  must **not** be "fixed" by ripping out API the plugin still needs on older
  IDEs. Deprecation is a heads-up to plan migration, not an error.
- `CRITICAL` / `PROBLEMS` / `INVALID_PLUGIN` — **hard failures**. These are
  binary incompatibilities that throw at runtime (`NoSuchMethodError`,
  `NoSuchFieldError`, `NoClassDefFoundError`, `IllegalAccessError`). This is
  what you must drive to zero.

A jump in *deprecated* count across a major (e.g. 7 → 38 at 2026.1, when the
whole `LspServer*` family was superseded by `LspClient*`) is expected and
harmless. Only the CRITICAL section matters for shipping.

## Workflow A — read what the Marketplace found (fastest triage)

Reading verification results needs the maintainer token (the same
`JETBRAINS_TOKEN` used to publish; it carries `READ_VERIFICATION_RESULTS`).
The helper resolves it and **never prints it**:

```bash
# Both channels' latest builds, exits non-zero if any CRITICAL:
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh

# One channel, with the exact problem messages from each failing build:
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh results --channel eap --full
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh results --channel stable --version 1.11.4 --full

# After uploading a build, ask the Marketplace to verify a specific IDE build
# (e.g. the newest EAP that isn't in our local ides list yet):
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh products --channel eap   # list IDE builds
.claude/skills/jetbrains-plugin-compat/marketplace_verify.sh schedule --channel eap --ide IU-262.8665.176
```

`--full` prints the offending call sites straight from the report, e.g.:

```
CRITICAL  IntelliJ IDEA  IU-262.8665.176   2 compatibility problems
  ! Invocation of unresolved method com.intellij.platform.lsp.api.LspServer.sendRequestSync$default(...)
  ! Method com.tcllsp.jetbrains.CompilerExplorerPanel.runCompile$lambda$0(...) contains an *invokestatic* …
  ! Method com.tcllsp.jetbrains.actions.TclLspActionBase.runCommand(...) contains an *invokestatic* …
```

Under the hood (all on `https://plugins.jetbrains.com`, `Authorization: Bearer <token>`):

| purpose | endpoint |
|---|---|
| plugin id ↔ xmlId | `GET /api/plugins/31801` (xmlId `com.tcllsp.jetbrains`) |
| updates in a channel | `GET /api/plugins/31801/updates?channel={stable\|eap}&size=50` (public) |
| **verdicts for a build** | `GET /api/verifications/update/{updateId}?verificationType=INTELLIJ_COMPATIBILITY` (auth) |
| full report for one verdict | `GET {result.fullVerificationResultUrl}` (auth) |
| IDE builds available to verify | `GET /api/verifications/update/{updateId}/products` (auth) |
| schedule a verification | `POST /api/verifications/update/{updateId}?ideVersion=…&verificationType=INTELLIJ_COMPATIBILITY` (auth) |

`verificationType` ∈ `INTELLIJ_COMPATIBILITY` (what we want), `RESHARPER_COMPATIBILITY`, `IDE_PERFORMANCE`, `SECURITY_TOOLING`.
`resultType` (verdict) ∈ `OK`, `WARNINGS`, `PROBLEMS`, `CRITICAL`, `INVALID_PLUGIN`, `NON_DOWNLOADABLE`, `UNABLE_TO_VERIFY`.

## Workflow B — the local pre-publish gate

```bash
make verify-editor-jetbrains          # -> ./gradlew verifyPlugin
```

Targets live in `editors/jetbrains/build.gradle.kts` under
`pluginVerification.ides`. Keep the `sinceBuild` floor **and** the newest
verified stable major in that list. **A >=2026.1 target is load-bearing**:
2026.1 is where the `LspServer*` API was superseded and `sendRequestSync` moved
to the `LspClient` super-interface — without it the local verifier cannot catch
the whole `$default` / moved-API class of failure. First run downloads each
IDE (multi-GB, cached under `~/.gradle`); the verifier itself is static
bytecode analysis, it does not launch the IDE.

## Workflow C — root-cause a hard failure

Pattern to recognise: *"Invocation of unresolved method `X.foo$default(…)`"* or
*"unresolved method/field `X.bar`"*, with a hint *"might have been declared in
the super interface: Y"*. That means the symbol our bytecode names literally
(`X.foo$default`, `X.bar`) no longer exists **at `X`** in the newer platform —
usually because it moved, was renamed, retyped, or removed.

Prove exactly what changed by disassembling the class at the floor build and at
the failing build — no IDE download needed:

```bash
S=.claude/skills/jetbrains-plugin-compat/inspect_sdk.sh
$S 241.14494.240 com.intellij.platform.lsp.api.LspServer          # what we compiled against
$S 262.8665.176  com.intellij.platform.lsp.api.LspServer   -c     # the failing build (-c = show bytecode)
$S 262.8665.176  com.intellij.platform.lsp.api.LspClient          # the suspected new home
$S --list                                                          # list available `lsp` module versions
$S 262.8665.176  some.other.Class  platform-impl                  # a different platform module
```

For the `sendRequestSync` case this shows, unambiguously:

- **241** — `LspServer` declares `sendRequestSync(int, Function1)` **and** the
  synthetic `sendRequestSync$default(LspServer, int, Function1, int, Object)`.
- **262** — `LspServer extends LspClient` and declares **neither**; both moved
  to `LspClient`. `LspServer.sendRequestSync(int, Function1)` still *resolves*
  (an interface method is inherited from a super-interface), but the **static**
  `LspServer.sendRequestSync$default` bridge does **not** (a static call must
  resolve at the exact owner named in the bytecode).

## The Kotlin `$default` trap (and the fix)

When you call a Kotlin function that has a **default parameter** and you omit
that parameter, the compiler does **not** emit a call to the real method. It
emits `invokestatic Owner.method$default(...)` — a synthetic static bridge
bound to *the class that declared the method when you compiled*. If that method
later moves to a super-interface (or the owner otherwise changes), the
`$default` bridge is gone from the old owner and you get a `NoSuchMethodError`
even though the real method is still perfectly callable.

**Fix: pass every defaulted argument explicitly.** That makes the compiler emit
a direct `invokeinterface Owner.method(...)` (or `invokevirtual`), which
resolves through super-interfaces and survives the move. Prefer supplying the
platform's own compile-time-`const` default so behaviour is byte-identical and
the constant *inlines* (no runtime reference to it either):

```kotlin
// BEFORE — emits invokestatic LspServer.sendRequestSync$default(...)  [breaks on 2026.1+]
val result = server.sendRequestSync { lsp4j -> lsp4j.workspaceService.executeCommand(params) }

// AFTER — emits invokeinterface LspServer.sendRequestSync(int, Function1)  [resolves everywhere];
// DEFAULT_REQUEST_TIMEOUT_MS is `const` (10_000 ms) so it inlines to a literal.
val result = server.sendRequestSync(LspServer.DEFAULT_REQUEST_TIMEOUT_MS) { lsp4j ->
    lsp4j.workspaceService.executeCommand(params)
}
```

Both call sites are `TclLspActionBase.runCommand` and
`CompilerExplorerToolWindowFactory.runCompile`. Each carries a comment pointing
back here — **do not "simplify" them back to the no-timeout form**, that
reintroduces the `$default` bridge.

## Fix catalogue for other breakage

- **Moved to a super-interface (methods, `const` fields)** — usually resolves
  with no change (interface/const resolution walks super-interfaces). Only the
  Kotlin `$default` static bridge is a problem → pass args explicitly (above).
- **Renamed / removed / retyped API** — migrate to the replacement. If you must
  keep working on *both* old and new IDEs from one binary and the signatures
  are incompatible, dispatch reflectively (resolve by name at runtime) or gate
  on `ApplicationInfo.build`. Do this only when a source-level call can't
  satisfy both; it's a last resort.
- **Deprecated API (WARNINGS only)** — do **not** remove it if the plugin still
  supports IDEs where the replacement doesn't exist. Track the eventual removal
  and migrate when the `sinceBuild` floor rises past the replacement's
  introduction.
- **Consuming newer-only API by accident** — the plugin compiles against the
  floor SDK precisely so this fails at compile time. Never bump the compile SDK
  to reach a newer symbol; that silently raises the effective floor.

## Prove the fix at the bytecode level (gold standard)

Because this is entirely about *emitted bytecode*, prove it rather than assume.
Compile a one-method repro of the before/after against the floor SDK and
disassemble — the fix is correct iff the `$default` `invokestatic` becomes a
plain `invokeinterface`/`invokevirtual`:

```bash
# jars: SDK module (inspect_sdk.sh caches them under tmp/idea-sdk-jars/), lsp4j
# from Maven Central, and kotlin-stdlib. A Kotlin compiler ships inside Gradle:
#   /opt/gradle-*/lib/kotlin-compiler-embeddable-*.jar  (+ kotlin-stdlib/reflect/
#   script-runtime and kotlinx-coroutines-core-jvm in the same lib dir)
# Point -kotlin-home at a dir whose lib/ has kotlin-stdlib.jar / kotlin-reflect.jar
# / kotlin-script-runtime.jar, then:
kotlinc -cp "lsp-<ver>.jar:lsp4j.jar:kotlin-stdlib.jar" Repro.kt -d out
javap -p -c -classpath out Repro | grep -iE 'invokestatic|invokeinterface|sendRequestSync'
# BEFORE: invokestatic  LspServer.sendRequestSync$default(...)
# AFTER:  sipush 10000  +  invokeinterface LspServer.sendRequestSync(I,Function1)
```

Then confirm the *other* direction with `inspect_sdk.sh <failing-build>`: the
plain method resolves at the new owner (directly or inherited), and the old
`$default` symbol is genuinely absent.

## Before you ship — checklist

- [ ] `make verify-editor-jetbrains` is green (0 CRITICAL) across all
      `pluginVerification.ides`, including a >=2026.1 target.
- [ ] Any new IntelliJ-platform call that takes a defaulted Kotlin parameter is
      called with that parameter **explicit** (no `$default` in the bytecode).
- [ ] The compile SDK is still the `sinceBuild` floor (`intellijIdeaUltimate`
      in `dependencies`), not bumped to reach a newer symbol.
- [ ] After publishing, `marketplace_verify.sh` shows no CRITICAL on the new
      build, including IDE majors newer than the local `ides` list. Schedule a
      run against the newest EAP if the matrix hasn't covered it yet.

## Key files

- `editors/jetbrains/build.gradle.kts` — compile SDK (`dependencies { … }`),
  `sinceBuild`/`untilBuild`, and `pluginVerification.ides`.
- `editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/actions/TclLspActionBase.kt`
- `editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/CompilerExplorerToolWindowFactory.kt`
- `Makefile` — `verify-editor-jetbrains`, `build-editor-jetbrains`,
  `publish-jetbrains`.
- `scripts/release/jetbrains_token.sh` — token resolution (env / Keychain /
  libsecret). The token is a secret: never echo it, never commit it.
