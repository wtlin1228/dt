use anyhow::Context;
use clap::Parser;
use dt_core::{
    graph::{depend_on_graph::DependOnGraph, used_by_graph::UsedByGraph},
    i18n::collect_all_translation_usage,
    parser::{collect_symbol_dependency, Input},
    path_resolver::ToCanonicalString,
    portable::Portable,
    scheduler::ParserCandidateScheduler,
};

use std::{fs::File, io::prelude::*, path::PathBuf};

#[derive(Parser)]
#[command(version, about = "Parse a project and serialize its output", long_about = None)]
struct Cli {
    /// Input path
    #[arg(short)]
    input: String,

    /// Output path
    #[arg(short)]
    output: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let project_root = PathBuf::from(&cli.input).to_canonical_string()?;

    let portable = Portable::new(
        project_root.to_owned(),
        collect_all_translation_usage(&project_root)?,
        construct_used_by_graph(&project_root)?,
    );

    let serialized = portable.export()?;
    let mut file = File::create(&cli.output)?;
    file.write_all(serialized.as_bytes())?;

    Ok(())
}

fn construct_used_by_graph(project_root: &str) -> anyhow::Result<UsedByGraph> {
    let mut scheduler = ParserCandidateScheduler::new(&project_root);
    let mut depend_on_graph = DependOnGraph::new(&project_root);
    loop {
        match scheduler.get_one_candidate() {
            Some(c) => {
                let module_src = c.to_str().context(format!("to_str() failed: {:?}", c))?;
                let module_ast = Input::Path(module_src).get_module_ast()?;
                let symbol_dependency = collect_symbol_dependency(&module_ast, module_src)?;
                depend_on_graph.add_symbol_dependency(symbol_dependency)?;
                scheduler.mark_candidate_as_parsed(c);
            }
            None => break,
        }
    }
    Ok(UsedByGraph::from(&depend_on_graph))
}
