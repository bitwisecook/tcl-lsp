#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Generate a proposed one-owner trace patch; never edit the Rust source."""

import difflib
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
OWNER = ROOT / "rust/tcl-compiler/src/command_binding/declaration_preview.rs"
EXPECTED = "aa7aa950137ebea78be511866a1b4c4d27f61cc301d3ed8d4b47517c02dacb66"
PROBE = ROOT / "experiments/name-handling/tcltest-shared-frame/probe.tcl"
PROBE_SHA = "82f6602d5492c1b321f6a26186f80741e5fd4b1a3f9c4a9f09d05dae508911ef"

BEFORE = """        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )?;
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        if !self.matches_original_source_image(site.source.source_image(), input.lexer_config()) {
            return None;
        }
        if tokens
            .source_binding
            .as_ref()?
            .original_source_advice_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
"""

AFTER = """        #[cfg(test)]
        let trace_enabled = std::env::var_os("TCL_LSP_TRACE_W210_INNER_PREDICATE").is_some()
            && self.root_origin.as_ref().is_some_and(|origin| {
                origin.source_image().bytes()
                    == include_bytes!("../../../../experiments/name-handling/tcltest-shared-frame/probe.tcl")
            });
        #[cfg(test)]
        let trace_hash = {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            if let Some(origin) = &self.root_origin {
                origin.source_image().bytes().hash(&mut hash);
            }
            hash.finish()
        };
        #[cfg(test)]
        let trace = |stage: &str, verdict: &str| {
            if trace_enabled {
                eprintln!("W210_INNER stage={stage} verdict={verdict} source_hash={trace_hash:016x}");
            }
        };
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        );
        #[cfg(test)]
        trace("metadata-context", if metadata.is_some() { "Some" } else { "Refused" });
        let metadata = metadata?;
        let binding = tokens.source_binding.as_ref();
        #[cfg(test)]
        trace("token-binding", if binding.is_some() { "Some" } else { "Refused" });
        let binding = binding?;
        let site = binding.invocation_site();
        #[cfg(test)]
        trace("original-site", if site.is_some() { "Some" } else { "Refused" });
        let site = site?;
        #[cfg(test)]
        if trace_enabled {
            let input_equal = binding.lookup_state.as_ref().map(|snapshot| {
                snapshot.state.baseline.metadata_context.source_analysis_input() == Some(input)
            });
            eprintln!("W210_INNER site={} input_equal={input_equal:?}", site.offset);
        }
        let image_current = self.matches_original_source_image(site.source.source_image(), input.lexer_config());
        #[cfg(test)]
        trace("original-image-config", if image_current { "Current" } else { "Refused" });
        if !image_current {
            return None;
        }
        let purpose = tokens
            .source_binding
            .as_ref()?
            .original_source_advice_purpose(tokens, metadata, registry);
        #[cfg(test)]
        trace("original-source-advice-purpose", match purpose.as_ref() {
            Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical) => "Logical",
            Some(_) => "NonLogical",
            None => "Refused",
        });
        if purpose? == crate::command_binding::OriginalInvocationSourcePurpose::Logical
"""


def main():
    before = OWNER.read_bytes()
    if hashlib.sha256(before).hexdigest() != EXPECTED:
        raise SystemExit("Owner changed; inspect current source before regenerating.")
    if hashlib.sha256(PROBE.read_bytes()).hexdigest() != PROBE_SHA:
        raise SystemExit("Original fixture changed; refuse different input.")
    text = before.decode()
    if text.count(BEFORE) != 1:
        raise SystemExit("Expected factory prefix is not unique.")
    after = text.replace(BEFORE, AFTER, 1)
    relative = str(OWNER.relative_to(ROOT))
    patch = "".join(difflib.unified_diff(
        text.splitlines(True), after.splitlines(True),
        fromfile="a/" + relative, tofile="b/" + relative,
    ))
    target = Path(__file__).with_name("temporary.patch")
    target.write_text(patch)
    print(f"Generated {target}; production owner unchanged.")


if __name__ == "__main__":
    main()
