use std::path::PathBuf;

use dependency_tracker::{
    depend_on_graph::DependOnGraph,
    dependency_tracker::{DependencyTracker, TraceTarget},
    parser::parse,
    path_resolver::{PathResolver, ResolvePath, ToCanonicalString},
    scheduler::ParserCandidateScheduler,
    spreadsheet::write_to_spreadsheet,
    used_by_graph::UsedByGraph,
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
    let used_by_graph = UsedByGraph::from(&depend_on_graph);
    let mut dependency_tracker = DependencyTracker::new(&used_by_graph);
    let traced_paths = dependency_tracker.trace((
        path_resolver.resolve_path("", "components/buttons/counter")?,
        TraceTarget::LocalVar(String::from("Counter")),
    ))?;
    write_to_spreadsheet("./output.xlsx", &traced_paths);

    Ok(())
}
