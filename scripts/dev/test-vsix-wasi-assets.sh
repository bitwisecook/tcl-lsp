#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Exercise content custody and the real packaging rule with a fake builder.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fixture="$work/repo"
mkdir -p "$fixture"/{rust/tcl-lsp-server-wasi/{src,dist},rust/tcl-lsp-server/src/nested,rust/tcl-registry/src,rust/tcl-dialect/src,rust/tcl-syntax/src,rust/xtask/src,runtime,specs,scripts/dev,.cargo,grammars,samples,.github,ai/claude/skills}
for file in Makefile Cargo.toml Cargo.lock rust-toolchain.toml \
    rust/tcl-lsp-server-wasi/Cargo.toml rust/tcl-lsp-server-wasi/Cargo.lock \
    rust/tcl-lsp-server-wasi/src/main.rs rust/tcl-lsp-server/src/nested/body.rs \
    rust/tcl-dialect/reference.tsv specs/fixture.tclspec .cargo/config.toml; do
    printf 'original\n' > "$fixture/$file"
done
cp "$repo/scripts/dev/vsix-wasi-assets.sh" "$fixture/scripts/dev/"
cp "$repo/rust/tcl-lsp-server-wasi/build-wasi.sh" "$fixture/rust/tcl-lsp-server-wasi/"
git -C "$fixture" init -q
git -C "$fixture" add .
git -C "$fixture" -c user.name=fixture -c user.email=fixture@example.invalid commit -qm fixture
helper="$fixture/scripts/dev/vsix-wasi-assets.sh"
module="$fixture/rust/tcl-lsp-server-wasi/dist/tcl-lsp-server-wasi.wasm"
receipt="$fixture/rust/tcl-lsp-server-wasi/dist/tcl-lsp-server-wasi.sources.sha256"
before="$work/before"

seal() {
    bash "$helper" snapshot "$fixture" "$before"
    printf 'wasm fixture\n' > "$module"
    bash "$helper" seal "$fixture" "$module" "$before" "$receipt"
}
check() { bash "$helper" check "$fixture" "$module" "$receipt"; }
reject() {
    if check; then echo "ERROR: stale WASI asset accepted: $1" >&2; exit 1; fi
}
seal
check
# Downloaded artifact timestamps and checkout paths do not carry authority.
touch -t 200001010000 "$module" "$receipt"
check
cp -a "$fixture" "$work/downloaded"
bash "$helper" check "$work/downloaded" \
    "$work/downloaded/rust/tcl-lsp-server-wasi/dist/tcl-lsp-server-wasi.wasm" \
    "$work/downloaded/rust/tcl-lsp-server-wasi/dist/tcl-lsp-server-wasi.sources.sha256"

for file in rust/tcl-lsp-server/src/nested/body.rs rust/tcl-dialect/reference.tsv \
    specs/fixture.tclspec Cargo.toml Cargo.lock \
    rust/tcl-lsp-server-wasi/Cargo.lock .cargo/config.toml; do
    cp -p "$fixture/$file" "$work/original"
    printf 'changed with retained timestamp\n' > "$fixture/$file"
    touch -r "$work/original" "$fixture/$file"
    reject "$file"
    cp -p "$work/original" "$fixture/$file"
    check
done
printf 'generated\n' > "$fixture/rust/tcl-lsp-server/src/generated.rs"
reject 'new generated source'
rm "$fixture/rust/tcl-lsp-server/src/generated.rs"
check
mv "$fixture/specs/fixture.tclspec" "$work/pack"
reject 'deleted literal asset'
mv "$work/pack" "$fixture/specs/fixture.tclspec"
check
printf 'different module\n' > "$module"
reject 'changed module'
seal
mv "$receipt" "$work/receipt"
reject 'missing receipt'
mv "$work/receipt" "$receipt"
CARGO_PROFILE_RELEASE_LTO=thin reject 'different release profile'
TCL_LSP_TRACE_COARSE_SOURCE=1 reject 'different compiled trace setting'
RUSTFLAGS=$'-C\nopt-level=1' seal
RUSTFLAGS=$'-C\nopt-level=1' check
RUSTFLAGS=$'-C\nopt-level=2' reject 'multiline flag continuation changed'
seal
mkdir "$work/cargo-home"
CARGO_HOME="$work/cargo-home" seal
CARGO_HOME="$work/cargo-home" check
printf '[build]\njobs=2\n' > "$work/cargo-home/config.toml"
CARGO_HOME="$work/cargo-home" reject 'new external Cargo configuration'
rm "$work/cargo-home/config.toml"
check

bash "$helper" snapshot "$fixture" "$before"
printf 'concurrent edit\n' >> "$fixture/rust/tcl-lsp-server/src/nested/body.rs"
rm "$receipt"
if bash "$helper" seal "$fixture" "$module" "$before" "$receipt"; then
    echo 'ERROR: changed-during-build source received a receipt' >&2; exit 1
fi
test ! -e "$receipt"
seal

# A fresh receipt must reuse the downloaded module; stale bytes must consult
# the builder. The builder writes only this fixture and never invokes Cargo.
builder="$work/builder"
cat > "$builder" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf 'called\\n' >> '$work/calls'
bash '$helper' snapshot '$fixture' '$before'
printf 'rebuilt fixture\\n' > '$module'
bash '$helper' seal '$fixture' '$module' '$before' '$receipt'
EOF
chmod +x "$builder"
package_check() {
    make --no-print-directory -f "$repo/Makefile" "ROOT=$fixture/" \
        "MAKE=$builder" prepare-vsix-wasi-assets
}
package_check
test ! -e "$work/calls"
cp -p "$fixture/rust/tcl-lsp-server/src/nested/body.rs" "$work/old-source"
printf 'stale shared source\n' >> "$fixture/rust/tcl-lsp-server/src/nested/body.rs"
touch -r "$work/old-source" "$fixture/rust/tcl-lsp-server/src/nested/body.rs"
package_check
test "$(wc -l < "$work/calls")" -eq 1
package_check
test "$(wc -l < "$work/calls")" -eq 1
rm "$receipt"
package_check
test "$(wc -l < "$work/calls")" -eq 2
mkdir "$work/bin" "$work/unrelated"
printf 'preserve\n' > "$work/unrelated/sentinel"
cat > "$work/bin/cargo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
test "\$CARGO_TARGET_DIR" = '$fixture/rust/tcl-lsp-server-wasi/target'
printf '%s\\n' "\$*" >> '$work/cargo-calls'
case "\$1" in clean) exit 0;; build) exit 77;; *) exit 99;; esac
EOF
chmod +x "$work/bin/cargo"
if PATH="$work/bin:$PATH" CARGO_TARGET_DIR="$work/unrelated" \
    bash "$fixture/rust/tcl-lsp-server-wasi/build-wasi.sh"; then
    echo 'ERROR: mock compilation should stop the builder' >&2; exit 1
fi
test "$(sed -n '1p' "$work/cargo-calls")" = clean
test "$(sed -n '2p' "$work/cargo-calls")" = 'build --target wasm32-wasip1 --release'
test "$(cat "$work/unrelated/sentinel")" = preserve
test ! -e "$fixture/rust/tcl-lsp-server-wasi/target"
test ! -e "$receipt"
echo 'WASI source/output packaging contract passed'
