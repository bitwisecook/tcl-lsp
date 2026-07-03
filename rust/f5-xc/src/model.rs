//! Data structures modelling F5 Distributed Cloud (XC) configuration
//! objects.
//!
//! These are the intermediate representation between iRule analysis and
//! serialisation to Terraform HCL or JSON API format. Source ranges are
//! kept as the IR-native byte [`Span`](tcl_lexer::Span); the
//! [`crate::diagnostics`] layer resolves them to line/column positions.

use tcl_lexer::Span;

/// Whether an iRule construct could be translated to XC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslateStatus {
    /// Fully translated to an XC construct.
    Translated,
    /// Partially translatable — needs manual review.
    Partial,
    /// No XC equivalent.
    Untranslatable,
    /// Maps to a separate XC feature (advisory).
    Advisory,
}

/// Classification of an XC output construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XCConstructKind {
    /// An L7 route.
    Route,
    /// A service-policy rule.
    ServicePolicyRule,
    /// A header-manipulation action.
    HeaderAction,
    /// An origin-pool reference.
    OriginPool,
    /// A WAF (App Firewall) exclusion rule.
    WafExclusion,
    /// An advisory note (no concrete construct).
    Note,
}

impl TranslateStatus {
    /// Lower-case status name, as used in the JSON-API / MCP output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Translated => "translated",
            Self::Partial => "partial",
            Self::Untranslatable => "untranslatable",
            Self::Advisory => "advisory",
        }
    }
}

impl XCConstructKind {
    /// Lower-case construct-kind name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::ServicePolicyRule => "service_policy_rule",
            Self::HeaderAction => "header_action",
            Self::OriginPool => "origin_pool",
            Self::WafExclusion => "waf_exclusion",
            Self::Note => "note",
        }
    }
}

// Match criteria

/// XC path match criterion. `match_type` is `"prefix"` | `"exact"` |
/// `"regex"` | `"suffix"`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCPathMatch {
    /// Match kind.
    pub match_type: String,
    /// Match value.
    pub value: String,
    /// Whether the match is negated.
    pub invert: bool,
}

/// XC HTTP method match criterion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCMethodMatch {
    /// Matched methods (upper-cased).
    pub methods: Vec<String>,
    /// Whether the match is negated.
    pub invert: bool,
}

/// XC HTTP header match criterion. `match_type` is `"exact"` |
/// `"presence"` | `"regex"`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCHeaderMatch {
    /// Header name.
    pub name: String,
    /// Match kind.
    pub match_type: String,
    /// Match value (empty for `"presence"`).
    pub value: String,
    /// Whether the match is negated.
    pub invert: bool,
}

/// XC host/domain match criterion. `match_type` is `"exact"` | `"regex"`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCHostMatch {
    /// Match kind.
    pub match_type: String,
    /// Match value.
    pub value: String,
}

/// XC HTTP query-string match criterion. `match_type` is `"exact"` |
/// `"regex"`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCQueryMatch {
    /// Match kind.
    pub match_type: String,
    /// Match value.
    pub value: String,
}

/// XC HTTP cookie match criterion. `match_type` is `"exact"` |
/// `"presence"` | `"regex"`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCCookieMatch {
    /// Cookie name.
    pub name: String,
    /// Match kind.
    pub match_type: String,
    /// Match value (empty for `"presence"`).
    pub value: String,
    /// Whether the match is negated.
    pub invert: bool,
}

// Actions

/// An XC header-manipulation action. `operation` is `"add"` | `"replace"`
/// | `"remove"`; `target` is `"request"` | `"response"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XCHeaderAction {
    /// Header name.
    pub name: String,
    /// Header value (empty for `"remove"`).
    pub value: String,
    /// Operation.
    pub operation: String,
    /// Direction.
    pub target: String,
    /// Source range of the originating iRule command.
    pub source_range: Option<Span>,
}

/// Reference to an XC origin pool (from an iRule `pool` command).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XCOriginPoolRef {
    /// Pool name.
    pub name: String,
    /// Source range of the originating iRule command.
    pub source_range: Option<Span>,
}

/// An HTTP redirect action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XCRedirectAction {
    /// Redirect target URL.
    pub url: String,
    /// HTTP response code (default 302).
    pub response_code: i64,
    /// Source range of the originating iRule command.
    pub source_range: Option<Span>,
}

/// A direct HTTP response action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XCDirectResponseAction {
    /// HTTP status code.
    pub status_code: i64,
    /// Response body.
    pub body: String,
    /// Source range of the originating iRule command.
    pub source_range: Option<Span>,
}

// Routes

/// An L7 route in an XC HTTP load balancer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCRoute {
    /// Path match criterion.
    pub path_match: Option<XCPathMatch>,
    /// Host match criterion.
    pub host_match: Option<XCHostMatch>,
    /// Method match criterion.
    pub method_match: Option<XCMethodMatch>,
    /// Header match criteria.
    pub header_matches: Vec<XCHeaderMatch>,
    /// Query match criterion.
    pub query_match: Option<XCQueryMatch>,
    /// Cookie match criteria.
    pub cookie_matches: Vec<XCCookieMatch>,
    /// Selected origin pool.
    pub origin_pool: Option<XCOriginPoolRef>,
    /// Redirect action.
    pub redirect: Option<XCRedirectAction>,
    /// Direct-response action.
    pub direct_response: Option<XCDirectResponseAction>,
    /// Header actions on this route.
    pub header_actions: Vec<XCHeaderAction>,
    /// Source range.
    pub source_range: Option<Span>,
}

// Service policies

/// A rule within an XC service policy. `action` is `"allow"` | `"deny"` |
/// `"next_policy"`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCServicePolicyRule {
    /// Rule name.
    pub name: String,
    /// Rule action.
    pub action: String,
    /// Path match criterion.
    pub path_match: Option<XCPathMatch>,
    /// Host match criterion.
    pub host_match: Option<XCHostMatch>,
    /// Method match criterion.
    pub method_match: Option<XCMethodMatch>,
    /// Header match criteria.
    pub header_matches: Vec<XCHeaderMatch>,
    /// Query match criterion.
    pub query_match: Option<XCQueryMatch>,
    /// Cookie match criteria.
    pub cookie_matches: Vec<XCCookieMatch>,
    /// IP prefix list (data-group references).
    pub ip_prefix_list: Vec<String>,
    /// Rule description.
    pub description: String,
    /// Source range.
    pub source_range: Option<Span>,
}

/// An XC service policy (collection of rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XCServicePolicy {
    /// Policy name.
    pub name: String,
    /// Match algorithm (default `"FIRST_MATCH"`).
    pub algo: String,
    /// Rules in the policy.
    pub rules: Vec<XCServicePolicyRule>,
}

// WAF exclusion rules

/// An XC WAF exclusion rule (bypass App Firewall for matching traffic).
/// Maps from iRule `ASM::disable` guarded by path / IP conditions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XCWafExclusionRule {
    /// Rule name.
    pub name: String,
    /// Path match criterion.
    pub path_match: Option<XCPathMatch>,
    /// IP prefix set (data-group name → XC IP prefix set).
    pub ip_prefix_set: Option<String>,
    /// Rule description.
    pub description: String,
    /// Source range.
    pub source_range: Option<Span>,
}

// Origin pools

/// An XC origin pool placeholder. Actual server addresses and ports must
/// be filled in manually since iRules reference pool names but not their
/// member definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XCOriginPool {
    /// Pool name.
    pub name: String,
    /// Pool port (default 80).
    pub port: i64,
}

// Translation items (coverage tracking)

/// One translated or untranslatable construct from the iRule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationItem {
    /// Translation status.
    pub status: TranslateStatus,
    /// Construct classification.
    pub kind: XCConstructKind,
    /// The originating iRule command (rendered).
    pub irule_command: String,
    /// Source range of the originating command.
    pub irule_range: Option<Span>,
    /// XC-side description.
    pub xc_description: String,
    /// Optional note / caveat.
    pub note: String,
    /// XC-series diagnostic code (e.g. `"XC100"`).
    pub diagnostic_code: String,
}

// Top-level result

/// Complete result of translating an iRule to XC.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XCTranslationResult {
    /// Generated L7 routes.
    pub routes: Vec<XCRoute>,
    /// Generated service policies.
    pub service_policies: Vec<XCServicePolicy>,
    /// Referenced origin pools.
    pub origin_pools: Vec<XCOriginPool>,
    /// LB-level header actions.
    pub header_actions: Vec<XCHeaderAction>,
    /// WAF exclusion rules.
    pub waf_exclusion_rules: Vec<XCWafExclusionRule>,
    /// Per-construct coverage items.
    pub items: Vec<TranslationItem>,
    /// Coverage percentage (rounded to 1 dp).
    pub coverage_pct: f64,
}

impl XCTranslationResult {
    /// Number of fully translated items.
    #[must_use]
    pub fn translatable_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == TranslateStatus::Translated)
            .count()
    }

    /// Number of partially translated items.
    #[must_use]
    pub fn partial_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == TranslateStatus::Partial)
            .count()
    }

    /// Number of untranslatable items.
    #[must_use]
    pub fn untranslatable_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == TranslateStatus::Untranslatable)
            .count()
    }

    /// Number of advisory items.
    #[must_use]
    pub fn advisory_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == TranslateStatus::Advisory)
            .count()
    }
}
