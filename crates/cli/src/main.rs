use anyhow::Context;
use clap::Parser;
use dt_core::{
    database::{models, Database, SqliteDb},
    graph::{depend_on_graph::DependOnGraph, used_by_graph::UsedByGraph},
    i18n::I18nToSymbol,
    parser::{collect_symbol_dependency, Input},
    path_resolver::ToCanonicalString,
    portable::Portable,
    route::SymbolToRoutes,
    scheduler::ParserCandidateScheduler,
};
use std::{
    fs::File,
    io::{prelude::*, BufReader},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about = "Parse a project and serialize its output", long_about = None)]
struct Cli {
    /// Input path
    #[arg(short)]
    input: String,

    /// translation.json path
    #[arg(short)]
    translation_path: String,

    /// Output path
    #[arg(short)]
    output: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let project_root = PathBuf::from(&cli.input).to_canonical_string()?;
    let translation_json = File::open(&cli.translation_path)?;
    let translation_json_reader = BufReader::new(translation_json);
    let mut scheduler = ParserCandidateScheduler::new(&project_root);
    let mut depend_on_graph = DependOnGraph::new(&project_root);
    let mut symbol_to_route = SymbolToRoutes::new();
    let mut i18n_to_symbol = I18nToSymbol::new();

    let db = SqliteDb::open("./database/1009.db3")?;
    db.create_tables()?;
    let project = models::Project::create(&db.conn, &project_root)?;
    loop {
        match scheduler.get_one_candidate() {
            Some(c) => {
                let module_src = c.to_str().context(format!("to_str() failed: {:?}", c))?;
                let module = project.add_module(&db.conn, module_src)?;
                let module_ast = Input::Path(module_src).get_module_ast()?;
                let symbol_dependency = collect_symbol_dependency(&module_ast, module_src)?;
                i18n_to_symbol.collect_i18n_usage(module_src, &module_ast)?;
                symbol_to_route.collect_route_dependency(&module_ast, &symbol_dependency)?;

                depend_on_graph.add_symbol_dependency(symbol_dependency)?;
                scheduler.mark_candidate_as_parsed(c);
            }
            None => break,
        }
    }

    let portable = Portable::new(
        project_root.to_owned(),
        serde_json::from_reader(translation_json_reader)?,
        i18n_to_symbol.table,
        symbol_to_route.table,
        UsedByGraph::from(&depend_on_graph),
    );

    let serialized = portable.export()?;
    let mut file = File::create(&cli.output)?;
    file.write_all(serialized.as_bytes())?;

    Ok(())
}
