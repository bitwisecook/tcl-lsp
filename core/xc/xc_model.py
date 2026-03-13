"""Dataclasses modelling F5 Distributed Cloud (XC) configuration objects.

These are the intermediate representation between iRule analysis and
serialisation to Terraform HCL or JSON API format.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto

from ..analysis.semantic_model import Range

# Enums


class TranslateStatus(Enum):
    """Whether an iRule construct could be translated to XC."""

    TRANSLATED = auto()
    PARTIAL = auto()
    UNTRANSLATABLE = auto()
    ADVISORY = auto()


class XCConstructKind(Enum):
    """Classification of an XC output construct."""

    ROUTE = auto()
    SERVICE_POLICY_RULE = auto()
    HEADER_ACTION = auto()
    ORIGIN_POOL = auto()
    WAF_EXCLUSION = auto()
    NOTE = auto()


# Match criteria


@dataclass(frozen=True, slots=True)
class XCPathMatch:
    """XC path match criterion."""

    match_type: str  # "prefix" | "exact" | "regex" | "suffix"
    value: str
    invert: bool = False


@dataclass(frozen=True, slots=True)
class XCMethodMatch:
    """XC HTTP method match criterion."""

    methods: tuple[str, ...]
    invert: bool = False


@dataclass(frozen=True, slots=True)
class XCHeaderMatch:
    """XC HTTP header match criterion."""

    name: str
    match_type: str  # "exact" | "presence" | "regex"
    value: str = ""
    invert: bool = False


@dataclass(frozen=True, slots=True)
class XCHostMatch:
    """XC host/domain match criterion."""

    match_type: str  # "exact" | "regex"
    value: str


@dataclass(frozen=True, slots=True)
class XCQueryMatch:
    """XC HTTP query string match criterion."""

    match_type: str  # "exact" | "regex"
    value: str


@dataclass(frozen=True, slots=True)
class XCCookieMatch:
    """XC HTTP cookie match criterion."""

    name: str
    match_type: str  # "exact" | "presence" | "regex"
    value: str = ""
    invert: bool = False


# Actions


@dataclass(frozen=True, slots=True)
class XCHeaderAction:
    """An XC header manipulation action."""

    name: str
    value: str
    operation: str  # "add" | "replace" | "remove"
    target: str  # "request" | "response"
    source_range: Range | None = None


@dataclass(frozen=True, slots=True)
class XCOriginPoolRef:
    """Reference to an XC origin pool (from an iRule ``pool`` command)."""

    name: str
    source_range: Range | None = None


@dataclass(frozen=True, slots=True)
class XCRedirectAction:
    """An HTTP redirect action."""

    url: str
    response_code: int = 302
    source_range: Range | None = None


@dataclass(frozen=True, slots=True)
class XCDirectResponseAction:
    """A direct HTTP response action."""

    status_code: int
    body: str = ""
    source_range: Range | None = None


# Routes


@dataclass(frozen=True, slots=True)
class XCRoute:
    """An L7 route in an XC HTTP load balancer."""

    path_match: XCPathMatch | None = None
    host_match: XCHostMatch | None = None
    method_match: XCMethodMatch | None = None
    header_matches: tuple[XCHeaderMatch, ...] = ()
    query_match: XCQueryMatch | None = None
    cookie_matches: tuple[XCCookieMatch, ...] = ()
    origin_pool: XCOriginPoolRef | None = None
    redirect: XCRedirectAction | None = None
    direct_response: XCDirectResponseAction | None = None
    header_actions: tuple[XCHeaderAction, ...] = ()
    source_range: Range | None = None


# Service policies


@dataclass(frozen=True, slots=True)
class XCServicePolicyRule:
    """A rule within an XC service policy."""

    name: str
    action: str  # "allow" | "deny" | "next_policy"
    path_match: XCPathMatch | None = None
    host_match: XCHostMatch | None = None
    method_match: XCMethodMatch | None = None
    header_matches: tuple[XCHeaderMatch, ...] = ()
    query_match: XCQueryMatch | None = None
    cookie_matches: tuple[XCCookieMatch, ...] = ()
    ip_prefix_list: tuple[str, ...] = ()
    description: str = ""
    source_range: Range | None = None


@dataclass(frozen=True, slots=True)
class XCServicePolicy:
    """An XC service policy (collection of rules)."""

    name: str
    algo: str = "FIRST_MATCH"
    rules: tuple[XCServicePolicyRule, ...] = ()


# WAF exclusion rules


@dataclass(frozen=True, slots=True)
class XCWafExclusionRule:
    """An XC WAF exclusion rule (bypass App Firewall for matching traffic).

    Maps from iRule ``ASM::disable`` guarded by path/IP conditions.
    """

    name: str
    path_match: XCPathMatch | None = None
    ip_prefix_set: str | None = None  # data-group name → XC IP prefix set
    description: str = ""
    source_range: Range | None = None


# Origin pools


@dataclass(frozen=True, slots=True)
class XCOriginPool:
    """An XC origin pool placeholder.

    Actual server addresses and ports must be filled in manually since
    iRules reference pool names but not their member definitions.
    """

    name: str
    port: int = 80


# Translation items (coverage tracking)


@dataclass(frozen=True, slots=True)
class TranslationItem:
    """One translated or untranslatable construct from the iRule."""

    status: TranslateStatus
    kind: XCConstructKind
    irule_command: str
    irule_range: Range | None
    xc_description: str
    note: str = ""
    diagnostic_code: str = ""


# Top-level result


@dataclass(frozen=True, slots=True)
class XCTranslationResult:
    """Complete result of translating an iRule to XC."""

    routes: tuple[XCRoute, ...] = ()
    service_policies: tuple[XCServicePolicy, ...] = ()
    origin_pools: tuple[XCOriginPool, ...] = ()
    header_actions: tuple[XCHeaderAction, ...] = ()  # LB-level
    waf_exclusion_rules: tuple[XCWafExclusionRule, ...] = ()
    items: tuple[TranslationItem, ...] = ()
    coverage_pct: float = 0.0

    @property
    def translatable_count(self) -> int:
        return sum(1 for i in self.items if i.status == TranslateStatus.TRANSLATED)

    @property
    def partial_count(self) -> int:
        return sum(1 for i in self.items if i.status == TranslateStatus.PARTIAL)

    @property
    def untranslatable_count(self) -> int:
        return sum(1 for i in self.items if i.status == TranslateStatus.UNTRANSLATABLE)

    @property
    def advisory_count(self) -> int:
        return sum(1 for i in self.items if i.status == TranslateStatus.ADVISORY)
