//! `PortSet` value type — F5 firewall-rule port specs.
//!
//! Parses `80-82,8081`-style comma-separated ranges. Each segment is a
//! single port or a `low-high` range; `any` / `*` / `0` is the wildcard.

use super::error::ValueError;
use super::port::{parse_decimal_int, py_repr};
use std::fmt;

/// One contiguous `low..=high` port range (inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortSegment {
    /// Inclusive low bound.
    pub low: u16,
    /// Inclusive high bound.
    pub high: u16,
}

impl PortSegment {
    /// Construct a segment from arbitrary integer bounds, validating the
    /// `[0, 65535]` range and `low <= high`.
    ///
    /// # Errors
    /// Returns [`ValueError`] when out of range or `low > high`.
    pub fn new(low: i64, high: i64) -> Result<Self, ValueError> {
        if low < 0 || high > 65535 {
            return Err(ValueError(format!(
                "PortSegment: out of range ({low}-{high})"
            )));
        }
        if low > high {
            return Err(ValueError(format!(
                "PortSegment: low > high ({low}-{high})"
            )));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self {
            low: low as u16,
            high: high as u16,
        })
    }

    /// `true` when the segment is a single port.
    #[must_use]
    pub fn is_single(&self) -> bool {
        self.low == self.high
    }

    /// `true` when `port` is inside the segment.
    #[must_use]
    pub fn contains(&self, port: u16) -> bool {
        self.low <= port && port <= self.high
    }

    /// `true` when `self` and `other` share at least one port.
    #[must_use]
    pub fn overlaps(&self, other: &PortSegment) -> bool {
        !(self.high < other.low || other.high < self.low)
    }
}

impl fmt::Display for PortSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_single() {
            write!(f, "{}", self.low)
        } else {
            write!(f, "{}-{}", self.low, self.high)
        }
    }
}

/// A set of port ranges expressed as `80-82,8081`.
///
/// The internal representation is a list of disjoint [`PortSegment`]s
/// sorted by their low end so equality and intersection are well-defined.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PortSet {
    /// Normalised, disjoint, sorted segments.
    pub segments: Vec<PortSegment>,
    /// Whether the set was spelt as the wildcard.
    pub is_wildcard: bool,
}

impl PortSet {
    /// Parse a comma-separated port spec into a normalised range.
    ///
    /// `"any"` / `"*"` / `"0"` produce a wildcard. Overlapping / adjacent
    /// segments are collapsed.
    ///
    /// # Errors
    /// Returns [`ValueError`] for empty, non-integer, or empty-after-split
    /// input, and for invalid segment bounds.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("PortSet: empty input".to_owned()));
        }
        if matches!(text.to_lowercase().as_str(), "any" | "*" | "0") {
            return Ok(Self {
                segments: vec![PortSegment::new(0, 65535)?],
                is_wildcard: true,
            });
        }
        let mut raw: Vec<PortSegment> = Vec::new();
        for chunk in text.split(',') {
            let piece = chunk.trim();
            if piece.is_empty() {
                continue;
            }
            if piece.contains('-') {
                // Split on the FIRST '-'.
                let (lo_text, hi_text) = piece.split_once('-').unwrap_or((piece, ""));
                let (Some(lo), Some(hi)) = (parse_int(lo_text), parse_int(hi_text)) else {
                    return Err(ValueError(format!(
                        "PortSet: non-integer in {}",
                        py_repr(piece)
                    )));
                };
                raw.push(PortSegment::new(lo, hi)?);
            } else {
                let Some(n) = parse_int(piece) else {
                    return Err(ValueError(format!(
                        "PortSet: non-integer port {}",
                        py_repr(piece)
                    )));
                };
                raw.push(PortSegment::new(n, n)?);
            }
        }
        if raw.is_empty() {
            return Err(ValueError("PortSet: no usable segments".to_owned()));
        }
        Ok(Self {
            segments: collapse_segments(&raw),
            is_wildcard: false,
        })
    }

    /// Like [`PortSet::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// `true` when `port` is inside any segment.
    #[must_use]
    pub fn contains(&self, port: u16) -> bool {
        self.segments.iter().any(|s| s.contains(port))
    }

    /// `true` when `self` and `other` share at least one port.
    #[must_use]
    pub fn overlaps(&self, other: &PortSet) -> bool {
        self.segments
            .iter()
            .any(|s| other.segments.iter().any(|o| s.overlaps(o)))
    }

    /// Total number of ports across every segment.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.segments
            .iter()
            .map(|s| u32::from(s.high) - u32::from(s.low) + 1)
            .sum()
    }
}

impl fmt::Display for PortSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_wildcard {
            return f.write_str("0-65535");
        }
        let joined: Vec<String> = self.segments.iter().map(ToString::to_string).collect();
        f.write_str(&joined.join(","))
    }
}

/// Parse a port token like an integer: strip surrounding whitespace, accept
/// an optional sign and ASCII digits.
fn parse_int(text: &str) -> Option<i64> {
    parse_decimal_int(text)
}

/// Merge overlapping / adjacent segments into a canonical list, sorted by
/// `(low, high)`.
fn collapse_segments(raw: &[PortSegment]) -> Vec<PortSegment> {
    if raw.is_empty() {
        return Vec::new();
    }
    let mut sorted = raw.to_vec();
    sorted.sort_by_key(|s| (s.low, s.high));
    let mut out: Vec<PortSegment> = vec![sorted[0]];
    for seg in &sorted[1..] {
        let last = *out.last().unwrap();
        // ``+1`` covers adjacency ([1,5] then [6,10] -> [1,10]).
        if u32::from(seg.low) <= u32::from(last.high) + 1 {
            out.pop();
            out.push(PortSegment {
                low: last.low,
                high: last.high.max(seg.high),
            });
        } else {
            out.push(*seg);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spec() {
        let p = PortSet::parse("80-82,8081").unwrap();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.to_string(), "80-82,8081");
        assert_eq!(p.count(), 4);
        assert!(p.contains(81));
        assert!(!p.contains(90));
    }

    #[test]
    fn wildcard() {
        for tok in ["any", "*", "0", "ANY"] {
            let p = PortSet::parse(tok).unwrap();
            assert!(p.is_wildcard);
            assert_eq!(p.to_string(), "0-65535");
        }
    }

    #[test]
    fn collapses_adjacent() {
        let p = PortSet::parse("1-5,6-10").unwrap();
        assert_eq!(p.segments.len(), 1);
        assert_eq!(p.to_string(), "1-10");
        let p = PortSet::parse("1-7,5-10").unwrap();
        assert_eq!(p.to_string(), "1-10");
    }

    #[test]
    fn overlaps_works() {
        let a = PortSet::parse("80-90").unwrap();
        let b = PortSet::parse("85,100").unwrap();
        assert!(a.overlaps(&b));
        let c = PortSet::parse("200-300").unwrap();
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(PortSet::parse("").unwrap_err().0, "PortSet: empty input");
        assert_eq!(
            PortSet::parse("abc").unwrap_err().0,
            "PortSet: non-integer port 'abc'"
        );
        assert!(PortSet::parse("70000").is_err());
    }
}
