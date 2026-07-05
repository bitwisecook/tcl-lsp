#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Parse an F5 `profile_base.conf` into the tcl-registry generated defaults table.

For each `ltm profile <type> <name> { ... }` object, the *root* default for a
type is the block that has no `defaults-from` (there is exactly one per type).
We extract that block's fields:
  - `key value`          -> scalar field
  - `key { a b c }`      -> list field  -> value = "a b c"  (no nested braces)
  - `key { k { ... } }`  -> object field -> value = "{ ... }" (nested braces kept)
and emit a Rust `pub static PROFILE_DEFAULTS_GENERATED` array. All ranges are
UNBOUNDED (one snapshot); hand-authored cross-version splits live in mod.rs and
are merged at load time.
"""
import re
import sys

SRC = sys.argv[1]
OUT = sys.argv[2]

with open(SRC, encoding="utf-8", errors="replace") as f:
    text = f.read()


def norm_type(t: str) -> str:
    """tmsh type -> registry uppercase key: `client-ssl` -> `CLIENTSSL`."""
    return re.sub(r"[^A-Za-z0-9]", "", t).upper()


def split_top_objects(s: str):
    """Yield (header_line, body) for every depth-0 `header { ... }` object."""
    i, n = 0, len(s)
    while i < n:
        # find next line start of a top-level object
        nl = s.find("{", i)
        if nl == -1:
            break
        header = s[i:nl].strip().splitlines()
        header = header[-1].strip() if header else ""
        depth = 1
        j = nl + 1
        while j < n and depth:
            c = s[j]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            j += 1
        body = s[nl + 1 : j - 1]
        yield header, body
        i = j


def parse_fields(body: str):
    """Parse one profile body into ordered (field, value) pairs at depth 1."""
    fields = []
    i, n = 0, len(body)
    line = ""
    # We walk char by char to respect nested braces on the value side.
    # Tokenise into logical statements separated by newlines at depth 0.
    tokens = []
    buf = []
    depth = 0
    for c in body:
        if c == "{":
            depth += 1
            buf.append(c)
        elif c == "}":
            depth -= 1
            buf.append(c)
        elif c == "\n" and depth == 0:
            tokens.append("".join(buf))
            buf = []
        else:
            buf.append(c)
    if buf:
        tokens.append("".join(buf))

    for tok in tokens:
        stmt = tok.strip()
        if not stmt:
            continue
        m = re.match(r"^([A-Za-z0-9_.\-]+)\s*(.*)$", stmt, re.DOTALL)
        if not m:
            continue
        key = m.group(1)
        rest = m.group(2).strip()
        if key == "defaults-from":
            continue
        if rest.startswith("{") and rest.endswith("}"):
            inner = rest[1:-1].strip()
            inner_norm = re.sub(r"\s+", " ", inner).strip()
            if "{" in inner:  # object -> keep braces, flattened
                value = "{ " + inner_norm + " }" if inner_norm else "{ }"
            else:  # list -> inner items only
                value = inner_norm
        else:
            value = re.sub(r"\s+", " ", rest).strip()
        fields.append((key, value))
    return fields


# type -> {name: fields}, and detect the root (no defaults-from)
roots = {}  # tmsh_type -> (fields)
for header, body in split_top_objects(text):
    m = re.match(r"^ltm profile ([A-Za-z0-9_\-]+)\s+(\S+)\s*$", header)
    if not m:
        continue
    tmsh_type = m.group(1)
    # A child profile has `defaults-from <parent>` with a real parent; the base
    # default either omits `defaults-from` or sets it to `none`.
    if re.search(r"(^|\n)\s*defaults-from\s+(?!none\b)\S", body):
        continue  # child profile
    fields = parse_fields(body)
    # keep the first root seen per type (the base default)
    if tmsh_type not in roots:
        roots[tmsh_type] = fields


def rs_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


# Known cross-version defaults a single snapshot can't express. Each maps a
# (profile-key, field) to an ordered list of (value, range-expr) — the range-expr
# is emitted verbatim as a Rust `VersionRange`. The snapshot's own value is
# asserted to equal the current (open-upper-bound) override so drift is caught.
OVERRIDES = {
    # TLS 1.3 shipped disabled-by-default in the base client/server-ssl profile
    # from 14.0 (base `options` gained `no-tlsv1.3`); before that the base
    # options were just `dont-insert-empty-fragments`.
    ("CLIENTSSL", "options"): [
        (
            "dont-insert-empty-fragments",
            "VersionRange::until(BigipVersion::new(14, 0, 0, 0))",
        ),
        (
            "dont-insert-empty-fragments no-tlsv1.3",
            "VersionRange::from(BigipVersion::new(14, 0, 0, 0))",
        ),
    ],
    ("SERVERSSL", "options"): [
        (
            "dont-insert-empty-fragments",
            "VersionRange::until(BigipVersion::new(14, 0, 0, 0))",
        ),
        (
            "dont-insert-empty-fragments no-tlsv1.3",
            "VersionRange::from(BigipVersion::new(14, 0, 0, 0))",
        ),
    ],
}


lines = []
lines.append("// @generated by scripts/registry-audit/gen_profile_defaults.py from")
lines.append("// f5-corkscrew's Apache-2.0 profile_base.conf (TMOS default profile base).")
lines.append("// DO NOT EDIT — regenerate to refresh. Cross-version splits live in the")
lines.append("// parent module and are merged over this table at load time.")
lines.append("")
lines.append("use super::{BigipVersion, FieldDefault, ProfileDefaults, VersionRange};")
lines.append("")
lines.append(
    "/// Baseline profile field defaults parsed from `profile_base.conf` "
    "(all\n/// version-unbounded). See [`super::PROFILE_DEFAULTS`] for the merged view."
)
lines.append("pub static PROFILE_DEFAULTS_GENERATED: &[ProfileDefaults] = &[")
for tmsh_type in sorted(roots):
    fields = roots[tmsh_type]
    key = norm_type(tmsh_type)
    lines.append("    ProfileDefaults {")
    lines.append(f'        profile: "{key}",')
    lines.append(f'        tmsh_kind: "ltm profile {tmsh_type}",')
    lines.append("        fields: &[")
    for field, value in fields:
        override = OVERRIDES.get((key, field))
        if override:
            # Sanity: the snapshot's value must match the current override.
            current = next((v for v, r in override if "::from" in r or "UNBOUNDED" in r), None)
            if current is not None and current != value:
                raise SystemExit(
                    f"drift: {key}.{field} snapshot={value!r} != override current={current!r}"
                )
            for ov_value, ov_range in override:
                lines.append(
                    f'            FieldDefault {{ field: "{rs_escape(field)}", '
                    f'value: "{rs_escape(ov_value)}", range: {ov_range} }},'
                )
        else:
            lines.append(
                f'            FieldDefault {{ field: "{rs_escape(field)}", '
                f'value: "{rs_escape(value)}", range: VersionRange::UNBOUNDED }},'
            )
    lines.append("        ],")
    lines.append("    },")
lines.append("];")
lines.append("")

with open(OUT, "w", encoding="utf-8") as f:
    f.write("\n".join(lines))

print(f"types: {len(roots)}  total fields: {sum(len(v) for v in roots.values())}")
print("sample types:", ", ".join(sorted(roots)[:12]))
