//! Registry-parity dumper.
//!
//! Emits one JSON object per command spec (JSONL) for a given dialect
//! group, using a normalised schema shared with the Python dumper
//! (`scripts/registry-audit/dump_python.py`). Used by the rust-rewrite
//! registry audit to diff the Rust port against the Python source of
//! truth.
//!
//! Usage: `cargo run -q --example dump_specs -- <group>`
//! where <group> is one of:
//!   tcl stdlib tcllib irules iapps tk expect
//!   sdc-base synopsys cadence xilinx quartus mentor

use std::fmt::Write as _;

use tcl_registry::commands;
use tcl_registry::dialects::DialectSet;
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
    (DialectSet::SYNOPSYS, "synopsys-eda-tcl"),
    (DialectSet::CADENCE, "cadence-eda-tcl"),
    (DialectSet::XILINX, "xilinx-eda-tcl"),
    (DialectSet::QUARTUS, "intel-quartus-eda-tcl"),
    (DialectSet::MENTOR, "mentor-eda-tcl"),
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

#[allow(clippy::too_many_lines)]
fn main() {
    let group = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: dump_specs <group>");
        std::process::exit(2);
    });

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

        // Emit JSON. event_* fields are intentionally always empty/false:
        // the Rust CommandSpec has no event_requires field (see audit).
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
        fields.push("\"event_profiles\":[]".to_string());
        fields.push("\"event_also_in\":[]".to_string());
        fields.push("\"event_requires_any\":false".to_string());
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
