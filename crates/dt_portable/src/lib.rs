use dt_graph::used_by_graph::UsedByGraph;
use dt_tracker::ModuleSymbol;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct Portable {
    pub project_root: String,

    // {
    //   "i18n.bird" => {
    //     "module path 1" => ["A", "B"],
    //     "module path 2" => ["Foo", "Bar"],
    //   },
    //   "i18n.cat" => {
    //     "module path 1" => ["A", "B", "C"],
    //   },
    // }
    pub i18n_to_symbol: HashMap<String, HashMap<String, HashSet<String>>>,

    // {
    //   ("module path 1", LocalVar("A")) => ["/route/path/x", "/route/path/y"]
    //   ("module path 1", LocalVar("B")) => ["/route/path/x"]
    //   ("module path 2", LocalVar("A")) => ["/route/path/z"]
    // }
    pub symbol_to_route: HashMap<ModuleSymbol, Vec<String>>,

    // {
    //   "module path 1" => {
    //     local_variable_table: ...
    //     named_export_table: ...
    //     default_export: ...
    //   }
    // }
    pub used_by_graph: UsedByGraph,
}

impl Portable {
    pub fn new(
        project_root: String,
        i18n_to_symbol: HashMap<String, HashMap<String, HashSet<String>>>,
        symbol_to_route: HashMap<ModuleSymbol, Vec<String>>,
        used_by_graph: UsedByGraph,
    ) -> Self {
        Self {
            project_root,
            i18n_to_symbol,
            symbol_to_route,
            used_by_graph,
        }
    }

    pub fn export(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn import(exported: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(exported)?)
    }
}
