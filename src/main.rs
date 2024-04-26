use std::path::PathBuf;

use dependency_tracker::{
    parser::parse,
    path_resolver::{PathResolver, ToCanonicalString},
    scheduler::ParserCandidateScheduler,
    DependencyTracker,
};

const ROOT: &'static str = "./test-project/everybodyyyy/src";

fn main() -> anyhow::Result<()> {
    let mut scheduler = ParserCandidateScheduler::new(&PathBuf::from(ROOT));
    let path_resolver = PathResolver::new(&PathBuf::from(ROOT).to_canonical_string()?);
    let mut dependency_tracker = DependencyTracker::new();

    loop {
        match scheduler.get_one_candidate() {
            Some(c) => {
                let parsed_module = parse(c.to_str().unwrap()).unwrap();
                dependency_tracker
                    .add_parsed_module(parsed_module, &path_resolver)
                    .unwrap();
                scheduler.mark_candidate_as_parsed(c);
            }
            None => break,
        }
    }

    println!("{:#?}", dependency_tracker);
    println!(
        "parsed module count: {}",
        dependency_tracker.parsed_modules_table.len()
    );

    Ok(())
}
