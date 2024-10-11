use crate::TraceTarget;

use super::ModuleSymbol;
use anyhow::Context;
use dt_database::{models, SqliteDb};
use std::collections::HashMap;

pub struct DependencyTracker<'db> {
    cache: HashMap<ModuleSymbol, Vec<Vec<ModuleSymbol>>>,
    db: &'db SqliteDb,
    project: models::Project,
    trace_full_path_only: bool,
}

impl<'db> DependencyTracker<'db> {
    pub fn new(db: &'db SqliteDb, project: models::Project, trace_full_path_only: bool) -> Self {
        Self {
            cache: HashMap::new(),
            db,
            project,
            trace_full_path_only,
        }
    }

    // Current implementation is mimick version of the trace with in-memory graph.
    // We can refactor it after the database feature gets validated.
    pub fn trace(&mut self, module_symbol: ModuleSymbol) -> anyhow::Result<Vec<Vec<ModuleSymbol>>> {
        // Treat routeNmaes specially since they cause a lot of circular dependencies in
        // some of our codebases. One assumption of this tool is "no circular dependency"
        // , so let's workaround here for now.
        if module_symbol.1.to_string() == "routeNames" {
            return Ok(vec![]);
        }

        // early return if cached
        if let Some(cached) = self.cache.get(&module_symbol) {
            return Ok(cached.clone());
        }

        let module = self
            .project
            .get_module(&self.db.conn, &module_symbol.0)
            .context(format!("module {} not found", module_symbol.0))?;

        let symbol = match &module_symbol.1 {
            crate::TraceTarget::NamedExport(name) => module
                .get_symbol(&self.db.conn, models::SymbolVariant::NamedExport, name)
                .context(format!(
                    "module {} doesn't have named export symbol {}",
                    module.path, name
                ))?,
            crate::TraceTarget::DefaultExport => module
                .get_symbol(&self.db.conn, models::SymbolVariant::DefaultExport, "")
                .context(format!(
                    "module {} doesn't have default export symbol",
                    module.path
                ))?,
            crate::TraceTarget::LocalVar(name) => module
                .get_symbol(&self.db.conn, models::SymbolVariant::LocalVariable, name)
                .context(format!(
                    "module {} doesn't have local variable symbol {}",
                    module.path, name
                ))?,
        };

        let used_by = symbol
            .get_used_by(&self.db.conn)
            .context(format!("get used-by vector for symbol {}", symbol.name))?;

        let mut res: Vec<Vec<ModuleSymbol>> = vec![];
        for next_target in used_by.iter() {
            let mut paths = match next_target.module_id == symbol.module_id {
                true => {
                    // used by symbol from the same module
                    match next_target.variant {
                        models::SymbolVariant::NamedExport => self.trace((
                            module_symbol.0.clone(),
                            TraceTarget::NamedExport(next_target.name.to_string()),
                        ))?,
                        models::SymbolVariant::DefaultExport => {
                            self.trace((module_symbol.0.clone(), TraceTarget::DefaultExport))?
                        }
                        models::SymbolVariant::LocalVariable => self.trace((
                            module_symbol.0.clone(),
                            TraceTarget::LocalVar(next_target.name.to_string()),
                        ))?,
                    }
                }
                false => {
                    // used by symbol from other module
                    let other_module =
                        models::Module::retrieve_by_id(&self.db.conn, next_target.module_id)?;
                    match next_target.variant {
                        models::SymbolVariant::NamedExport => self.trace((
                            other_module.path.clone(),
                            TraceTarget::NamedExport(next_target.name.to_string()),
                        ))?,
                        models::SymbolVariant::DefaultExport => {
                            self.trace((other_module.path.clone(), TraceTarget::DefaultExport))?
                        }
                        models::SymbolVariant::LocalVariable => self.trace((
                            other_module.path.clone(),
                            TraceTarget::LocalVar(next_target.name.to_string()),
                        ))?,
                    }
                }
            };
            res.append(&mut paths);
        }

        // append current ModuleSymbol to each path
        for path in res.iter_mut() {
            path.push(module_symbol.clone());
        }
        if self.trace_full_path_only {
            // because we only want to trace the full path, we only need to add a new path
            // when this ModuleSymbol is not using by anyone.
            if res.len() == 0 {
                res.push(vec![module_symbol.clone()]);
            }
        } else {
            // always append the current ModuleSymbol since we want to list every single path
            // that is reachable from the target.
            res.push(vec![module_symbol.clone()]);
        }

        // update cache
        self.cache.insert(module_symbol.clone(), res.clone());

        Ok(res)
    }
}
