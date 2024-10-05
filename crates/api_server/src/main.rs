use actix_cors::Cors;
use actix_web::{error, get, web, App, HttpServer, Result};
use dt_core::{
    graph::used_by_graph::UsedByGraph,
    portable::Portable,
    tracker::{DependencyTracker, TraceTarget},
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

#[get("/search/{search}")]
async fn search(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<web::Json<SearchResponse>> {
    let search = path.into_inner();

    // TODO: deal with search mode

    let mut dependency_tracker = DependencyTracker::new(&data.used_by_graph, true);
    let matched_i18n_keys = vec![&search];
    let mut trace_result = HashMap::new();
    for i18n_key in matched_i18n_keys {
        let mut route_to_paths = HashMap::new();
        if let Some(i18n_key_usage) = data.i18n_to_symbol.get(i18n_key) {
            for (module_path, symbols) in i18n_key_usage {
                for symbol in symbols {
                    let full_paths = dependency_tracker
                        .trace((module_path.clone(), TraceTarget::LocalVar(symbol.clone())))
                        .unwrap();
                    // traverse each path and check if any symbol is used in some routes
                    for mut full_path in full_paths {
                        full_path.reverse();
                        for (i, (step_module_path, step_trace_target)) in
                            full_path.iter().enumerate()
                        {
                            match step_trace_target {
                                TraceTarget::LocalVar(step_symbol_name) => {
                                    if let Some(symbol_to_routes) =
                                        data.symbol_to_route.get(step_module_path)
                                    {
                                        if let Some(routes) = symbol_to_routes.get(step_symbol_name)
                                        {
                                            let dependency_from_target_to_route: Vec<Step> =
                                                full_path[0..i]
                                                    .iter()
                                                    .map(|(path, target)| Step {
                                                        module_path: path.clone(),
                                                        symbol_name: target.to_string(),
                                                    })
                                                    .collect();
                                            for route in routes.iter() {
                                                if !route_to_paths.contains_key(route) {
                                                    route_to_paths
                                                        .insert(route.clone(), HashMap::new());
                                                }
                                                if !route_to_paths
                                                    .get(route)
                                                    .unwrap()
                                                    .contains_key(symbol)
                                                {
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
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
        trace_result.insert(i18n_key.to_string(), route_to_paths);
    }

    Ok(web::Json(SearchResponse {
        project_root: data.project_root.to_owned(),
        trace_result,
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TODO: get portable path from args
    let mut file = File::open("<the portable path>")?;
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
