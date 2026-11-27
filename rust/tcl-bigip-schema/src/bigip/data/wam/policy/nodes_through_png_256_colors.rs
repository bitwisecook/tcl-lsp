// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Properties `nodes` to `png-256-colors` of `wam policy`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "nodes",
        value_type: ValueKind::List,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "always-proxy",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-compression",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-compression-ows",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-concatenation",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-concatenation-sets",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-inlining",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-inlining-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-reorder",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-reorder-cache-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-reorder-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch-domain-lists",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch-https-automatic",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch-https-enable",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-ibr",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-css-inlining-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-force",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-image-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-js-inlining-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-max-num-urls",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-min-client-expiry",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-image-inlining",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-image-inlining-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-image-inlining-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-intelligent-client-cache",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-inlining",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-inlining-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-reorder",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-reorder-cache-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-reorder-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-minification",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-multiconnect",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-on-proxies",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-pdf-linearization",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-complete-only",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-first-hit",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-mode",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["memory-and-disk", "memory-only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-priority",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["high", "low", "medium"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-stand-in-period",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "code",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "coherency",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["blade", "cluster"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-progressive-encoding",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-quality",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-quality-is-relative",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-sampling",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["1x1", "1x2", "2x1", "2x2", "preserve"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-strip-exif",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["if-safe", "make-safe", "no", "yes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-strip-keeps-copyright",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpegxr-quality",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-cache-control-extensions",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-cache-max-age",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-ows",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-ows-values",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-request",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-request-values",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-http-heuristic",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-insert-no-cache",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-preserve-response",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-preserve-response-values",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-response-max-age",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-response-s-maxage",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-stand-in-codes",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-use-heuristic",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "matching",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "object-max-size",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["from-profile"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "object-min-size",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["from-profile"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "optimize-for-client",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "optimize-image",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                allow_none: true,
                enum_values: &["none", "to-gif", "to-jpeg", "to-png", "to-tiff"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "png-256-colors",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-queueing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-codes-cached",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "variation",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-acceleration-ad-policy",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-ad-frequency",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-fast-start",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-insert-ad",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-max-bitrate",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-preroll-ad",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "viewstate-cache",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "viewstate-cache-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "viewstate-tag",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "webp-quality",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "always-proxy",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-service",
        value_type: ValueKind::String,
        in_sections: &["nodes"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-alias",
        value_type: ValueKind::String,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-direction",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["left-to-right", "right-to-left"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-name",
        value_type: ValueKind::String,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-ordinal",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-compression",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-compression-ows",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-concatenation",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-concatenation-sets",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-css-inlining",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-css-inlining-urls",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-css-reorder",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-css-reorder-cache-size",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-css-reorder-urls",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-dns-prefetch",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-dns-prefetch-domain-lists",
        value_type: ValueKind::List,
        in_sections: &["nodes"],
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-dns-prefetch-https-automatic",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-dns-prefetch-https-enable",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-ibr",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-icc-css-inlining-max-size",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-icc-force",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-icc-image-max-size",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-icc-js-inlining-max-size",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-icc-max-num-urls",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-icc-min-client-expiry",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-image-inlining",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-image-inlining-max-size",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-image-inlining-urls",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-intelligent-client-cache",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-js-inlining",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-js-inlining-urls",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-js-reorder",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-js-reorder-cache-size",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-js-reorder-urls",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-minification",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-multiconnect",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-on-proxies",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "assembly-pdf-linearization",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cache-complete-only",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cache-first-hit",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cache-mode",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["memory-and-disk", "memory-only"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cache-priority",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["high", "low", "medium"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cache-stand-in-period",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-empty",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-missing",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "code",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "coherency",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["blade", "cluster"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "defaults-from",
        value_type: ValueKind::Reference,
        in_sections: &["nodes"],
        references: &["wam_policy"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "invert-match",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jpeg-progressive-encoding",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jpeg-quality",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jpeg-quality-is-relative",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jpeg-sampling",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["1x1", "1x2", "2x1", "2x2", "preserve"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jpeg-strip-exif",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["if-safe", "make-safe", "no", "yes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jpeg-strip-keeps-copyright",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jpegxr-quality",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-cache-control-extensions",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-cache-max-age",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-honor-ows",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-honor-ows-values",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-honor-request",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-honor-request-values",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-http-heuristic",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-insert-no-cache",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-preserve-response",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-preserve-response-values",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-response-max-age",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-response-s-maxage",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-stand-in-codes",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lifetime-use-heuristic",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "matching",
        value_type: ValueKind::List,
        in_sections: &["nodes"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["nodes", "matching"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-service",
        value_type: ValueKind::String,
        in_sections: &["nodes", "matching"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-alias",
        value_type: ValueKind::String,
        in_sections: &["nodes", "matching"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-direction",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "matching"],
        enum_values: &["left-to-right", "right-to-left"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-name",
        value_type: ValueKind::String,
        in_sections: &["nodes", "matching"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-ordinal",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes", "matching"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        in_sections: &["nodes", "matching"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "value-case-sensitive",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "matching"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "values",
        value_type: ValueKind::List,
        in_sections: &["nodes", "matching"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching", "values"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-service",
        value_type: ValueKind::String,
        in_sections: &["nodes", "matching", "values"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-empty",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "matching", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-missing",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "matching", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "invert-match",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "matching", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "object-max-size",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["from-profile"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "object-min-size",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["from-profile"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "optimize-for-client",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "optimize-image",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        allow_none: true,
        enum_values: &["none", "to-gif", "to-jpeg", "to-png", "to-tiff"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "options",
        value_type: ValueKind::List,
        in_sections: &["nodes"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "order",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "png-256-colors",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
];
