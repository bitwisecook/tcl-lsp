// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native parse-error source extents, independent of execution permission.

/// C8.4 compiler context from the original remaining command and parser term.
/// A failed `Tcl_ParseCommand` spans to the supplied source end. Compilation
/// removes the terminator only when it is the final byte of that span.
/// The caller must supply independently proved parser geometry; this function
/// neither parses source nor attests a native parse failure.
#[must_use]
pub fn c84_compilation_command_extent(
    source: &[u8],
    command_start: usize,
    term: usize,
) -> Option<&[u8]> {
    if command_start > term || term >= source.len() {
        return None;
    }
    let end = if term == source.len() - 1 {
        term
    } else {
        source.len()
    };
    source.get(command_start..end)
}
