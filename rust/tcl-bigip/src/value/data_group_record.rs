//! Typed BIG-IP data-group record value.

use std::fmt;

/// One record inside a data-group's records list.
///
/// `key` is the lookup name. `data` is the per-record value (often an IP,
/// label, or arbitrary string); empty when the record is bare. `raw`
/// preserves the original body.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DataGroupRecord {
    /// Record lookup key.
    pub key: String,
    /// Record data value.
    pub data: String,
    /// Original brace-body text (stripped).
    pub raw: String,
}

impl DataGroupRecord {
    /// Build a record from the keyed-block parser output.
    #[must_use]
    pub fn from_raw(key: &str, body: &str) -> Self {
        let mut data = String::new();
        let tokens: Vec<&str> = body.split_whitespace().collect();
        for (i, tok) in tokens.iter().enumerate() {
            if *tok == "data" && i + 1 < tokens.len() {
                // Last-wins, matching the Python loop with no break.
                tokens[i + 1].clone_into(&mut data);
            }
        }
        Self {
            key: key.to_owned(),
            data,
            raw: body.trim().to_owned(),
        }
    }
}

impl fmt::Display for DataGroupRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.raw.is_empty() {
            return write!(f, "{} {{ {} }}", self.key, self.raw);
        }
        if !self.data.is_empty() {
            return write!(f, "{} {{ data {} }}", self.key, self.data);
        }
        write!(f, "{} {{ }}", self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_raw_with_data() {
        let r = DataGroupRecord::from_raw("host1.example.com", "data 10.0.0.1");
        assert_eq!(r.data, "10.0.0.1");
        assert_eq!(r.to_string(), "host1.example.com { data 10.0.0.1 }");
    }

    #[test]
    fn bare_record() {
        let r = DataGroupRecord::from_raw("prod-allowed", "");
        assert_eq!(r.data, "");
        assert_eq!(r.to_string(), "prod-allowed { }");
    }

    #[test]
    fn structured_render() {
        let r = DataGroupRecord {
            key: "k".to_owned(),
            data: "v".to_owned(),
            raw: String::new(),
        };
        assert_eq!(r.to_string(), "k { data v }");
    }
}
