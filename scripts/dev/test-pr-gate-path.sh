#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract test for issue #2014. The Rust fast gate must run in an independent
# worker after `channel`; the required `pr-gate` context aggregates that result
# with the SpecTcl and front-end prerequisites and fails closed on any failure.
# The mutation checks below make the contract itself fail closed when a future
# edit removes an edge, adds a skip or ignored-failure escape hatch, weakens a
# tag condition, or puts the Rust work back behind a prerequisite.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WORKFLOW=${WORKFLOW:-$REPO_ROOT/.github/workflows/ci.yml}

fail() {
    echo "pr-gate path contract: $*" >&2
    exit 1
}

job_block() {
    awk -v wanted="$1" '
        $0 == "  " wanted ":" { found = 1; print; next }
        found && /^  [A-Za-z0-9_-]+:/ { exit }
        found { print }
    ' "$WORKFLOW"
}

require_job() {
    block=$(job_block "$1")
    [ -n "$block" ] || fail "ci.yml must define the $1 job"
    printf '%s\n' "$block"
}

needs_value() {
    printf '%s\n' "$1" | sed -n 's/^    needs: //p'
}

needs_sequence() {
    printf '%s\n' "$1" | awk '
        /^    needs: / {
            sub(/^    needs: /, "")
            print
            exit
        }
        /^    needs:$/ {
            if (getline > 0 && $0 ~ /^      \[/) {
                sub(/^      /, "")
                print
            }
            exit
        }
    '
}

require_contains() {
    block=$1
    text=$2
    message=$3
    printf '%s\n' "$block" | grep -Fq "$text" || fail "$message"
}

require_line() {
    block=$1
    text=$2
    message=$3
    printf '%s\n' "$block" | grep -Fqx "$text" || fail "$message"
}

step_block() {
    block=$1
    name=$2
    printf '%s\n' "$block" | awk -v wanted="$name" '
        $0 == "      - name: " wanted { found = 1; print; next }
        found && /^      - name: / { exit }
        found { print }
    '
}

run_block() {
    step=$1
    printf '%s\n' "$step" | awk '
        /^        run: \|$/ { found = 1; next }
        found && $0 !~ /^          / && $0 !~ /^[[:space:]]*$/ { exit }
        found {
            sub(/^          /, "")
            print
        }
    '
}

env_block() {
    step=$1
    printf '%s\n' "$step" | awk '
        /^        env:$/ { found = 1; next }
        found && $0 !~ /^          / && $0 !~ /^[[:space:]]*$/ { exit }
        found {
            sub(/^          /, "")
            print
        }
    '
}

channel=$(require_job channel)
rust_check=$(require_job rust-check)
spectcl_compat=$(require_job spectcl-compat)
web_frontends=$(require_job web-frontends)
pr_gate=$(require_job pr-gate)
create_release=$(require_job create-release)

[ "$(needs_value "$rust_check")" = '[channel]' ] ||
    fail 'rust-check must depend only on channel so its work overlaps prerequisites'
[ "$(needs_value "$pr_gate")" = '[channel, rust-check, spectcl-compat, web-frontends]' ] ||
    fail 'pr-gate must aggregate channel, rust-check, spectcl-compat and web-frontends'

for job_name in channel rust-check spectcl-compat web-frontends pr-gate; do
    job=$(require_job "$job_name")
    if printf '%s\n' "$job" | grep -Eq '^    continue-on-error:'; then
        fail "$job_name must not ignore failures at job level"
    fi
done

# No channel or front-end step is optional: allowing one to fail can leave a
# successful prerequisite job with missing classification or validation work.
for job_name in channel web-frontends; do
    job=$(require_job "$job_name")
    if printf '%s\n' "$job" | grep -Eq '^        continue-on-error:'; then
        fail "$job_name steps must not ignore failures"
    fi
done
if printf '%s\n' "$channel" | grep -Eq '^        if:'; then
    fail 'channel steps must remain unconditional'
fi

# SpecTcl has exactly two best-effort cache/telemetry steps. Keep the actual
# compatibility command fail-closed and reject any additional ignored step.
spectcl_continue_lines=$(printf '%s\n' "$spectcl_compat" | grep -Ec '^        continue-on-error:' || true)
[ "$spectcl_continue_lines" = 2 ] ||
    fail 'spectcl-compat may ignore only its two cache telemetry steps'
for optional_step_name in 'Set up sccache' 'Report sccache statistics'; do
    optional_step=$(step_block "$spectcl_compat" "$optional_step_name")
    require_line "$optional_step" '        continue-on-error: true' \
        "spectcl-compat optional step must remain explicit: $optional_step_name"
done
spectcl_gate_step=$(step_block "$spectcl_compat" 'Run exact SpecTcl 1.x + 2.0 + real-Tcl compatibility gate')
[ -n "$spectcl_gate_step" ] ||
    fail 'spectcl-compat must define its named compatibility gate step'
spectcl_gate_condition="        if: needs.channel.outputs.spectcl_compat_changed == 'true' && needs.channel.outputs.already_green != 'true'"
require_line "$spectcl_gate_step" "$spectcl_gate_condition" \
    'spectcl-compat gate must retain its exact path and green-result condition'
[ "$(printf '%s\n' "$spectcl_gate_step" | grep -Ec '^        if:' || true)" = 1 ] ||
    fail 'spectcl-compat gate must define exactly one condition'
require_line "$spectcl_gate_step" '        run: make test-spectcl-compat' \
    'spectcl-compat gate must run the unchanged compatibility target'
[ "$(printf '%s\n' "$spectcl_gate_step" | grep -Ec '^        run:' || true)" = 1 ] ||
    fail 'spectcl-compat gate must define exactly one command'
if printf '%s\n' "$spectcl_gate_step" | grep -Eq '^        continue-on-error:'; then
    fail 'spectcl-compat gate step must not ignore failures'
fi

web_frontends_condition="      RUN_WEB_FRONTENDS: \${{ (needs.channel.outputs.web_frontends_changed == 'true' || startsWith(github.ref, 'refs/tags/')) && needs.channel.outputs.already_green != 'true' }}"
require_line "$web_frontends" "$web_frontends_condition" \
    'web-frontends must retain its exact path, tag and green-result condition'
[ "$(printf '%s\n' "$web_frontends" | grep -Ec '^      RUN_WEB_FRONTENDS:' || true)" = 1 ] ||
    fail 'web-frontends must define exactly one run condition'
web_if_lines=$(printf '%s\n' "$web_frontends" | grep -Ec "^        if: env.RUN_WEB_FRONTENDS == 'true'" || true)
[ "$web_if_lines" = 6 ] ||
    fail 'all six conditional web-frontends steps must use the exact run condition'
for web_step_name in \
    'Set up Node' \
    'Enable Corepack (pin npm via packageManager)' \
    'Cache npm registry' \
    'Install both front-end dependency trees' \
    'Shared report front-end (typecheck + lint + asset drift)' \
    'Spec studio front-end (typecheck + lint + build + unit tests)'; do
    web_step=$(step_block "$web_frontends" "$web_step_name")
    require_line "$web_step" "        if: env.RUN_WEB_FRONTENDS == 'true'" \
        "web-frontends step must retain its exact condition: $web_step_name"
    [ "$(printf '%s\n' "$web_step" | grep -Ec '^        if:' || true)" = 1 ] ||
        fail "web-frontends step must define exactly one condition: $web_step_name"
done
report_web_step=$(step_block "$web_frontends" 'Shared report front-end (typecheck + lint + asset drift)')
require_line "$report_web_step" '        run: make typecheck-report-ts lint-report-ts check-report-assets' \
    'web-frontends must retain the complete report validation command'
[ "$(printf '%s\n' "$report_web_step" | grep -Ec '^        run:' || true)" = 1 ] ||
    fail 'report front-end validation must define exactly one command'
studio_web_step=$(step_block "$web_frontends" 'Spec studio front-end (typecheck + lint + build + unit tests)')
require_line "$studio_web_step" '        run: make typecheck-spec-studio-ts lint-spec-studio-ts spec-studio-assets spec-studio-test' \
    'web-frontends must retain the complete Spec Studio validation command'
[ "$(printf '%s\n' "$studio_web_step" | grep -Ec '^        run:' || true)" = 1 ] ||
    fail 'Spec Studio front-end validation must define exactly one command'

# The worker has no independent job-level skip. If channel itself fails,
# Actions marks this worker skipped and the aggregate rejects that result.
# Its expensive work remains step-gated, preserving the pre-release and
# already-green tag behaviour.
if printf '%s\n' "$rust_check" | grep -Eq '^    if:'; then
    fail 'rust-check must not have a job-level skip'
fi
if printf '%s\n' "$rust_check" | grep -Eq '^        continue-on-error:'; then
    fail 'rust-check steps must not ignore failures'
fi
require_contains "$rust_check" 'actions/checkout@' \
    'rust-check must own checkout'
require_contains "$rust_check" 'name: Set up Rust' \
    'rust-check must own Rust setup'
worker_step=$(step_block "$rust_check" 'Run fast CI gate (format + Clippy + generated-file drift)')
[ -n "$worker_step" ] ||
    fail 'rust-check must define the named fast-gate step'
require_line "$worker_step" '        run: make rust-check' \
    'rust-check fast-gate step must run the unchanged make rust-check contract'
require_line "$worker_step" "        if: needs.channel.outputs.prerelease != 'true' && !(startsWith(github.ref, 'refs/tags/') && needs.channel.outputs.already_green == 'true')" \
    'rust-check fast-gate step must preserve pre-release and already-green skips'
worker_if_lines=$(printf '%s\n' "$worker_step" | grep -Ec '^        if:' || true)
[ "$worker_if_lines" = 1 ] ||
    fail 'rust-check fast-gate step must define exactly one condition'
if printf '%s\n' "$worker_step" | grep -Eq '^        continue-on-error:'; then
    fail 'rust-check fast-gate step must not ignore failures'
fi

checkout_line=$(printf '%s\n' "$rust_check" | grep -nF 'actions/checkout@' | head -n 1 | cut -d: -f1)
setup_line=$(printf '%s\n' "$rust_check" | grep -nF 'name: Set up Rust' | head -n 1 | cut -d: -f1)
run_line=$(printf '%s\n' "$rust_check" | grep -nF 'run: make rust-check' | head -n 1 | cut -d: -f1)
[ "$checkout_line" -lt "$setup_line" ] && [ "$setup_line" -lt "$run_line" ] ||
    fail 'rust-check steps must remain checkout, setup, then make rust-check'

# The aggregate is the only job allowed to absorb failed needs and must make
# every prerequisite result explicit. It owns no Rust work of its own.
require_line "$pr_gate" '    if: ${{ always() }}' \
    'pr-gate must always run to propagate failed prerequisites'
pr_gate_if_lines=$(printf '%s\n' "$pr_gate" | grep -Ec '^    if:' || true)
[ "$pr_gate_if_lines" = 1 ] ||
    fail 'pr-gate must define exactly one job-level condition'
if printf '%s\n' "$pr_gate" | grep -Fq 'run: make rust-check'; then
    fail 'pr-gate must not own the Rust work'
fi

# Parse the named aggregate step and its YAML literal block. These checks must
# inspect operative shell, not merely strings elsewhere in the job: a comment
# or an environment value must not be able to satisfy the fail-closed proof.
aggregate_step=$(step_block "$pr_gate" 'Propagate prerequisite gate failures')
[ -n "$aggregate_step" ] ||
    fail 'pr-gate must define the named aggregate step'
if printf '%s\n' "$aggregate_step" | grep -Eq '^        if:'; then
    fail 'aggregate step must not be conditional'
fi
if printf '%s\n' "$aggregate_step" | grep -Eq '^        continue-on-error:'; then
    fail 'aggregate step must not ignore failures'
fi
aggregate_run=$(run_block "$aggregate_step")
[ -n "$aggregate_run" ] ||
    fail 'aggregate step must define an operative shell run block'
aggregate_env=$(env_block "$aggregate_step")
[ -n "$aggregate_env" ] ||
    fail 'aggregate step must define an operative result environment mapping'

aggregate_env_lines=$(printf '%s\n' "$aggregate_env" | sed '/^[[:space:]]*$/d' | wc -l | tr -d ' ')
[ "$aggregate_env_lines" = 4 ] ||
    fail 'aggregate step must define exactly four operative result bindings'
for binding in \
    'CHANNEL_RESULT: ${{ needs.channel.result }}' \
    'RUST_CHECK_RESULT: ${{ needs.rust-check.result }}' \
    'SPECTCL_COMPAT_RESULT: ${{ needs.spectcl-compat.result }}' \
    'WEB_FRONTENDS_RESULT: ${{ needs.web-frontends.result }}'; do
    require_line "$aggregate_env" "$binding" \
        "aggregate env mapping must remain exact (missing: $binding)"
done

aggregate_condition_line=$(printf '%s\n' "$aggregate_run" | sed -n '1p')
[ "$aggregate_condition_line" = 'if [ "$CHANNEL_RESULT" != success ] || [ "$RUST_CHECK_RESULT" != success ] \' ] ||
    fail 'aggregate run must begin with the four-result failure condition'
aggregate_continuation_line=$(printf '%s\n' "$aggregate_run" | sed -n '2p')
[ "$aggregate_continuation_line" = '  || [ "$SPECTCL_COMPAT_RESULT" != success ] || [ "$WEB_FRONTENDS_RESULT" != success ]; then' ] ||
    fail 'aggregate run must test SpecTcl and front-end results in the condition'
require_contains "$aggregate_run" 'exit 1' \
    'aggregate run must fail when a prerequisite result is not success'
require_contains "$aggregate_run" 'fi' \
    'aggregate run must close its failure condition'
if printf '%s\n' "$aggregate_run" | grep -Fq 'exit 0'; then
    fail 'aggregate run must not unconditionally succeed'
fi

expected_release_needs='[channel, pr-gate, rust-tests, rust-tests-heavy, spectcl-compat, lsp-server-wasm, lsp-e2e, cargo-deny, python, web-frontends]'
[ "$(needs_sequence "$create_release")" = "$expected_release_needs" ] ||
    fail 'create-release must retain the exact stable release-gate dependency set'

# Run the contract against deliberate mutations. Nested runs skip this block
# so each mutation tests only the structural assertions above.
if [ "${PR_GATE_CONTRACT_NEGATIVE:-true}" = true ]; then
    tmp=$(mktemp -d "${TMPDIR:-/tmp}/tcl-lsp-pr-gate-contract.XXXXXX")
    trap 'rm -rf "$tmp"' EXIT HUP INT TERM

    expect_rejected() {
        name=$1
        candidate=$2
        if WORKFLOW=$candidate PR_GATE_CONTRACT_NEGATIVE=false sh "$0"; then
            fail "negative control unexpectedly passed: $name"
        fi
    }

    # Missing aggregate edge: the worker would no longer be required.
    candidate=$tmp/missing-needs.yml
    sed 's/needs: \[channel, rust-check, spectcl-compat, web-frontends\]/needs: [channel, spectcl-compat, web-frontends]/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected missing-rust-check-need "$candidate"

    # Job-level skip: a skipped worker would make the required result vacuous.
    candidate=$tmp/job-skip.yml
    sed '/^  rust-check:$/a\    if: false' "$WORKFLOW" > "$candidate"
    expect_rejected rust-check-job-skip "$candidate"

    candidate=$tmp/rust-check-job-continue-on-error.yml
    sed '/^  rust-check:$/a\    continue-on-error: true' \
        "$WORKFLOW" > "$candidate"
    expect_rejected rust-check-job-continue-on-error "$candidate"

    for prerequisite in channel spectcl-compat web-frontends; do
        candidate=$tmp/$prerequisite-job-continue-on-error.yml
        sed "/^  $prerequisite:\$/a\\    continue-on-error: true" \
            "$WORKFLOW" > "$candidate"
        expect_rejected "$prerequisite-job-continue-on-error" "$candidate"
    done

    candidate=$tmp/channel-step-continue-on-error.yml
    sed '/^      - name: Classify changed paths$/a\        continue-on-error: true' \
        "$WORKFLOW" > "$candidate"
    expect_rejected channel-step-continue-on-error "$candidate"

    candidate=$tmp/spectcl-gate-continue-on-error.yml
    sed '/^      - name: Run exact SpecTcl 1.x + 2.0 + real-Tcl compatibility gate$/a\        continue-on-error: true' \
        "$WORKFLOW" > "$candidate"
    expect_rejected spectcl-gate-continue-on-error "$candidate"

    candidate=$tmp/web-frontends-step-continue-on-error.yml
    sed '/^      - name: Shared report front-end (typecheck + lint + asset drift)$/a\        continue-on-error: true' \
        "$WORKFLOW" > "$candidate"
    expect_rejected web-frontends-step-continue-on-error "$candidate"

    candidate=$tmp/channel-step-skip.yml
    sed '/^      - name: Classify changed paths$/a\        if: false' \
        "$WORKFLOW" > "$candidate"
    expect_rejected channel-step-skip "$candidate"

    candidate=$tmp/spectcl-gate-skip.yml
    sed "/^        if: needs.channel.outputs.spectcl_compat_changed == 'true' && needs.channel.outputs.already_green != 'true'$/s//        if: false/" \
        "$WORKFLOW" > "$candidate"
    expect_rejected spectcl-gate-skip "$candidate"

    candidate=$tmp/spectcl-gate-noop.yml
    sed '/^        run: make test-spectcl-compat$/s//        run: true/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected spectcl-gate-noop "$candidate"

    candidate=$tmp/web-frontends-disabled.yml
    sed '/^      RUN_WEB_FRONTENDS:/s//      RUN_WEB_FRONTENDS: false #/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected web-frontends-disabled "$candidate"

    candidate=$tmp/web-frontends-step-skip.yml
    sed "/^        if: env.RUN_WEB_FRONTENDS == 'true'$/s//        if: false/" \
        "$WORKFLOW" > "$candidate"
    expect_rejected web-frontends-step-skip "$candidate"

    candidate=$tmp/web-frontends-noop.yml
    sed '/^        run: make typecheck-report-ts lint-report-ts check-report-assets$/s//        run: true/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected web-frontends-noop "$candidate"

    # A comment containing the required spelling must not disguise a
    # conditional aggregate. Only the exact job-level always() guard is safe.
    candidate=$tmp/conditional-aggregate.yml
    sed 's/^    if: \${{ always() }}$/    if: false # if: ${{ always() }}/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected conditional-aggregate "$candidate"

    # Duplicate YAML keys are parser-dependent; an exact safe guard followed
    # by a second condition must not satisfy the structural contract.
    candidate=$tmp/duplicate-aggregate-condition.yml
    sed '/^    if: \${{ always() }}$/a\    if: false' \
        "$WORKFLOW" > "$candidate"
    expect_rejected duplicate-aggregate-condition "$candidate"

    # Step-level skip: the job-level always() guard is insufficient if the
    # only fail-closed step can be skipped.
    candidate=$tmp/conditional-aggregate-step.yml
    sed '/^      - name: Propagate prerequisite gate failures$/a\        if: false' \
        "$WORKFLOW" > "$candidate"
    expect_rejected conditional-aggregate-step "$candidate"

    # Neither the aggregate step nor its job may turn a failing prerequisite
    # into a successful required status.
    candidate=$tmp/aggregate-step-continue-on-error.yml
    sed '/^      - name: Propagate prerequisite gate failures$/a\        continue-on-error: true' \
        "$WORKFLOW" > "$candidate"
    expect_rejected aggregate-step-continue-on-error "$candidate"

    candidate=$tmp/aggregate-job-continue-on-error.yml
    sed '/^  pr-gate:$/a\    continue-on-error: true' \
        "$WORKFLOW" > "$candidate"
    expect_rejected aggregate-job-continue-on-error "$candidate"

    # Weakened tag gate: pre-release/already-green semantics must remain exact.
    candidate=$tmp/tag-gate.yml
    sed "s@if: needs.channel.outputs.prerelease != 'true' && !(startsWith(github.ref, 'refs/tags/') && needs.channel.outputs.already_green == 'true')@if: true@" \
        "$WORKFLOW" > "$candidate"
    expect_rejected weakened-tag-gate "$candidate"

    candidate=$tmp/rust-check-step-continue-on-error.yml
    sed '/^      - name: Run fast CI gate (format + Clippy + generated-file drift)$/a\        continue-on-error: true' \
        "$WORKFLOW" > "$candidate"
    expect_rejected rust-check-step-continue-on-error "$candidate"

    candidate=$tmp/duplicate-rust-check-step-condition.yml
    sed "/^        if: needs.channel.outputs.prerelease != 'true'/a\\        if: false" \
        "$WORKFLOW" > "$candidate"
    expect_rejected duplicate-rust-check-step-condition "$candidate"

    # Comment decoy in place of the worker command: only the operative named
    # step may satisfy the make-rust-check contract.
    candidate=$tmp/commented-worker.yml
    sed 's/^        run: make rust-check$/        # run: make rust-check\n        run: true/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected commented-worker-command "$candidate"

    # Comment decoy and unconditional success: preserving the expected result
    # strings in comments must not fool the operative aggregate-run proof.
    candidate=$tmp/unconditional-aggregate.yml
    awk '
        !replaced && index($0, "          if [ \"$CHANNEL_RESULT\" != success ]") == 1 {
            print "          # if [ \"$CHANNEL_RESULT\" != success ]"
            print "          # [ \"$RUST_CHECK_RESULT\" != success ]"
            print "          # [ \"$SPECTCL_COMPAT_RESULT\" != success ]"
            print "          # [ \"$WEB_FRONTENDS_RESULT\" != success ]"
            print "          if true; then"
            print "            echo \"aggregate unexpectedly unconditional\""
            print "            exit 0"
            print "          fi"
            replaced = 1
            skip_next = 1
            next
        }
        skip_next { skip_next = 0; next }
        { print }
    ' "$WORKFLOW" > "$candidate"
    expect_rejected unconditional-aggregate-success "$candidate"

    # Literal-success result bindings: retaining the expected expressions in
    # comments must not fool the exact operative env mapping proof.
    candidate=$tmp/literal-result-bindings.yml
    awk '
        $0 == "      - name: Propagate prerequisite gate failures" { in_step = 1 }
        in_step && /^      - name: / && $0 != "      - name: Propagate prerequisite gate failures" { in_step = 0; in_env = 0 }
        in_step && /^        env:$/ { in_env = 1; print; next }
        in_env && /^        run:/ { in_env = 0; print; next }
        in_env && /^          (CHANNEL_RESULT|RUST_CHECK_RESULT|SPECTCL_COMPAT_RESULT|WEB_FRONTENDS_RESULT):/ {
            original = $0
            sub(/^          /, "", original)
            key = original
            sub(/:.*/, "", key)
            print "          # " original
            print "          " key ": success"
            next
        }
        { print }
    ' "$WORKFLOW" > "$candidate"
    expect_rejected literal-success-result-bindings "$candidate"

    # The release graph must continue to consume the stable required context.
    candidate=$tmp/release-without-pr-gate.yml
    sed '/^  create-release:/,/^  [A-Za-z0-9_-]*:/ s/\[channel, pr-gate, rust-tests,/[channel, rust-tests,/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected release-without-pr-gate "$candidate"

    # Work moved back behind a prerequisite: this would restore the serial
    # critical path that the independent worker is meant to remove.
    candidate=$tmp/serial-worker.yml
    sed '/^  rust-check:/,/^  pr-gate:/ s/^    needs: \[channel\]$/    needs: [channel, spectcl-compat]/' \
        "$WORKFLOW" > "$candidate"
    expect_rejected serial-rust-worker "$candidate"
fi

echo "pr-gate overlap and fail-closed contract tests passed"
