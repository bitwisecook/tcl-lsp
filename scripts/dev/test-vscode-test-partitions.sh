#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
node "$root/scripts/dev/verify-vscode-test-partitions.mjs" "$root" >/dev/null

manifest="$root/editors/vscode/test-partitions.json"
for partition in 1 2 3; do
  count=$(jq -r --arg p "$partition" '.partitions[$p] | length' "$manifest")
  test "$count" -gt 0
done
test "$(jq '[.partitions[][]] | length' "$manifest")" -eq 106
test "$(jq '[.partitions[][]] | unique | length' "$manifest")" -eq 106
test "$(jq '.expected_tests.single_root.identities' "$manifest")" -eq 978
test "$(jq '.expected_tests.single_root.passed' "$manifest")" -eq 977
test "$(jq '.expected_tests.single_root.pending' "$manifest")" -eq 1
test "$(jq '.expected_tests.multi_folder.identities' "$manifest")" -eq 14
test "$(jq '.expected_tests.multi_folder.passed' "$manifest")" -eq 14
test "$(jq '.expected_tests.multi_folder.pending' "$manifest")" -eq 0
test "$(jq -r '.multi_folder_files | join(",")' "$manifest")" = "multiFolderConfig.test.js"
echo "VS Code test partitions: exact-once proof passed (106 files, 977 passed + 1 pending single-root identities, 14 multi-folder tests, 3 partitions)"

workflow="${root}/.github/workflows/ci.yml"
check_workflow() {
  set -uo pipefail
  local file=$1 partition_job multi_job aggregate_job propagation download verify
  job_block() {
    awk -v wanted="  $1:" '$0 == wanted { in_job=1; found=1 } in_job && /^  [A-Za-z0-9_-]+:/ && $0 != wanted { exit } in_job { print } END { if (!found) exit 1 }' "$file"
  }
  step_block() {
    local job=$1 name=$2
    awk -v wanted="      - name: $name" '$0 == wanted { in_step=1; found=1 } in_step && /^      (- name:|#)/ && $0 != wanted { exit } in_step && /^  [#A-Za-z0-9_-]/ { exit } in_step { print } END { if (!found) exit 1 }' <<<"$job"
  }
  partition_job=$(job_block test-ext-partition); multi_job=$(job_block test-ext-multi-folder); aggregate_job=$(job_block test-ext)
  test "$(grep -Fc '  test-ext-partition:' "$file")" -eq 1
  test "$(grep -Fc '  test-ext-multi-folder:' "$file")" -eq 1
  test "$(grep -Fc '  test-ext:' "$file")" -eq 1
  test "$(grep -Fc '      - name: Verify VS Code test partitions' <<<"$partition_job")" -eq 1
  test "$(grep -Fc '      - name: Verify VS Code test partitions' <<<"$multi_job")" -eq 1
  test "$(grep -Fc '      - name: Propagate extension prerequisite failures' <<<"$aggregate_job")" -eq 1
  test "$(grep -Fc '      - name: Download extension test metadata' <<<"$aggregate_job")" -eq 1
  test "$(grep -Fc '      - name: Verify extension test exact-once metadata' <<<"$aggregate_job")" -eq 1
  test "$(grep -Fc '          - { index: 1, id: one }' <<<"$partition_job")" -eq 1
  test "$(grep -Fc '          - { index: 2, id: two }' <<<"$partition_job")" -eq 1
  test "$(grep -Fc '          - { index: 3, id: three }' <<<"$partition_job")" -eq 1
  grep -Fqx '    needs: [channel, build-tcl-lsp-server]' <<<"$partition_job"
  grep -Fqx '      TCL_LSP_TEST_PARTITION: ${{ matrix.partition.index }}/3' <<<"$partition_job"
  grep -Fqx '          name: vscode-test-partition-${{ matrix.partition.id }}' <<<"$partition_job"
  test "$(step_block "$partition_job" 'Verify VS Code test partitions')" = "$(cat <<'EOF'
      - name: Verify VS Code test partitions
        if: env.RUN_EXT == 'true'
        run: make check-vscode-test-partitions
EOF
)" || return 1
  grep -Fqx '        run: make test-ext-multi-folder' <<<"$multi_job"
  test "$(step_block "$multi_job" 'Verify VS Code test partitions')" = "$(cat <<'EOF'
      - name: Verify VS Code test partitions
        if: env.RUN_EXT == 'true'
        run: make check-vscode-test-partitions
EOF
)" || return 1
  grep -Fqx '    if: ${{ always() }}' <<<"$aggregate_job"
  grep -Fqx '    needs: [channel, test-ext-partition, test-ext-multi-folder]' <<<"$aggregate_job"
  propagation=$(step_block "$aggregate_job" 'Propagate extension prerequisite failures')
  download=$(step_block "$aggregate_job" 'Download extension test metadata')
  verify=$(step_block "$aggregate_job" 'Verify extension test exact-once metadata')
  test "$propagation" = "$(cat <<'EOF'
      - name: Propagate extension prerequisite failures
        env:
          CHANNEL: ${{ needs.channel.result }}
          PARTITIONS: ${{ needs.test-ext-partition.result }}
          MULTI: ${{ needs.test-ext-multi-folder.result }}
        run: |
          if [ "$CHANNEL" != success ] || { [ "$RUN_EXT" = true ] && { [ "$PARTITIONS" != success ] || [ "$MULTI" != success ]; }; }; then
            echo "channel=$CHANNEL partitions=$PARTITIONS multi=$MULTI" >&2
            exit 1
          fi
EOF
)" || return 1
  test "$download" = "$(cat <<'EOF'
      - name: Download extension test metadata
        if: env.RUN_EXT == 'true'
        uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1
        with:
          pattern: vscode-test-*
          path: ${{ runner.temp }}/vscode-test-results
          merge-multiple: false
EOF
)" || return 1
  test "$verify" = "$(cat <<'EOF'
      - name: Verify extension test exact-once metadata
        if: env.RUN_EXT == 'true'
        run: |
          mkdir -p "$RUNNER_TEMP/vscode-test-results/partition-1" "$RUNNER_TEMP/vscode-test-results/partition-2" "$RUNNER_TEMP/vscode-test-results/partition-3" "$RUNNER_TEMP/vscode-test-results/multi-folder"
          for pair in '1 one' '2 two' '3 three'; do set -- $pair; mv "$RUNNER_TEMP/vscode-test-results/vscode-test-partition-$2/mocha-result.json" "$RUNNER_TEMP/vscode-test-results/partition-$1/"; done
          mv "$RUNNER_TEMP/vscode-test-results/vscode-test-multi-folder/mocha-result-multifolder.json" "$RUNNER_TEMP/vscode-test-results/multi-folder/"
          node scripts/dev/verify-vscode-test-results.mjs "$GITHUB_WORKSPACE" "$RUNNER_TEMP/vscode-test-results"
EOF
)" || return 1
}
check_workflow "$workflow"
# Each mutation must be rejected by the same checker, not a separate grep.
reject_mutation() {
  set +e
  check_workflow "$1" >/dev/null 2>&1
  local status=$?
  set -e
  test "$status" -ne 0
}
fixture=""
mutated=$(mktemp)
trap 'rm -f "$mutated"; rm -rf "$fixture"' EXIT
sed '\|node scripts/dev/verify-vscode-test-results.mjs|d' "$workflow" >"$mutated"; reject_mutation "$mutated"
sed 's/        if: env.RUN_EXT == '\''true'\''/        # if: env.RUN_EXT == '\''true'\''\n        if: true/' "$workflow" >"$mutated"; reject_mutation "$mutated"
sed 's/          PARTITIONS: .*/          # PARTITIONS: expected mapping\n          PARTITIONS: success/' "$workflow" >"$mutated"; reject_mutation "$mutated"
sed 's/\[ "$CHANNEL" != success \]/false/' "$workflow" >"$mutated"; reject_mutation "$mutated"
sed 's/exit 1/true/' "$workflow" >"$mutated"; reject_mutation "$mutated"
sed 's|          node scripts/dev/verify-vscode-test-results|          # node scripts/dev/verify-vscode-test-results|' "$workflow" >"$mutated"; reject_mutation "$mutated"
sed '/      - name: Verify extension test exact-once metadata/i\      - name: Verify extension test exact-once metadata\n        if: true\n        run: true' "$workflow" >"$mutated"; reject_mutation "$mutated"
rm -f "$mutated"
echo "VS Code workflow lifecycle contract passed"
grep -Fq 'check-vscode-test-partitions' "$root/Makefile"
make_target_block() {
  awk -v wanted="$1:" '$0 == wanted { in_target=1; found=1 } in_target && /^[^[:space:]#][^:]*:/ && $0 != wanted { exit } in_target { print } END { if (!found) exit 1 }' "$root/Makefile"
}
for target in test-ext-partition test-ext-multi-folder; do
  target_block=$(make_target_block "$target")
  test "$(grep -Fc 'cd "$(EXT_DIR)" && "$(NPM)" run copy-canonical && "$(NPM)" run bundle;' <<<"$target_block")" -eq 1 || {
    echo "$target must copy canonical assets before bundling" >&2
    exit 1
  }
done
grep -Fq 'cd "$(EXT_DIR)" && xvfb-run -a node ./out/test/runTest.js;' "$root/Makefile"
test "$(grep -Fc 'mocha.suite.beforeAll' "$root/editors/vscode/src/test/index.ts")" -eq 1
test "$(grep -Fc 'mocha.suite.afterAll' "$root/editors/vscode/src/test/index.ts")" -eq 1
test "$(grep -Ec '^(suiteSetup|suiteTeardown)\(' "$root/editors/vscode/src/test/serverHealth.test.ts")" -eq 0
test "$(jq -r '.scripts.test' "$root/editors/vscode/package.json")" = 'node ./out/test/runTest.js'
test "$(jq -r '.scripts.pretest' "$root/editors/vscode/package.json")" = 'npm run compile'
test "$(grep -Fc 'editors/vscode/* | scripts/dev/json-no-duplicate-keys.mjs | scripts/dev/test-vscode-test-partitions.sh | scripts/dev/verify-vscode-test-partitions.mjs | scripts/dev/verify-vscode-test-results.mjs | rust/*' "$workflow")" -eq 1
grep -Fq '"status": "hosted-duration-balanced"' "$manifest"
grep -Fq '"measurement_field": "fileDurationsMs"' "$manifest"
echo "VS Code partition gate call-chain contract passed"

# Fresh-checkout proof: the verifier must use tracked TypeScript sources when
# ignored compilation output is absent, and must fail on a source inventory
# mutation rather than silently accepting a stale output tree.
fixture=$(mktemp -d)
mkdir -p "$fixture/editors/vscode"
cp "$manifest" "$fixture/editors/vscode/test-partitions.json"
cp -R "$root/editors/vscode/src" "$fixture/editors/vscode/src"
node "$root/scripts/dev/verify-vscode-test-partitions.mjs" "$fixture" >/dev/null
sed 's/  "version": 1,/  "version": 1,\n  "version": 1,/' "$fixture/editors/vscode/test-partitions.json" >"$mutated"
mv "$mutated" "$fixture/editors/vscode/test-partitions.json"
if node "$root/scripts/dev/verify-vscode-test-partitions.mjs" "$fixture" >/dev/null 2>&1; then
  echo "duplicate manifest key unexpectedly passed" >&2
  exit 1
fi
cp "$manifest" "$fixture/editors/vscode/test-partitions.json"
rm "$fixture/editors/vscode/src/test/aliasTracking.test.ts"
if node "$root/scripts/dev/verify-vscode-test-partitions.mjs" "$fixture" >/dev/null 2>&1; then
  echo "source inventory mutation unexpectedly passed" >&2
  exit 1
fi
echo "VS Code fresh source inventory contract passed"

# Runner-shaped metadata proves the aggregate without extension dependencies.
results="$fixture/results"
mkdir -p "$results/partition-1" "$results/partition-2" "$results/partition-3" "$results/multi-folder"
node - "$manifest" "$results" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");
const [manifestPath, results] = process.argv.slice(2);
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
let remaining = manifest.expected_tests.single_root.identities;
for (const [partition, files] of Object.entries(manifest.partitions)) {
  const isLast = partition === "3";
  const count = isLast ? remaining : files.length;
  remaining -= count;
  const identities = files.map((file, index) => `${file}:synthetic ${index}`);
  while (identities.length < count) identities.push(`${files[0]}:synthetic ${identities.length}`);
  const pending = files.includes("specPackTorture.test.js") ? 1 : 0;
  const durations = Object.fromEntries(files.map((file) => [file, 1]));
  fs.writeFileSync(
    path.join(results, `partition-${partition}`, "mocha-result.json"),
    JSON.stringify({
      failures: 0,
      partition: { index: Number(partition), count: 3 },
      files,
      discoveredTestIdentities: identities,
      testIdentities: identities,
      fileDurationsMs: durations,
      testsStarted: count,
      testsCompleted: count,
      testsPassed: count - pending,
      testsPending: pending,
    }),
  );
}
const files = manifest.multi_folder_files;
const count = manifest.expected_tests.multi_folder.identities;
const identities = Array.from({ length: count }, (_, index) => `${files[0]}:synthetic ${index}`);
fs.writeFileSync(
  path.join(results, "multi-folder", "mocha-result-multifolder.json"),
  JSON.stringify({
    failures: 0,
    files,
    discoveredTestIdentities: identities,
    testIdentities: identities,
    fileDurationsMs: Object.fromEntries(files.map((file) => [file, 1])),
    testsStarted: count,
    testsCompleted: count,
    testsPassed: count,
    testsPending: 0,
  }),
);
NODE
node "$root/scripts/dev/verify-vscode-test-results.mjs" "$root" "$results" >/dev/null
result="$results/partition-1/mocha-result.json"
sed 's/{/{"failures":0,/' "$result" >"$mutated"
mv "$mutated" "$result"
if node "$root/scripts/dev/verify-vscode-test-results.mjs" "$root" "$results" >/dev/null 2>&1; then
  echo "duplicate result key unexpectedly passed" >&2
  exit 1
fi
echo "VS Code aggregate metadata contract passed"
