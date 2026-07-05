# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""BIG-IP object registry catalogue."""

from .data import (
    HEADER_KIND_MAP,
    KIND_SPECS,
    OBJECT_KIND_SPECS,
    PROPERTY_NAMES_BY_TYPE,
    PROPERTY_REFERENCE_SPECS,
    PROPERTY_SPECS_BY_TYPE,
    candidate_registry_kinds_for_display,
    list_operator_for,
    property_names_for,
    property_spec_for,
)
from .models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from .properties import (
    ObjectKeySpec,
    ObjectSpec,
    PropertySpec,
)
from .references import (
    iter_object_references,
    references_via_spec,
)
from .value_specs import (
    AddressSpec,
    BoolSpec,
    CertKeyChainSpec,
    DataGroupRecordSpec,
    DestinationSpec,
    Diagnostic,
    EnumSpec,
    FirewallRuleSpec,
    GtmRegionMemberSpec,
    IntSpec,
    ListSpec,
    LtmPolicyActionSpec,
    LtmPolicyConditionSpec,
    LtmPolicyRuleSpec,
    MonitorExpressionSpec,
    NatRuleSpec,
    NetworkSpec,
    ObjectRefSpec,
    ParseContext,
    ParsedValue,
    PersistenceAttachmentSpec,
    PortSpec,
    ProfileAttachmentSpec,
    ProjectionContext,
    Reference,
    ReferenceContext,
    RenderContext,
    SnatModeSpec,
    SourceRange,
    StringSpec,
    ValueSpec,
)

__all__ = [
    # Phase-1 new shape
    "ObjectSpec",
    "ObjectKeySpec",
    "PropertySpec",
    "ValueSpec",
    "ParsedValue",
    "ParseContext",
    "ProjectionContext",
    "RenderContext",
    "ReferenceContext",
    "Reference",
    "SourceRange",
    "Diagnostic",
    "StringSpec",
    "IntSpec",
    "BoolSpec",
    "EnumSpec",
    "ObjectRefSpec",
    "ListSpec",
    "CertKeyChainSpec",
    "DataGroupRecordSpec",
    "DestinationSpec",
    "FirewallRuleSpec",
    "GtmRegionMemberSpec",
    "LtmPolicyActionSpec",
    "LtmPolicyConditionSpec",
    "LtmPolicyRuleSpec",
    "MonitorExpressionSpec",
    "NatRuleSpec",
    "NetworkSpec",
    "AddressSpec",
    "PersistenceAttachmentSpec",
    "PortSpec",
    "ProfileAttachmentSpec",
    "SnatModeSpec",
    # Phase 5 reference dispatch
    "iter_object_references",
    "references_via_spec",
    # Legacy shape (still in use across every spec file under specs/)
    "BigipObjectSpec",
    "BigipObjectKindSpec",
    "BigipPropertySpec",
    "KIND_SPECS",
    "OBJECT_KIND_SPECS",
    "PROPERTY_REFERENCE_SPECS",
    "PROPERTY_NAMES_BY_TYPE",
    "PROPERTY_SPECS_BY_TYPE",
    "HEADER_KIND_MAP",
    "candidate_registry_kinds_for_display",
    "list_operator_for",
    "property_names_for",
    "property_spec_for",
]
