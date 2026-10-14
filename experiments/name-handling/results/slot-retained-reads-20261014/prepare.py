from pathlib import Path
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-slot-retained-reads-20261014');path='rust/tcl-compiler/src/slot_allocation.rs';before=root.joinpath(path).read_text();after=before
old='use crate::var_refs::{VarReferenceScanner, VarScanOptions, vars_in_expr};';assert after.count(old)==1;after=after.replace(old,'use crate::place::{self, PlaceKind};')
start=after.index('/// Build a fresh [`VarReferenceScanner`]');end=after.index('/// The set of variable **names** an SSA statement reads / writes.',start)
after=after[:start]+'''/// Retained terminator reads select existing SSA cells; unknown reads keep
/// every known cell live. Detached standalone lexical text grants no cell proof.
fn terminator_read_symbols(
    cfg: &cfg::Function,
    id: BlockId,
    block: &cfg::Block,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
) -> HashSet<Symbol> {
    let Some(term) = block.terminator.as_ref() else {
        return HashSet::new();
    };
    let all_symbols = || {
        ssa.var_names()
            .iter()
            .filter_map(|name| ssa.var_symbol(name))
            .collect::<HashSet<_>>()
    };
    let Some(points) = ssa.point_contexts.as_ref() else {
        if !cfg.metadata_context.is_standalone() {
            return all_symbols();
        }
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let has_commands = match term {
            Terminator::Branch { condition, .. } => !condition.command_texts().is_empty(),
            Terminator::Return { value, expr, .. } => {
                value.as_ref().is_some_and(|value| {
                    !crate::var_refs::command_subst_texts_with_config(value, config).is_empty()
                }) || expr.as_ref().is_some_and(|expr| !expr.command_texts().is_empty())
            }
            Terminator::Goto { .. } | Terminator::Complete { .. } => false,
        };
        if has_commands {
            return all_symbols();
        }
        return crate::def_use::terminator_read_vars(Some(term), config)
            .into_iter()
            .filter_map(|(name, _)| ssa.var_symbol(&name))
            .filter(|symbol| ssa.cell_key(*symbol).authored_spelling().is_some())
            .collect();
    };
    let mut symbols = HashSet::new();
    for read in crate::place_bridge::terminator_read_places_at(term, id, points, registry) {
        for read in std::iter::once(read.clone()).chain(place::places_read_to_form(&read)) {
            if read.kind == PlaceKind::Unknown {
                return all_symbols();
            }
            let Some(key) = crate::var_resolve::canonical_binding_value_key(&read) else {
                if matches!(read.kind, PlaceKind::ArrayElem | PlaceKind::ArrayWhole) {
                    let root = crate::var_resolve::cell_key(&read);
                    symbols.extend(all_symbols().into_iter().filter(|symbol| {
                        ssa.cell_key(*symbol).root() == root.root()
                    }));
                    continue;
                }
                return all_symbols();
            };
            if let Some(symbol) = ssa.cell_symbol(&key) {
                symbols.insert(symbol);
            }
            if read.kind == PlaceKind::ArrayWhole {
                symbols.extend(all_symbols().into_iter().filter(|symbol| {
                    ssa.cell_key(*symbol).root() == key.root()
                }));
            }
        }
    }
    symbols
}

fn terminator_read_names(
    cfg: &cfg::Function,
    id: BlockId,
    block: &cfg::Block,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
) -> HashSet<String> {
    terminator_read_symbols(cfg, id, block, ssa, registry)
        .into_iter()
        .map(|symbol| ssa.var_name(symbol).to_owned())
        .collect()
}

'''+after[end:]
old='    let mut scanner = make_scanner(registry);\n';assert after.count(old)==2;after=after.replace(old,'')
old='terminator_read_names(block, &mut scanner, registry)';assert after.count(old)==3
# scalar consumer replaced separately.
start=after.index('    let mut read_families: HashMap<&str, HashSet<Symbol>>');end=after.index('        let mut upward_uses = HashSet::new();',start)
after=after[:start]+'''    let mut inputs = HashMap::new();
    for (id, block) in &cfg.blocks {
        let terminal_reads = terminator_read_symbols(cfg, *id, block, ssa, registry);
'''+after[end:]
old='terminator_read_names(block, &mut scanner, registry)';assert after.count(old)==2;after=after.replace(old,'terminator_read_names(cfg, *id, block, ssa, registry)')
for side,text in [('before',before),('after',after)]:
 dest=out/side/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(text)
print(path)
