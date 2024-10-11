use actix_cors::Cors;
use actix_web::{error, get, web, App, HttpServer, Result};
use clap::Parser;
use dt_core::{
    database::{models, Database, SqliteDb},
    tracker::{db_version::DependencyTracker, TraceTarget},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Clone)]
struct Step {
    module_path: String,
    symbol_name: String,
}

#[derive(Serialize)]
struct SearchResponse {
    project_root: String,
    trace_result: HashMap<String, HashMap<String, HashMap<String, Vec<Vec<Step>>>>>,
}

#[derive(Deserialize)]
struct Info {
    q: String,
    exact_match: bool,
}

#[get("/search")]
async fn search(
    data: web::Data<AppState>,
    info: web::Query<Info>,
) -> Result<web::Json<SearchResponse>> {
    let db = &data.db;
    let search = &info.q;
    let exact_match = info.exact_match;
    // project name "default_project" can be different in feature "cross-project tracing"
    let project = models::Project::retrieve_by_name(&db.conn, "default_project").unwrap();
    let matched_i18n_keys = project
        .search_translation(&db.conn, search, exact_match)
        .unwrap();
    if matched_i18n_keys.len() == 0 {
        return Err(error::ErrorNotFound(format!("No result for {}", search)));
    }
    let mut dependency_tracker = DependencyTracker::new(&db, project.clone(), true);
    let mut trace_result = HashMap::new();
    for translation in matched_i18n_keys.iter() {
        let mut route_to_paths = HashMap::new();
        let translation_used_by = translation.get_used_by(&db.conn).unwrap();
        for symbol in translation_used_by.iter() {
            let module = models::Module::retrieve_by_id(&db.conn, symbol.module_id).unwrap();
            let full_paths = dependency_tracker
                .trace((
                    module.path.to_string(),
                    TraceTarget::LocalVar(symbol.name.to_string()),
                ))
                .unwrap();
            // traverse each path and check if any symbol is used in some routes
            for mut full_path in full_paths {
                full_path.reverse();
                for (i, (step_module_path, step_trace_target)) in full_path.iter().enumerate() {
                    match step_trace_target {
                        TraceTarget::LocalVar(step_symbol_name) => {
                            let step_module =
                                project.get_module(&db.conn, &step_module_path).unwrap();
                            let step_symbol = step_module
                                .get_symbol(
                                    &db.conn,
                                    models::SymbolVariant::LocalVariable,
                                    &step_symbol_name,
                                )
                                .unwrap();
                            let routes = step_symbol.get_used_by_routes(&db.conn).unwrap();
                            if routes.len() > 0 {
                                let dependency_from_target_to_route: Vec<Step> = full_path[0..i]
                                    .iter()
                                    .map(|(path, target)| Step {
                                        module_path: path.clone(),
                                        symbol_name: target.to_string(),
                                    })
                                    .collect();
                                for route in routes.iter() {
                                    let route = &route.path;
                                    let symbol = &symbol.name;
                                    if !route_to_paths.contains_key(route) {
                                        route_to_paths.insert(route.clone(), HashMap::new());
                                    }
                                    if !route_to_paths.get(route).unwrap().contains_key(symbol) {
                                        route_to_paths
                                            .get_mut(route)
                                            .unwrap()
                                            .insert(symbol.to_string(), vec![]);
                                    }
                                    route_to_paths
                                        .get_mut(route)
                                        .unwrap()
                                        .get_mut(symbol)
                                        .unwrap()
                                        .push(dependency_from_target_to_route.clone());
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        trace_result.insert(translation.key.to_string(), route_to_paths);
    }

    Ok(web::Json(SearchResponse {
        project_root: "".to_string(),
        trace_result,
    }))
}

struct AppState {
    db: SqliteDb,
}

#[derive(Parser)]
#[command(version, about = "Start the server to provide search API", long_about = None)]
struct Cli {
    /// The path of your database
    #[arg(long)]
    db: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::default().allow_any_method().allow_any_origin())
            .app_data(web::Data::new(AppState {
                db: SqliteDb::open(&cli.db).expect(&format!("open database from {}", cli.db)),
            }))
            .service(search)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
