# Sol-36: pack shadowing a dialect built-in (PARKED)

Status: parked by the coordinator on 2026-10-19 (see user-decisions-20261019T1230Z.json). Everything below is static analysis at HEAD `82617ee15`. I ran no cargo, made no change to the repository, and wrote **no patch**. The work stopped before implementation began.

New direction from the coordinator: Jim becomes a separate core dialect, and tcl-registry builds Jim's core store (Sol-34 is redrafting that). The proposed rule below is therefore restated against "the dialect's own core store" rather than against the current family-surface layering.

## 1. The existing rule for a pack vs the C Tcl core

**Policy: shipped wins unless the pack says `-override`.** Both outcomes are reported, at Information severity.

- **Policy statement**
  - The module docs state the policy at `rust/tcl-spectcl/src/install.rs:33-40`.
  - The same policy is stated again at `rust/tcl-spectcl/src/pack.rs:1236-1242`.
- **Where the decision is made**
  - `pack::installs_over(command, registry)` (`pack.rs:1279-1281`) returns `command.overrides_shipped || registry.get(name).is_none()`.
  - It is the single shared predicate. Its doc comment says it exists so that "the report and the behaviour cannot drift".
- **Install**
  - `install::install_into` (`install.rs:128-193`) runs inside the overlay builder `tcl_registry::registry_for_profile_with_overlay` (`install.rs:86` → `tcl-registry/src/cache.rs:244-273`).
  - The store it installs into is `build_default` plus `profile.base_layers`, which is the C Tcl core, stdlib and tcllib.
  - A command is inserted only when both the vendor gate and `installs_over` pass (`install.rs:159-161`). A non-`-override` name the store already holds is **not installed**.
- **Security floor (invariant I6)**
  - When an admitted command replaces an existing row, the installed spec is `SecurityFloor::of(registry.get(name)).apply(copy)` (`install.rs:163-176`; `tcl-registry/src/security_floor.rs`, `of`/`apply`).
  - Security traits and set-valued taint facts are unioned.
  - Single-valued sink, colour and hook fields take the shipped value.
  - The floored copy is leaked and inserted with `insert_static`. Its pack origin and reference text are keyed by the copy's address (`install.rs:177-187`; `registry.rs:2358`, `:2367`).
- **Notice**
  - `pack::collision_notices(packs, registry)` (`pack.rs:1243-1274`) emits one `PackNotice` per pack command whose name `registry.get` already answers. Each notice has:
    - Severity `Information`
    - context `command <name>`
    - line 1 of the pack's first file
  - Notice text without `-override`: "`X` is already a shipped command; the shipped spec wins (declare it `-override` to replace it)".
  - Notice text with `-override`: "`X` replaces the shipped command of that name (`-override`)".
- **Where the LSP computes the notices**
  - `report_pack_reload` (`tcl-lsp-server/src/lib.rs:27357`, call at about `:27378-27386`) checks against `tcl_lsp_core::registry_for_dialect(session_dialect)` (`tcl-lsp-core/src/lib.rs:292`). That is the session dialect's **un-overlaid** static store.
  - The comment there gives the reason for using the session dialect rather than each folder's: packs install at workspace scope.
- **How the LSP publishes the notices**
  - `publish_spec_pack_notices` (`lib.rs:~27952-27963`) publishes them with code `SPEC_PACK_DIAGNOSTIC_CODE` (shared by every pack notice) and source `tcl-lsp`.
- **Pack vs pack** is a separate rule (`pack.rs:857-981`, `cross_pack_command_notices`). It is not affected here.

## 2. What happens today for a family built-in (Jim)

- **Where Jim's own commands live**
  - They are not in the store that `install_into` sees.
  - They are layered afterwards by `cache::registry_with_core_surface` (`cache.rs:399-433`), which calls `CommandRegistry::with_core_surface` (`registry.rs:2320-2347`).
  - That builds `unoverlaid.by_name`, then the family specs, then **every** `overlaid.overlay_specs`, with no collision check.
- **The two paths that layer pack rows over Jim's rows**
  - The LSP generation: `command_store` in `tcl-registry/src/model/assembly.rs:403-436`, over `(plain_tcl, key)`, then projected to jim.
  - The pack path: `install::registry_with_packs`, once Sol-34's `fix-only.patch` adds `store_with_core_surface`.
- **Result for a non-`-override` pack `defer`**
  1. `installs_over` sees no `defer` and installs it.
  2. The Jim layer appends Jim's `defer` and then the pack row.
  3. `get("defer")` returns the pack row.
  4. Jim's security floor is never applied.
  5. No notice is emitted unless the session dialect is `jim`.
- **The report and the behaviour already disagree for a `jim` session**
  - The session store `registry_for_dialect("jim")` carries the Jim surface, because overlay 0 goes through `command_store`, which layers it.
  - So `collision_notices` reports "the shipped spec wins", but the pack row is the one that wins. This is exactly the drift `installs_over` was written to prevent.
- **Consumers disagree with each other**
  - Raw `get` consumers (explorer, CLI, MCP, LSP handlers using `registry_for_dialect`) see the pack row.
  - The context's `resolve_command` (`assembly.rs:261-293`) ranks candidates by nearest core point.
    - On an un-projected store, a pack row with `surface: None` loses to Jim's `Core(Jim)` row.
    - On the LSP's projected store, the pack row wins.

## 3. Proposed rule (for decision)

**Same policy, applied where the dialect's own core rows join the store.** With the new direction, that is wherever tcl-registry builds Jim's core store under the overlay.

1. **Without `-override`**
   - A pack command whose name the dialect's own core store declares is **not installed** in that dialect's store.
   - It stays installed in stores without that row, such as plain Tcl documents.
2. **With `-override`**
   - The command is installed with `SecurityFloor::of(<the dialect's own row>)` applied.
   - Take the last-registered row of that name, which mirrors `registry.get` in `install_into`.
   - The floor is applied on top of any Tcl-core floor `install_into` already applied.
3. **What the store layer needs**
   - The store layer must know whether each overlay spec was declared `-override`.
   - Proposal: a per-registry installation fact next to `pack_origins`, for example `pack_overrides: FxHashSet<usize>`.
     - The installer sets it from `PackCommand::overrides_shipped`.
     - It is included in the snapshot key, which the exhaustive destructure at `registry.rs:~1903` forces a decision on.
   - I rejected adding the flag to `PackOrigin`, because that would change the VM fact-stamp identity and the `site_claims` fixtures.
   - Dropped overlay rows must also leave `overlay_specs`, because `project_for_profile` (`registry.rs:~2244-2252`) re-adds overlays from that list.
   - A floored copy needs its `pack_origins` and `reference_texts` re-keyed to the copy's address.
4. **If Jim becomes a real core dialect whose store `install_into` sees directly** (a catalogue-like profile with Jim rows in `build_default`/`base_layers`), points 1-3 need **no new code**.
   - `installs_over`, `SecurityFloor`, and `collision_notices` already apply.
   - That is the preferred outcome of Sol-34's redraft. The open item is only the notice scope (point 5).
5. **Notice scope: undefined today for this case. Flagged for your decision.**
   - The Tcl-core rule checks only the session dialect's store. That is safe for Tcl core commands, which every profile has. It misses a dialect-only built-in when the session dialect is something else, for example a tcl8.6 session with jim folders.
   - **(A) Strict same rule.** Keep checking against the session dialect only. Jim collisions are then reported only in `jim` sessions.
   - **(B) Recommended.** `collision_notices` also asks tcl-registry which core dialects' own stores declare the name. For example a new `pub fn core_surface_families(name) -> Vec<Family>` over the registered specs (`cache.rs:311-379`). It would then emit the same Information notice, naming the dialect:
     - Without `-override`: "`defer` is a built-in command of jim; jim documents keep the built-in spec (declare it `-override` to replace it)".
     - With `-override`: "`defer` replaces jim's built-in command of that name (`-override`)".
     - Use the generic text when a non-family shipped row also matches, so a command never gets two notices.
   - Either way the code stays `SPEC_PACK_DIAGNOSTIC_CODE` and the severity stays Information. The task title says "Warn"; keeping Information matches the existing rule, so raising it to Warning would be a policy change.

## 4. Tests planned (not written)

- **Context for these tests**
  - `rust/tcl-spectcl/tests/jim_own_surface.rs` is the natural home. Sol-34's v2 already adds `a_pack_carrying_jim_store_keeps_jims_own_surface` there.
  - `defer`, `local` and `getref` are Jim-only heads that no other surface declares.
- **Non-`-override` `defer` pack**
  - `get("defer")` is pointer-equal to Jim's row on both stores:
    - the LSP store, `resolve_environment("jim").context_registry(_, key)`
    - the pack store, `registry_with_packs(jim, set)`
  - The pack row is still present in the `tcl8.6` and plain stores.
  - `collision_notices` yields one Information notice with the dialect text.
- **`-override` `defer` pack** whose spec drops a trait or taint fact Jim's row has
  - The installed row is the pack's arity with Jim's security facts.
  - `pack_origin` is preserved.
  - The notice uses the override text.
- **Controls**
  - A non-colliding pack (`probe::hello`) and the `tcl8.6` profile return the same `Arc` as before.
  - An existing `install.rs` test, `shipped_wins_a_collision_unless_the_pack_says_override`, stays unchanged.

## 5. Cargo plan (when allowed)

```sh
cd /home/user/tcl-lsp && source scripts/dev/agent-build-env.sh
cargo test -p tcl-spectcl --test jim_own_surface 2>&1 | tee /tmp/sol36-jim.log
cargo test -p tcl-spectcl --lib install:: pack:: 2>&1 | tee /tmp/sol36-spectcl-lib.log
cargo test -p tcl-registry --lib 2>&1 | tee /tmp/sol36-registry.log
cargo test -p tcl-spectcl --test workspace_packs --test pack_torture 2>&1 | tee /tmp/sol36-packs.log
grep -E "^test .*(FAILED|panicked)|test result" /tmp/sol36-*.log
make rust-check 2>&1 | tee /tmp/rust-check-sol36.log
```

## Docs to update when implemented

- `docs/design/registry/spec-packs.md`: the collision policy section and § Jim's compiled-in packs.
- Any KCS note that lists pack notices. I had not searched for these before the work was parked.
