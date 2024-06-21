use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, BasicHistory, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
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

const SYMBOL_TYPE_SELECTIONS: [&str; 3] = ["Default Export", "Named Export", "Local Variable"];

#[derive(Parser, Debug)]
#[command(version, about = "Track fine-grained symbol dependency graph", long_about = None)]
struct Args {
    /// Path of project to trace
    #[arg(short)]
    src: String,

    /// Path of the output folder
    #[arg(short)]
    dst: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut scheduler = ParserCandidateScheduler::new(&PathBuf::from(&args.src));
    let path_resolver = PathResolver::new(&PathBuf::from(&args.src).to_canonical_string()?);
    let mut depend_on_graph = DependOnGraph::new();

    let bar = ProgressBar::new(scheduler.get_total_remaining_candidate_count() as u64);
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )?
        .progress_chars("##-"),
    );
    loop {
        match scheduler.get_one_candidate() {
            Some(c) => {
                let parsed_module = parse(c.to_str().unwrap())?;
                depend_on_graph.add_parsed_module(parsed_module, &path_resolver)?;
                scheduler.mark_candidate_as_parsed(c);
                bar.inc(1);
            }
            None => break,
        }
    }
    bar.finish_with_message("all modules parsed 🌲");

    let used_by_graph = UsedByGraph::from(&depend_on_graph);
    let mut dependency_tracker = DependencyTracker::new(&used_by_graph);
    loop {
        let mut target_path_history = BasicHistory::new().max_entries(8).no_duplicates(true);
        let target_path = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Your module path")
            .history_with(&mut target_path_history)
            .validate_with(|input: &String| -> anyhow::Result<()> {
                let input = path_resolver.resolve_path("", input)?;
                dependency_tracker.validate_module_path(&input)
            })
            .interact_text()?;
        let resolved_target_path = path_resolver.resolve_path("", &target_path)?;

        let symbol_type_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Pick the symbol type")
            .default(0)
            .items(&SYMBOL_TYPE_SELECTIONS[..])
            .interact()
            .unwrap();

        let target_symbol = match SYMBOL_TYPE_SELECTIONS[symbol_type_selection] {
            "Default Export" => TraceTarget::DefaultExport,
            "Named Export" => {
                let traceable_symbols =
                    dependency_tracker.get_traceable_named_exports(&resolved_target_path)?;
                let traceable_symbols_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Pick named export")
                    .default(0)
                    .items(&traceable_symbols[..])
                    .interact()
                    .unwrap();
                TraceTarget::NamedExport(traceable_symbols[traceable_symbols_selection].to_string())
            }
            "Local Variable" => {
                let traceable_symbols =
                    dependency_tracker.get_traceable_named_exports(&resolved_target_path)?;
                let traceable_symbols_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Pick local variable")
                    .default(0)
                    .items(&traceable_symbols[..])
                    .interact()
                    .unwrap();
                TraceTarget::LocalVar(traceable_symbols[traceable_symbols_selection].to_string())
            }
            _ => unreachable!(),
        };

        let track_result =
            dependency_tracker.trace((resolved_target_path, target_symbol.clone()))?;

        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();
        let output_path = format!(
            "{}/{}__{}.xlsx",
            args.dst,
            target_symbol.to_string(),
            rand_string
        );
        write_to_spreadsheet(&output_path, &track_result);

        println!(
            "Track result has been saved to {}",
            style(&output_path).cyan()
        );

        match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to continue?")
            .interact_opt()?
        {
            Some(true) => {}
            Some(false) | None => break,
        }
    }

    Ok(())
}
