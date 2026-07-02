//! Tcl 9.1 list-returning math commands: `divmod` / `frexp` / `modf` /
//! `remquo`.
//!
//! Each is a pure, deterministic numeric operation that returns a two-element
//! list.  Verified against C Tcl 9.1b0 `generic/tclBasic.c` (the built-in
//! command table: `DivModObjCmd` / `FracExpObjCmd` / `ModFObjCmd` /
//! `RemQuoObjCmd`, all `CMD_IS_SAFE`) and `doc/{divmod,frexp,modf,remquo}.n`.

use crate::prelude::*;

/// Build a pure 9.1 math command returning a two-element list.  `arity` is the
/// exact argument count after the command word (`divmod x y` ⇒ 2, `frexp
/// value` ⇒ 1).
fn make(
    name: &'static str,
    args: u16,
    summary: &'static str,
    synopsis: &'static [&'static str],
) -> CommandSpec {
    CommandSpec {
        name,
        // Deterministic and side-effect-free ⇒ constant-foldable / CSE-able,
        // like `lseq`.
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::TCL91),
        arity: Arity::exact(args),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(summary, synopsis, "Tcl man page")),
        ..CommandSpec::DEFAULT
    }
}

/// `divmod x y` — integer quotient and remainder as a two-element list.
pub fn divmod() -> CommandSpec {
    make(
        "divmod",
        2,
        "Divide an integer by another and report quotient and remainder (Tcl 9.1).",
        &["divmod x y"],
    )
}

/// `frexp value` — fraction and exponent parts of a float as a list.
pub fn frexp() -> CommandSpec {
    make(
        "frexp",
        1,
        "Split a floating-point number into fraction and exponent parts (Tcl 9.1).",
        &["frexp value"],
    )
}

/// `modf value` — integer and fraction parts of a float as a list.
pub fn modf() -> CommandSpec {
    make(
        "modf",
        1,
        "Split a floating-point number into integer and fraction parts (Tcl 9.1).",
        &["modf value"],
    )
}

/// `remquo x y` — floating-point remainder and quadrant determinant as a list.
pub fn remquo() -> CommandSpec {
    make(
        "remquo",
        2,
        "Compute floating-point remainder and quadrant determinant (Tcl 9.1).",
        &["remquo x y"],
    )
}
