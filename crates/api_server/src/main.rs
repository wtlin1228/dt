use actix_cors::Cors;
use actix_web::{error, get, web, App, HttpServer, Result};
use dt_core::{
    graph::used_by_graph::UsedByGraph,
    portable::Portable,
    tracker::{DependencyTracker, ModuleSymbol, TraceTarget},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
};

struct AppState {
    project_root: String,
    i18n_to_symbol: HashMap<String, HashMap<String, HashSet<String>>>,
    symbol_to_route: HashMap<String, HashMap<String, Vec<String>>>,
    used_by_graph: UsedByGraph,
}

#[derive(Serialize)]
struct SearchResponse {
    project_root: String,
    i18n_to_symbol: HashMap<String, HashSet<String>>,
    trace_result: HashMap<String, HashMap<String, Vec<Vec<ModuleSymbol>>>>,
}

#[get("/search/{search}")]
async fn search(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<web::Json<SearchResponse>> {
    let search = path.into_inner();

    // TODO: deal with search mode

    match data.i18n_to_symbol.get(&search) {
        None => Err(error::ErrorNotFound(format!("{} not found", search))),
        Some(ts) => {
            let mut dependency_tracker = DependencyTracker::new(&data.used_by_graph, true);
            let mut trace_result: HashMap<String, HashMap<String, Vec<Vec<ModuleSymbol>>>> =
                HashMap::new();
            for (module_path, symbols) in ts {
                trace_result.insert(module_path.to_owned(), HashMap::new());
                for symbol in symbols {
                    let full_paths = dependency_tracker
                        .trace((module_path.clone(), TraceTarget::LocalVar(symbol.clone())))
                        .unwrap();
                    trace_result
                        .entry(module_path.to_owned())
                        .and_modify(|symbol_table| {
                            symbol_table.insert(symbol.to_owned(), full_paths);
                        });
                }
            }

            Ok(web::Json(SearchResponse {
                project_root: data.project_root.to_owned(),
                i18n_to_symbol: ts.to_owned(),
                trace_result,
            }))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TODO: get portable path from args
    // let mut file = File::open("<the portable path>")?;
    let mut file =
        File::open("/Users/linweitang/rust/js-symbol-dependency-tracker/outputs/sample.json")?;
    let mut exported = String::new();
    file.read_to_string(&mut exported)?;
    let portable = Portable::import(&exported).unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::default().allowed_origin("http://localhost:5173"))
            .app_data(web::Data::new(AppState {
                project_root: portable.project_root.clone(),
                i18n_to_symbol: portable.i18n_to_symbol.clone(),
                symbol_to_route: portable.symbol_to_route.clone(),
                used_by_graph: portable.used_by_graph.clone(),
            }))
            .service(search)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
