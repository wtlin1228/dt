use std::path::PathBuf;

use dependency_tracker::{
    depend_on_graph::DependOnGraph,
    parser::parse,
    path_resolver::{PathResolver, ToCanonicalString},
    scheduler::ParserCandidateScheduler,
};

const ROOT: &'static str = "./test-project/everybodyyyy/src";

fn main() -> anyhow::Result<()> {
    let mut scheduler = ParserCandidateScheduler::new(&PathBuf::from(ROOT));
    let path_resolver = PathResolver::new(&PathBuf::from(ROOT).to_canonical_string()?);
    let mut depend_on_graph = DependOnGraph::new();

    loop {
        match scheduler.get_one_candidate() {
            Some(c) => {
                let parsed_module = parse(c.to_str().unwrap())?;
                depend_on_graph.add_parsed_module(parsed_module, &path_resolver)?;
                scheduler.mark_candidate_as_parsed(c);
            }
            None => break,
        }
    }

    // all modules are parsed, draw the reversed graph

    println!("{:#?}", depend_on_graph);
    println!(
        "parsed module count: {}",
        depend_on_graph.parsed_modules_table.len()
    );

    Ok(())
}
