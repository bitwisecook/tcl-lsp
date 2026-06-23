//! The `highlight` verb — emit ANSI- or HTML-highlighted source.
//!
//! Note the tab
//! handling differs from the transform verbs: here highlighting runs *first*
//! and tab expansion *after* (so `str.expandtabs` sees the escape codes), which
//! is the order for this verb specifically.

use tcl_cli_support::{
    OutputTarget, combine_sources, expand_tabs, highlight_ansi, highlight_html,
    read_input_documents, resolve_use_colour, write_text_output,
};

use crate::cli::{ColourArgs, InputArgs};

/// Default tab-expansion width on stdout (the CLI default).
const DEFAULT_TAB_WIDTH: usize = 4;

/// `tcl highlight` — emit syntax-highlighted source (ANSI or HTML).
pub fn run_highlight(input: &InputArgs, format: &str, colour: &ColourArgs) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let source = combine_sources(&documents);
    let target = OutputTarget::from_arg(input.output.as_deref());

    let mut out = if format == "html" {
        highlight_html(&source, &input.dialect)
    } else if resolve_use_colour(colour.colour, colour.no_colour, &target) {
        highlight_ansi(&source, &input.dialect)
    } else {
        source
    };

    if target.is_stdout() && DEFAULT_TAB_WIDTH > 0 {
        out = expand_tabs(&out, DEFAULT_TAB_WIDTH);
    }
    write_text_output(&target, &out)?;
    Ok(0)
}
