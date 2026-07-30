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

//! Registry spec dumper.
//!
//! Emits one JSON object per command spec (JSONL) for a given dialect
//! group, using a normalised schema shared with the reference dumper.
//! Used by the registry audit to diff against the reference dumper.
//!
//! Usage: `cargo run -q --example dump_specs -- <group>`
//! where <group> is one of:
//!   tcl stdlib tcllib irules iapps tk expect
//!   sdc-base synopsys cadence xilinx quartus mentor

use std::fmt::Write as _;

use tcl_dialect::DialectSet;
use tcl_registry::commands;
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::ProfileRegistry;
use tcl_registry::spec::CommandSpec;

fn group_specs(group: &str) -> Vec<CommandSpec> {
    match group {
        "tcl" => commands::tcl::tcl_command_specs(),
        "stdlib" => commands::stdlib::stdlib_command_specs(),
        "tcllib" => commands::tcllib::tcllib_command_specs(),
        "irules" => commands::irules::irules_command_specs(),
        "iapps" => commands::iapps::iapps_command_specs(),
        "tk" => commands::tk::tk_command_specs(),
        "expect" => commands::expect::expect_command_specs(),
        "sdc-base" => commands::sdc_base::sdc_base_command_specs(),
        "synopsys" => commands::eda_synopsys::eda_synopsys_command_specs(),
        "cadence" => commands::eda_cadence::eda_cadence_command_specs(),
        "xilinx" => commands::eda_xilinx::eda_xilinx_command_specs(),
        "quartus" => commands::eda_quartus::eda_quartus_command_specs(),
        "mentor" => commands::eda_mentor::eda_mentor_command_specs(),
        other => {
            eprintln!("unknown group: {other}");
            std::process::exit(2);
        }
    }
}

const DIALECT_TAGS: &[(DialectSet, &str)] = &[
    (DialectSet::TCL84, "tcl8.4"),
    (DialectSet::TCL85, "tcl8.5"),
    (DialectSet::TCL86, "tcl8.6"),
    (DialectSet::TCL90, "tcl9.0"),
    (DialectSet::IRULES, "f5-irules"),
    (DialectSet::IAPPS, "f5-iapps"),
    (DialectSet::TK, "tk"),
    (DialectSet::EXPECT, "expect"),
    // The EDA shells are packaged base-version dialects (no vendor bit); their
    // commands tag under the base Tcl version (eda-library-packages.md).
];

fn dialect_tags(d: DialectSet) -> Vec<&'static str> {
    DIALECT_TAGS
        .iter()
        .filter(|(flag, _)| d.contains(*flag))
        .map(|(_, tag)| *tag)
        .collect()
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

fn json_str_list(items: &[&str]) -> String {
    let mut v: Vec<&str> = items.to_vec();
    v.sort_unstable();
    let parts: Vec<String> = v.iter().map(|s| json_str(s)).collect();
    format!("[{}]", parts.join(","))
}

use tcl_registry::side_effects::SideEffect;
use tcl_registry::spec::SubCommand;
use tcl_registry::traits::Traits;

/// Schema field name -> Rust trait, for the content audit's `bools`.
const BOOL_TRAITS: &[(&str, Traits)] = &[
    ("creates_dynamic_barrier", Traits::CREATES_DYNAMIC_BARRIER),
    ("has_loop_body", Traits::HAS_LOOP_BODY),
    ("never_inline_body", Traits::NEVER_INLINE_BODY),
    ("evaluates_code", Traits::EVALUATES_CODE),
    ("performs_substitution", Traits::PERFORMS_SUBSTITUTION),
    ("opens_channel", Traits::OPENS_CHANNEL),
    ("sources_file", Traits::SOURCES_FILE),
    ("has_switch_body", Traits::HAS_SWITCH_BODY),
    (
        "has_string_list_confusion_risk",
        Traits::STRING_LIST_CONFUSION,
    ),
    ("configures_channel", Traits::CONFIGURES_CHANNEL),
    ("has_interp_eval", Traits::HAS_INTERP_EVAL),
    ("has_destructive_ops", Traits::HAS_DESTRUCTIVE_OPS),
    ("is_irules_event_handler", Traits::IS_EVENT_HANDLER),
    (
        "is_unnormalized_http_getter",
        Traits::UNNORMALISED_HTTP_GETTER,
    ),
    ("pure", Traits::PURE),
    ("cse_candidate", Traits::CSE_CANDIDATE),
    ("diagram_action", Traits::DIAGRAM_ACTION),
    ("is_control_flow", Traits::CONTROL_FLOW),
    ("needs_start_cmd", Traits::NEEDS_START_CMD),
    ("terminates_block", Traits::TERMINATES_BLOCK),
    ("is_oo_metaclass", Traits::IS_OO_METACLASS),
    ("irules_top_level_only", Traits::IRULES_TOP_LEVEL_ONLY),
    ("is_unescape_command", Traits::IS_UNESCAPE),
    ("reads_variable_before_write", Traits::READS_BEFORE_WRITE),
    ("has_boolean_condition", Traits::HAS_BOOLEAN_COND),
    ("loop_list_header", Traits::LOOP_LIST_HEADER),
    ("creates_scope_alias", Traits::CREATES_SCOPE_ALIAS),
    ("returns_path", Traits::RETURNS_PATH),
    ("pure_evaluation", Traits::PURE_EVALUATION),
    ("destroys_variable", Traits::DESTROYS_VARIABLE),
    ("is_language_keyword", Traits::LANGUAGE_KEYWORD),
    ("produces_canonical_list", Traits::PRODUCES_CANONICAL_LIST),
    ("builds_command_prefix", Traits::BUILDS_COMMAND_PREFIX),
    ("wraps_command_prefix", Traits::WRAPS_COMMAND_PREFIX),
    ("is_side_switch", Traits::IS_SIDE_SWITCH),
    ("defines_procedure", Traits::DEFINES_PROCEDURE),
    ("unsafe", Traits::UNSAFE),
    ("password_option_command", Traits::PASSWORD_OPTION),
    ("taint_sink", Traits::TAINT_SINK),
];

fn opt_str_or_null(v: Option<&str>) -> String {
    v.map_or_else(|| "null".to_string(), json_str)
}

fn colours_json(c: Option<tcl_registry::TaintColour>) -> String {
    match c {
        None => "null".to_string(),
        Some(c) => {
            let mut names: Vec<&str> = c.iter_names().map(|(n, _)| n).collect();
            names.sort_unstable();
            json_str_list(&names)
        }
    }
}

fn side_effects_json(ses: &[SideEffect]) -> String {
    let mut v: Vec<String> = ses
        .iter()
        .map(|s| {
            format!(
                "{:?}|{}|{}|{:?}",
                s.target,
                i32::from(s.reads),
                i32::from(s.writes),
                s.connection_side
            )
        })
        .collect();
    v.sort_unstable();
    let parts: Vec<String> = v.iter().map(|s| json_str(s)).collect();
    format!("[{}]", parts.join(","))
}

fn options_json(opts: &[tcl_registry::hover::OptionSpec]) -> String {
    let mut v: Vec<String> = opts
        .iter()
        .map(|o| {
            format!(
                "{}|{}|{}",
                o.name,
                i32::from(o.takes_value()),
                o.value_hint()
            )
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    let parts: Vec<String> = v.iter().map(|s| json_str(s)).collect();
    format!("[{}]", parts.join(","))
}

fn arg_types_json(at: &[(u8, tcl_registry::hooks::ArgTypeHint)]) -> String {
    let mut v: Vec<(u8, &tcl_registry::hooks::ArgTypeHint)> =
        at.iter().map(|(i, h)| (*i, h)).collect();
    v.sort_by_key(|(i, _)| *i);
    let parts: Vec<String> = v
        .iter()
        .map(|(i, h)| {
            let t = h
                .expected
                .map_or_else(|| "null".to_string(), |t| json_str(&format!("{t:?}")));
            format!("\"{i}\":{t}")
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn arg_roles_json(ar: &[(u8, tcl_registry::ArgRole)]) -> String {
    use std::collections::BTreeMap;
    let mut m: BTreeMap<u8, Vec<String>> = BTreeMap::new();
    for (i, r) in ar {
        m.entry(*i).or_default().push(format!("{r:?}"));
    }
    let parts: Vec<String> = m
        .iter()
        .map(|(i, rs)| {
            let mut rs = rs.clone();
            rs.sort_unstable();
            let arr: Vec<String> = rs.iter().map(|s| json_str(s)).collect();
            format!("\"{i}\":[{}]", arr.join(","))
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn arg_values_json(av: &[(u8, &[tcl_registry::ArgValue])]) -> String {
    let mut v: Vec<(u8, &[tcl_registry::ArgValue])> = av.to_vec();
    v.sort_by_key(|(i, _)| *i);
    let parts: Vec<String> = v
        .iter()
        .map(|(i, vals)| {
            let mut names: Vec<&str> = vals.iter().map(|a| a.value).collect();
            names.sort_unstable();
            format!("\"{i}\":{}", json_str_list(&names))
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn sub_record_json(sub: &SubCommand) -> String {
    let ret = sub
        .return_type
        .map_or_else(|| "null".to_string(), |t| json_str(&format!("{t:?}")));
    let ist = sub
        .inferred_storage_type
        .map_or_else(|| "null".to_string(), |t| json_str(&format!("{t:?}")));
    format!(
        "{{\"return_type\":{ret},\"pure\":{},\"mutator\":{},\"destructive\":{},\"returns_path\":{},\"is_unescape\":{},\"loop_list_header\":{},\"creates_scope_alias\":{},\"options\":{},\"arg_types\":{},\"arg_roles\":{},\"arg_values\":{},\"n_forms\":{},\"side_effects\":{},\"safe_on_uninit\":{},\"credential_arg\":{},\"sensitive_headers\":{},\"taint_transform\":{},\"taint_output_sink\":{},\"cfg_rewrite_name\":{},\"inferred_storage_type\":{ist}}}",
        sub.pure,
        sub.mutator,
        sub.destructive,
        sub.returns_path,
        sub.is_unescape,
        sub.loop_list_header,
        sub.creates_scope_alias,
        options_json(sub.options),
        arg_types_json(sub.arg_types),
        arg_roles_json(sub.arg_roles),
        arg_values_json(sub.arg_values),
        sub.subcommand_forms.len(),
        side_effects_json(sub.side_effects),
        sub.safe_on_uninit.is_some(),
        sub.credential_arg
            .map_or_else(|| "null".to_string(), |n| n.to_string()),
        json_str_list(sub.sensitive_headers),
        colours_json(sub.taint_transform),
        opt_str_or_null(sub.taint_output_sink),
        opt_str_or_null(sub.cfg_rewrite_name),
    )
}

/// Full-field signature for one `BigIP` property (recursive over blocks).
fn bigip_prop_sig(p: &tcl_registry::bigip::BigipPropertySpec) -> String {
    let mut secs: Vec<&str> = p.in_sections.to_vec();
    secs.sort_unstable();
    let mut enums: Vec<&str> = p.enum_values.to_vec();
    enums.sort_unstable();
    let mut refs: Vec<&str> = p.references.to_vec();
    refs.sort_unstable();
    let mut flags: Vec<&str> = p.usage_flags.to_vec();
    flags.sort_unstable();
    let mut ops: Vec<&str> = p.list_operators.to_vec();
    ops.sort_unstable();
    let shape = p
        .shape_kind
        .map_or("", tcl_registry::bigip::ValueKind::as_str);
    let numfmt = |v: Option<f64>| -> String {
        match v {
            None => String::new(),
            // Guarded by `x.fract() == 0.0`: the value is integral, so the
            // `as i64` cast is exact for the small spec numbers we dump here.
            #[allow(clippy::cast_possible_truncation)]
            Some(x) if x.fract() == 0.0 => format!("{}", x as i64),
            Some(x) => format!("{x}"),
        }
    };
    let block: Vec<String> = {
        let mut b: Vec<String> = p.block.iter().map(bigip_prop_sig).collect();
        b.sort_unstable();
        b
    };
    format!(
        "{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~[{}]",
        p.name,
        p.value_type.as_str(),
        i32::from(p.required),
        i32::from(p.repeated),
        i32::from(p.allow_none),
        secs.join(","),
        numfmt(p.min_value),
        numfmt(p.max_value),
        p.pattern,
        refs.join(","),
        p.description,
        shape,
        p.default.unwrap_or(""),
        enums.join(","),
        flags.join(","),
        ops.join(","),
        block.join(";"),
    )
}

fn deep_dump(group: &str) {
    let specs = group_specs(group);
    for spec in &specs {
        let snippet = spec.hover.is_some_and(|h| !h.snippet.is_empty());
        let mut subs: Vec<(&str, String)> = spec
            .subcommands
            .iter()
            .map(|s| (s.name, sub_record_json(s)))
            .collect();
        subs.sort_by_key(|(n, _)| *n);
        let subs_json = {
            let parts: Vec<String> = subs
                .iter()
                .map(|(n, r)| format!("{}:{}", json_str(n), r))
                .collect();
            format!("{{{}}}", parts.join(","))
        };
        let bools: Vec<String> = BOOL_TRAITS
            .iter()
            .map(|(n, t)| format!("{}:{}", json_str(n), spec.traits.contains(*t)))
            .chain(std::iter::once(format!(
                "{}:{}",
                json_str("warn_missing_import"),
                spec.warn_missing_import
            )))
            .collect();
        let net = spec.taint_network_sink_args.map_or_else(
            || "null".to_string(),
            |a| {
                let mut v: Vec<u8> = a.to_vec();
                v.sort_unstable();
                format!(
                    "[{}]",
                    v.iter().map(u8::to_string).collect::<Vec<_>>().join(",")
                )
            },
        );
        let scalars = format!(
            "{{\"taint_output_sink\":{},\"taint_log_sink\":{},\"taint_transform\":{},\"taint_double_encode\":{},\"taint_sink_safe\":{},\"taint_network_sink_args\":{net},\"taint_interp_eval\":{},\"taint_output_sink_subcommands\":{},\"credential_options\":{},\"sensitive_headers\":{},\"pattern_type\":{},\"format_string_type\":{},\"tcllib_package\":{},\"is_namespace_exported\":{},\"xc_translatable\":{},\"deprecated_replacement\":{},\"inferred_storage_type\":{},\"assigns_variable_at\":{},\"safe_on_uninit\":{}}}",
            opt_str_or_null(spec.taint_output_sink),
            opt_str_or_null(spec.taint_log_sink),
            colours_json(spec.taint_transform),
            colours_json(spec.taint_double_encode_colour),
            colours_json(spec.taint_sink_safe_colour),
            json_str_list(spec.taint_interp_eval_subcommands),
            json_str_list(spec.taint_output_sink_subcommands),
            json_str_list(spec.credential_options),
            json_str_list(spec.sensitive_headers),
            spec.pattern_type
                .map_or_else(|| "null".to_string(), |p| json_str(&format!("{p:?}"))),
            spec.format_string_type
                .map_or_else(|| "null".to_string(), |f| json_str(&format!("{f:?}"))),
            opt_str_or_null(spec.tcllib_package),
            spec.is_namespace_exported,
            spec.xc_translatable
                .map_or_else(|| "null".to_string(), |b| b.to_string()),
            opt_str_or_null(spec.deprecated_replacement),
            spec.inferred_storage_type
                .map_or_else(|| "null".to_string(), |t| json_str(&format!("{t:?}"))),
            spec.assigns_variable_at
                .map_or_else(|| "null".to_string(), |n| n.to_string()),
            spec.safe_on_uninit.is_some(),
        );
        println!(
            "{{\"name\":{},\"snippet\":{snippet},\"side_effects\":{},\"arg_types\":{},\"arg_roles\":{},\"options\":{},\"subs\":{subs_json},\"scalars\":{scalars},\"bools\":{{{}}}}}",
            json_str(spec.name),
            side_effects_json(spec.side_effects),
            arg_types_json(spec.arg_types),
            arg_roles_json(spec.arg_roles),
            options_json(spec.options),
            bools.join(","),
        );
    }
}

// Flat top-to-bottom argument parsing and dispatch for this debug dumper;
// splitting it into helpers would scatter the one linear control flow.
#[allow(clippy::too_many_lines)]
fn main() {
    let group = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: dump_specs <group>");
        std::process::exit(2);
    });

    // Deep per-command dump (content-level completeness audit). Pairs
    // with the reference deep dumper.
    if let Some(g) = group.strip_prefix("deep-") {
        deep_dump(g);
        return;
    }

    // BIG-IP object registry: one JSON record per object kind.
    if group == "bigip" {
        let reg = tcl_registry::bigip::BigipRegistry::build();
        let mut specs: Vec<_> = reg.specs().to_vec();
        specs.sort_by_key(|s| s.kind_spec.kind);
        for spec in specs {
            let ks = spec.kind_spec;
            let object_types = json_str_list(ks.object_types);
            let prop_names: Vec<&str> = spec.properties.iter().map(|p| p.name).collect();
            let props = json_str_list(&prop_names);
            // `(name, kind)` pairs as a sorted JSON array — keeps
            // duplicate property names (the bgp object legitimately
            // declares e.g. `distance` three times with different
            // kinds), so the comparison is an order-independent multiset.
            // Full per-property signature (every field), as a sorted
            // multiset — duplicates kept (bgp's `distance` ×3).
            let prop_pairs: Vec<String> = {
                let mut pv: Vec<String> = spec.properties.iter().map(bigip_prop_sig).collect();
                pv.sort_unstable();
                pv.iter().map(|s| json_str(s)).collect()
            };
            println!(
                "{{\"kind\":{},\"module\":{},\"object_types\":{},\"n_header_types\":{},\"n_properties\":{},\"properties\":{},\"property_pairs\":[{}]}}",
                json_str(ks.kind),
                ks.module.map_or_else(|| "null".to_string(), json_str),
                object_types,
                spec.header_types.len(),
                spec.properties.len(),
                props,
                prop_pairs.join(","),
            );
        }
        return;
    }

    // Meta registries (events / profiles / namespaces): emit one name per line.
    match group.as_str() {
        "meta-events" => {
            let reg = EventRegistry::build();
            let mut names = reg.all_event_names();
            names.sort_unstable();
            for n in names {
                println!("{n}");
            }
            return;
        }
        "meta-profiles" => {
            let reg = ProfileRegistry::build();
            let mut names = reg.all_profile_names();
            names.sort_unstable();
            for n in names {
                println!("{n}");
            }
            return;
        }
        "meta-namespaces" => {
            let reg = ProfileRegistry::build();
            let mut names = reg.all_namespace_prefixes();
            names.sort_unstable();
            for n in names {
                println!("{n}");
            }
            return;
        }
        "meta-events-props" => {
            let reg = EventRegistry::build();
            let mut names = reg.all_event_names();
            names.sort_unstable();
            for n in names {
                let p = reg.get_props(n).expect("name from registry");
                let transport = json_str_list(p.transport);
                let implied = json_str_list(p.implied_profiles);
                let setup = p.setup_event.map_or_else(|| "null".to_string(), json_str);
                println!(
                    "{{\"name\":{},\"client_side\":{},\"server_side\":{},\"transport\":{},\"implied_profiles\":{},\"flow\":{},\"deprecated\":{},\"hot\":{},\"common\":{},\"setup_event\":{}}}",
                    json_str(n),
                    p.client_side,
                    p.server_side,
                    transport,
                    implied,
                    p.flow,
                    p.deprecated,
                    p.hot,
                    p.common,
                    setup,
                );
            }
            return;
        }
        _ => {}
    }

    let specs = group_specs(&group);
    for spec in &specs {
        // dialects
        let (dialects_all, dialects) = match spec.dialects {
            None => (true, Vec::new()),
            Some(d) => (false, dialect_tags(d)),
        };
        // arity
        let arity_min = spec.arity.min;
        let arity_max = if spec.arity.is_unlimited() {
            "null".to_string()
        } else {
            spec.arity.max.to_string()
        };
        // hover
        let (hover, summary, synopsis, source, examples, return_value) = match &spec.hover {
            None => (
                false,
                String::new(),
                Vec::new(),
                String::new(),
                false,
                false,
            ),
            Some(h) => (
                true,
                h.summary.to_string(),
                h.synopsis.to_vec(),
                h.source.to_string(),
                !h.examples.is_empty(),
                !h.return_value.is_empty(),
            ),
        };
        let source_is_url = source.starts_with("http");
        // options (command level)
        let option_names: Vec<&str> = spec.options.iter().map(|o| o.name).collect();
        // subcommands
        let subcommand_names: Vec<&str> = spec.subcommands.iter().map(|s| s.name).collect();
        // return type
        let return_type = spec
            .return_type
            .map(|t| format!("{t:?}"))
            .unwrap_or_default();
        let body_kind = format!("{:?}", spec.body_kind);

        // event_requires (GAP-3a): the Rust CommandSpec now carries an
        // `event_requires` field; emit it in the shared normalised schema
        // so the audit's event_* dimensions compare like-for-like.
        let (event_profiles, event_also_in, event_requires_any) = match &spec.event_requires {
            None => (Vec::new(), Vec::new(), false),
            Some(er) => {
                let mut profiles: Vec<&str> = er.profiles.to_vec();
                profiles.sort_unstable();
                let mut also_in: Vec<&str> = er.also_in.to_vec();
                also_in.sort_unstable();
                let any = er.client_side
                    || er.server_side
                    || er.transport.is_some()
                    || !er.profiles.is_empty()
                    || !er.also_in.is_empty()
                    || er.init_only
                    || er.flow
                    || er.capability.is_some();
                (profiles, also_in, any)
            }
        };

        let mut fields: Vec<String> = Vec::new();
        fields.push(format!("\"name\":{}", json_str(spec.name)));
        fields.push(format!("\"dialects_all\":{dialects_all}"));
        fields.push(format!("\"dialects\":{}", json_str_list(&dialects)));
        fields.push(format!("\"arity_min\":{arity_min}"));
        fields.push(format!("\"arity_max\":{arity_max}"));
        fields.push(format!("\"hover\":{hover}"));
        fields.push(format!("\"summary\":{}", json_str(&summary)));
        fields.push(format!("\"synopsis\":{}", json_str_list(&synopsis)));
        fields.push(format!("\"source\":{}", json_str(&source)));
        fields.push(format!("\"source_is_url\":{source_is_url}"));
        fields.push(format!("\"examples\":{examples}"));
        fields.push(format!("\"return_value\":{return_value}"));
        fields.push(format!("\"n_forms\":{}", spec.forms.len()));
        fields.push(format!("\"options\":{}", json_str_list(&option_names)));
        fields.push(format!("\"n_subcommands\":{}", spec.subcommands.len()));
        fields.push(format!(
            "\"subcommands\":{}",
            json_str_list(&subcommand_names)
        ));
        fields.push(format!("\"n_side_effects\":{}", spec.side_effects.len()));
        fields.push(format!(
            "\"event_profiles\":{}",
            json_str_list(&event_profiles)
        ));
        fields.push(format!(
            "\"event_also_in\":{}",
            json_str_list(&event_also_in)
        ));
        fields.push(format!("\"event_requires_any\":{event_requires_any}"));
        fields.push(format!(
            "\"excluded_events\":{}",
            json_str_list(spec.excluded_events)
        ));
        fields.push(format!(
            "\"required_package\":{}",
            spec.required_package
                .map_or_else(|| "null".to_string(), json_str)
        ));
        fields.push(format!(
            "\"return_type\":{}",
            if return_type.is_empty() {
                "null".to_string()
            } else {
                json_str(&return_type)
            }
        ));
        fields.push(format!("\"n_arg_types\":{}", spec.arg_types.len()));
        fields.push(format!("\"n_arg_roles\":{}", spec.arg_roles.len()));
        fields.push(format!("\"body_kind\":{}", json_str(&body_kind)));
        // codegen-registry dimensions.
        fields.push(format!("\"has_lowering\":{}", spec.lowering_hook.is_some()));
        fields.push(format!("\"has_codegen\":{}", spec.codegen_hook.is_some()));
        fields.push(format!(
            "\"has_const_fold\":{}",
            spec.const_fold.is_some() || spec.const_fold_versioned.is_some()
        ));
        fields.push(format!("\"has_traits\":{}", !spec.traits.is_empty()));

        println!("{{{}}}", fields.join(","));
    }
    eprintln!("[dump_specs] group={group} count={}", specs.len());
}
