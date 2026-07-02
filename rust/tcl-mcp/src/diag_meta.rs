//! Diagnostic categorisation metadata.
//!
//! Mirrors `ai/shared/diagnostics.json` — the single canonical source of the
//! code→category mapping, category ordering, convertible-code set, and
//! conversion map shared with the Python AI layer. The JSON is compiled in via
//! `include_str!` so the native MCP server and the Python one can never drift:
//! if the file moves or changes shape, this crate stops compiling.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde_json::Value;

const DIAG_JSON: &str = include_str!("../../../ai/shared/diagnostics.json");

/// Parsed diagnostic categorisation tables.
pub struct DiagMeta {
    /// Explicit code→category-key map (categories[].codes).
    code_to_category: HashMap<String, String>,
    /// `(prefix, category)` fallbacks, tried in order.
    prefix_rules: Vec<(String, String)>,
    /// Category returned when nothing else matches.
    default_category: String,
    /// `(key, label)` in canonical display order.
    pub category_order: Vec<(String, String)>,
    /// Codes the modernise/find-legacy pass can auto-convert.
    pub convertible_codes: HashSet<String>,
    /// Code→human conversion description (find-legacy).
    pub conversion_map: HashMap<String, String>,
    /// Codes in the `security` category.
    pub security_codes: HashSet<String>,
    /// Codes in the `taint` category.
    pub taint_codes: HashSet<String>,
    /// Codes in the `thread_safety` category.
    pub thread_codes: HashSet<String>,
}

fn str_set(value: Option<&Value>) -> HashSet<String> {
    value
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

impl DiagMeta {
    fn parse() -> Self {
        let data: Value = serde_json::from_str(DIAG_JSON).expect("diagnostics.json is valid JSON");

        let categories = data.get("categories").and_then(Value::as_array);
        let mut code_to_category = HashMap::new();
        let mut category_order = Vec::new();
        let mut category_codes: HashMap<String, HashSet<String>> = HashMap::new();
        if let Some(cats) = categories {
            for cat in cats {
                let key = cat.get("key").and_then(Value::as_str).unwrap_or_default().to_owned();
                let label = cat.get("label").and_then(Value::as_str).unwrap_or_default().to_owned();
                let codes = str_set(cat.get("codes"));
                for code in &codes {
                    code_to_category.insert(code.clone(), key.clone());
                }
                category_order.push((key.clone(), label));
                category_codes.insert(key, codes);
            }
        }

        let prefix_rules = data
            .get("prefix_rules")
            .and_then(Value::as_array)
            .map(|rules| {
                rules
                    .iter()
                    .filter_map(|r| {
                        Some((
                            r.get("prefix")?.as_str()?.to_owned(),
                            r.get("category")?.as_str()?.to_owned(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let conversion_map = data
            .get("conversion_map")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                    .collect()
            })
            .unwrap_or_default();

        let take = |key: &str| category_codes.get(key).cloned().unwrap_or_default();

        Self {
            security_codes: take("security"),
            taint_codes: take("taint"),
            thread_codes: take("thread_safety"),
            code_to_category,
            prefix_rules,
            default_category: data
                .get("default_category")
                .and_then(Value::as_str)
                .unwrap_or("style")
                .to_owned(),
            category_order,
            convertible_codes: str_set(data.get("convertible_codes")),
            conversion_map,
        }
    }

    /// Map a diagnostic code to its category key: explicit map first, then
    /// prefix rules, then the default. Mirrors `diagnostics.categorise`.
    #[must_use]
    pub fn categorise(&self, code: &str) -> &str {
        if let Some(cat) = self.code_to_category.get(code) {
            return cat;
        }
        for (prefix, category) in &self.prefix_rules {
            if code.starts_with(prefix) {
                return category;
            }
        }
        &self.default_category
    }
}

/// The process-wide parsed categorisation tables.
pub fn meta() -> &'static DiagMeta {
    static META: OnceLock<DiagMeta> = OnceLock::new();
    META.get_or_init(DiagMeta::parse)
}
