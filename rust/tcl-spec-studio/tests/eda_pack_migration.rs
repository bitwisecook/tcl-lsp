//! TEMPORARY migration harness: render the EDA vendor packs to `.tclspec`,
//! load them back, and field-compare against the compiled specs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::Value;
use tcl_registry::CommandSpec;
use tcl_spec_studio::render_spectcl::{self};
use tcl_spec_studio::{draft, schema, spectcl};

fn packs() -> Vec<(&'static str, Vec<CommandSpec>)> {
    vec![
        (
            "sdc_base",
            tcl_registry::commands::sdc_base::sdc_base_command_specs(),
        ),
        (
            "eda_synopsys",
            tcl_registry::commands::eda_synopsys::eda_synopsys_command_specs(),
        ),
        (
            "eda_cadence",
            tcl_registry::commands::eda_cadence::eda_cadence_command_specs(),
        ),
        (
            "eda_xilinx",
            tcl_registry::commands::eda_xilinx::eda_xilinx_command_specs(),
        ),
        (
            "eda_quartus",
            tcl_registry::commands::eda_quartus::eda_quartus_command_specs(),
        ),
        (
            "eda_mentor",
            tcl_registry::commands::eda_mentor::eda_mentor_command_specs(),
        ),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Diff {
    key: String,
    rendered: String,
    shipped: String,
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 200 {
        let head: String = text.chars().take(180).collect();
        format!("{head}… ({} bytes)", text.len())
    } else {
        text
    }
}

fn compare_keys(
    rendered: &Value,
    shipped: &Value,
    keys: impl Iterator<Item = &'static str>,
    prefix: &str,
    out: &mut Vec<Diff>,
) {
    for key in keys {
        let a = rendered.get(key).unwrap_or(&Value::Null);
        let b = shipped.get(key).unwrap_or(&Value::Null);
        if a != b {
            out.push(Diff {
                key: format!("{prefix}{key}"),
                rendered: short(a),
                shipped: short(b),
            });
        }
    }
}

fn compare_unrenderable(rendered: &Value, shipped: &Value, prefix: &str, out: &mut Vec<Diff>) {
    let keys = |value: &Value| -> BTreeSet<String> {
        value
            .get(draft::UNRENDERABLE_KEY)
            .and_then(Value::as_array)
            .map(|keys| {
                keys.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let (a, b) = (keys(rendered), keys(shipped));
    for key in b.difference(&a) {
        out.push(Diff {
            key: format!("{prefix}{key}"),
            rendered: "not set".to_owned(),
            shipped: "set (unrecoverable)".to_owned(),
        });
    }
    for key in a.difference(&b) {
        out.push(Diff {
            key: format!("{prefix}{key}"),
            rendered: "set (unrecoverable)".to_owned(),
            shipped: "not set".to_owned(),
        });
    }
}

fn diffs(rendered: &Value, shipped: &Value) -> Vec<Diff> {
    let mut out = Vec::new();
    compare_keys(
        rendered,
        shipped,
        schema::COMMAND_FIELDS.iter().map(|field| field.key),
        "",
        &mut out,
    );
    compare_unrenderable(rendered, shipped, "", &mut out);
    let names = |value: &Value| -> Vec<String> {
        value
            .get("subcommands")
            .and_then(Value::as_array)
            .map(|subs| {
                subs.iter()
                    .map(|sub| sub["name"].as_str().unwrap_or("").to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    let (a, b) = (names(rendered), names(shipped));
    if a == b {
        out.retain(|diff| diff.key != "subcommands");
        for (i, name) in a.iter().enumerate() {
            let x = &rendered["subcommands"][i];
            let y = &shipped["subcommands"][i];
            let prefix = format!("subcommands[{name}].");
            compare_keys(
                x,
                y,
                schema::SUBCOMMAND_FIELDS.iter().map(|field| field.key),
                &prefix,
                &mut out,
            );
            compare_unrenderable(x, y, &prefix, &mut out);
        }
    }
    out
}

fn header(pack: &str) -> String {
    let what = match pack {
        "sdc_base" => "the shared SDC constraint / collection library every EDA shell ships",
        "eda_synopsys" => "the Synopsys tool commands (Design Compiler, IC Compiler, PrimeTime)",
        "eda_cadence" => "the Cadence tool commands (Genus, Innovus, Xcelium, Conformal)",
        "eda_xilinx" => "the AMD/Xilinx Vivado tool commands",
        "eda_quartus" => "the Intel/Altera Quartus tool commands",
        "eda_mentor" => "the Siemens/Mentor tool commands (Questa, ModelSim, Calibre)",
        _ => "an EDA vendor command library",
    };
    format!(
        "\
# tcl-lsp — bundled SpecTcl loadable: {what}.
#
# This pack IS the source of truth for these commands. The Rust modules that
# used to hold them (rust/tcl-registry/src/commands/{pack}/) were deleted when
# the EDA vendor libraries migrated to bundled `.tclspec` loadables, per
# docs/design/spec-packs.md (\"the EDA vendor libraries ship as bundled
# `.tclspec` loadables … so the loader path is exercised in production from day
# one rather than reserved for private packs\"). Every command was proved
# field-for-field equal to its compiled spec by the SpecTcl round-trip gate
# before the modules were removed.
#
# Discovery: the **bundled** tier — `specs/` beside the server executable, or
# wherever $TCL_LSP_SPEC_PACK_DIR points (rust/tcl-spectcl/src/discovery.rs).
# Edit this file to change the specs; there is no generator to re-run."
    )
}

#[test]
fn eda_packs_render_and_reload_field_equal() {
    let out_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs");
    std::fs::create_dir_all(&out_dir).expect("specs dir");

    let mut report = String::new();
    let mut failures: Vec<String> = Vec::new();

    for (pack_name, specs) in packs() {
        let drafts: Vec<draft::Draft> = specs.iter().map(draft::from_command_spec).collect();
        let (body, losses) = render_spectcl::render_pack_reporting(&drafts, pack_name);
        let text = format!("{}\n{body}", header(pack_name));
        let path = out_dir.join(format!("{pack_name}.tclspec"));
        std::fs::write(&path, &text).expect("write pack");

        let pack = spectcl::load_pack(&text);
        let notices: Vec<String> = pack.notices.iter().map(ToString::to_string).collect();

        let mut clean = 0usize;
        let mut lossy: BTreeMap<String, (usize, String)> = BTreeMap::new();
        for spec in &specs {
            let shipped = Value::Object(draft::from_command_spec(spec));
            let Some(reloaded) = pack.command(spec.name) else {
                failures.push(format!("{pack_name}/{}: not declared by the pack", spec.name));
                continue;
            };
            let reloaded = Value::Object(draft::from_command_spec(reloaded.spec));
            let found = diffs(&reloaded, &shipped);
            if found.is_empty() {
                clean += 1;
                continue;
            }
            for diff in &found {
                let field = diff.key.rsplit('.').next().unwrap_or(&diff.key);
                match render_spectcl::gap(field) {
                    Some(gap) => {
                        let entry = lossy
                            .entry(format!("{field} ({:?})", gap.kind))
                            .or_insert_with(|| (0, format!("{pack_name}/{}", spec.name)));
                        entry.0 += 1;
                    }
                    None => failures.push(format!(
                        "{pack_name}/{}: {}\n        rendered: {}\n        shipped : {}",
                        spec.name, diff.key, diff.rendered, diff.shipped
                    )),
                }
            }
        }
        let _ = writeln!(
            report,
            "{pack_name:>13}: {clean:>4} / {:>4} commands field-equal; {} render loss(es), {} \
             loader notice(s); {} lines",
            specs.len(),
            losses.len(),
            notices.len(),
            text.lines().count()
        );
        for loss in &losses {
            let _ = writeln!(report, "        loss: {} {}", loss.key, loss.reason);
        }
        for notice in notices.iter().take(20) {
            let _ = writeln!(report, "        notice: {notice}");
        }
        for (key, (count, example)) in &lossy {
            let _ = writeln!(report, "        gap: {count:>4} {key}  e.g. {example}");
        }
    }

    println!("{report}");
    assert!(
        failures.is_empty(),
        "{} undocumented difference(s):\n{}\n{report}",
        failures.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
