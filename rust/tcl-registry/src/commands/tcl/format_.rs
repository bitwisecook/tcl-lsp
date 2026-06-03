//! `format` — format a string using printf-style conversion.
use crate::prelude::*;

/// SYNC-JUN02d (#525 B-tail): constant-fold the **plain** `%s` / `%d` /
/// `%%` subset of `format` — the cases where the result is unambiguous
/// without printf flag / width / precision semantics.
///
/// `args[0]` is the format string; `args[1..]` are the values.  The
/// format string is ASCII-restricted (we index it by byte for the `%`
/// look-ahead, so a multi-byte char would be mis-sliced).  Any
/// conversion other than `%s` / `%d` / `%%` — i.e. a flag (`-`, `+`,
/// space, `0`, `#`), a width, a precision, a size modifier, or another
/// verb (`%f`, `%x`, `%c`, …) — bails, leaving the richer printf
/// matrix to a differential-pinned follow-up (same subset the
/// codegen-side `try_format_fold` is validated for).  `%d` with a
/// non-integer argument bails (Tcl would raise at runtime; we never
/// fold an error to a value).  Too few arguments for the conversions
/// bails.
fn fold_format(args: &[&str]) -> Option<String> {
    let (fmt, vals) = args.split_first()?;
    if !fmt.is_ascii() {
        return None;
    }
    let fmt = fmt.as_bytes();
    let mut out = String::new();
    let mut ai = 0;
    let mut fi = 0;
    while fi < fmt.len() {
        if fmt[fi] == b'%' && fi + 1 < fmt.len() {
            match fmt[fi + 1] {
                b's' => {
                    out.push_str(vals.get(ai)?);
                    ai += 1;
                }
                b'd' => {
                    let n: i64 = vals.get(ai)?.trim().parse().ok()?;
                    out.push_str(&n.to_string());
                    ai += 1;
                }
                b'%' => out.push('%'),
                // flags / width / precision / other verbs → defer.
                _ => return None,
            }
            fi += 2;
        } else {
            // `fmt` is ASCII, so each byte is one char.
            out.push(char::from(fmt[fi]));
            fi += 1;
        }
    }
    Some(out)
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "format",
        traits: Traits::BYTE_COMPILED | Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        const_fold: Some(fold_format),
        hover: Some(HoverSnippet::brief(
            "Format a string.",
            &["format formatString ?arg ...?"],
            "Tcl format(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_format;

    #[test]
    fn format_folds_plain_s_d_percent_subset() {
        assert_eq!(fold_format(&["hello"]).as_deref(), Some("hello"));
        assert_eq!(fold_format(&["%s", "world"]).as_deref(), Some("world"));
        assert_eq!(fold_format(&["%d", "42"]).as_deref(), Some("42"));
        assert_eq!(fold_format(&["%d", " -7 "]).as_deref(), Some("-7"));
        assert_eq!(
            fold_format(&["v=%s n=%d", "x", "3"]).as_deref(),
            Some("v=x n=3")
        );
        assert_eq!(fold_format(&["100%%"]).as_deref(), Some("100%"));
        // Flags / width / precision are deferred → bail.
        assert_eq!(fold_format(&["%05d", "7"]), None);
        assert_eq!(fold_format(&["%-10s", "x"]), None);
        // Other verbs bail.
        assert_eq!(fold_format(&["%x", "255"]), None);
        // `%d` with a non-integer arg bails (Tcl errors at runtime).
        assert_eq!(fold_format(&["%d", "hi"]), None);
        // Too few args bails.
        assert_eq!(fold_format(&["%s %s", "only"]), None);
    }
}
