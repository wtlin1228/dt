use super::depend_on_graph::DependOnGraph;
use dt_parser::{
    anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
    types::{FromOtherModule, FromType, ModuleExport, ModuleScopedVariable},
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::{cmp::Ordering, collections::HashMap};

// local variables can be used by:
// - local variables
//       const name1 = name2; -> Local(name2) is used by Local(name1)
// - named exports
//       export { name2 as name1 } -> Local(name2) is used by NamedExport(name1)
// - default export
//       export default name -> Local(name) is used by DefaultExport
//
// named exports can be used by:
// - local variables of other modules
//       in 'some-module':
//       import { name } from 'this-module' -> NamedExport(name) is used by Local(name) of 'some-module'
//       import { name as name1 } from 'this-module' -> NamedExport(name) is used by Local(name1) of 'some-module'
// - named exports of other modules
//       in 'some-module':
//       export { name } from 'this-module' -> NamedExport(name) is used by NamedExport(name) of 'some-module'
//       export { name as name1 } from 'this-module' -> NamedExport(name) is used by NamedExport(name1) of 'some-module'
//       export * from 'this-module' -> NamedExport(name) is used by NamedExport(name) of 'some-module'
// - default exports of other modules
//       in 'some-module':
//       export { name as default } from 'this-module' -> NamedExport(name) is used by DefaultExport of 'some-module'
//
// default exports can be used by:
// - local variables of other modules
//       in 'some-module':
//       import name from 'this-module' -> DefaultExport is used by Local(name) of 'some-module'
// - named exports of other modules
//       in 'some-module':
//       export { default as name } from 'this-module' -> DefaultExport is used by NamedExport(name) of 'some-module'
// - default exports of other modules
//       in 'some-module':
//       export { default } from 'this-module' -> DefaultExport is used by DefaultExport of 'some-module'

#[derive(Serialize, Deserialize, Debug)]
pub struct UsedByGraph {
    pub modules: HashMap<String, Module>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Module {
    pub local_variable_table: HashMap<String, Option<Vec<UsedBy>>>,
    pub named_export_table: HashMap<String, Option<Vec<UsedBy>>>,
    pub default_export: Option<Vec<UsedBy>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum UsedBy {
    Itself(UsedByType),
    Other(UsedByOther),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq)]
pub struct UsedByOther {
    pub by: String,
    pub by_type: UsedByType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq)]
pub enum UsedByType {
    NamedExport(String),
    DefaultExport,
    LocalVar(String),
}

impl Ord for UsedByOther {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.by == other.by {
            true => self.by_type.cmp(&other.by_type),
            false => self.by.cmp(&other.by),
        }
    }
}

impl Ord for UsedByType {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (UsedByType::NamedExport(a), UsedByType::NamedExport(b)) => a.cmp(b),
            (UsedByType::NamedExport(_), UsedByType::DefaultExport) => Ordering::Greater,
            (UsedByType::NamedExport(_), UsedByType::LocalVar(_)) => Ordering::Greater,
            (UsedByType::DefaultExport, UsedByType::NamedExport(_)) => Ordering::Less,
            (UsedByType::DefaultExport, UsedByType::DefaultExport) => {
                unreachable!("a module can't have two default export")
            }
            (UsedByType::DefaultExport, UsedByType::LocalVar(_)) => Ordering::Greater,
            (UsedByType::LocalVar(_), UsedByType::NamedExport(_)) => Ordering::Less,
            (UsedByType::LocalVar(_), UsedByType::DefaultExport) => Ordering::Less,
            (UsedByType::LocalVar(a), UsedByType::LocalVar(b)) => a.cmp(b),
        }
    }
}

impl UsedByGraph {
    fn new(depend_on_graph: &DependOnGraph) -> Self {
        let mut modules: HashMap<String, Module> = HashMap::new();
        for (module_id, parsed_module) in depend_on_graph.parsed_modules_table.iter() {
            let mut local_variable_table: HashMap<String, Option<Vec<UsedBy>>> = HashMap::new();
            for (symbol_name, _) in parsed_module.local_variable_table.iter() {
                local_variable_table.insert(symbol_name.to_owned(), None);
            }
            let mut named_export_table: HashMap<String, Option<Vec<UsedBy>>> = HashMap::new();
            for (exported_name, _) in parsed_module.named_export_table.iter() {
                named_export_table.insert(exported_name.to_owned(), None);
            }
            modules.insert(
                module_id.to_owned(),
                Module {
                    local_variable_table,
                    named_export_table,
                    default_export: None,
                },
            );
        }
        Self { modules }
    }

    fn add_used_by_to_local_variable(
        &mut self,
        module_id: &str,
        symbol_name: &str,
        used_by: UsedBy,
    ) {
        self.modules
            .entry(module_id.to_owned())
            .and_modify(|module| {
                module
                    .local_variable_table
                    .entry(symbol_name.to_owned())
                    .and_modify(|used_by_list| match used_by_list {
                        Some(used_by_list) => used_by_list.push(used_by.clone()),
                        None => *used_by_list = Some(vec![used_by.clone()]),
                    });
            });
    }

    fn add_used_by_to_named_export(
        &mut self,
        module_id: &str,
        exported_name: &str,
        used_by: UsedBy,
    ) {
        self.modules
            .entry(module_id.to_owned())
            .and_modify(|module| {
                module
                    .named_export_table
                    .entry(exported_name.to_owned())
                    .and_modify(|used_by_list| match used_by_list {
                        Some(used_by_list) => used_by_list.push(used_by.clone()),
                        None => *used_by_list = Some(vec![used_by.clone()]),
                    });
            });
    }

    fn add_used_by_to_default_export(&mut self, module_id: &str, used_by: UsedBy) {
        self.modules
            .entry(module_id.to_owned())
            .and_modify(|module| match module.default_export.as_mut() {
                Some(used_by_list) => used_by_list.push(used_by),
                None => module.default_export = Some(vec![used_by]),
            });
    }

    fn add_used_by_to_all_named_exports(&mut self, module_id: &str, used_by: UsedBy) {
        self.modules
            .entry(module_id.to_owned())
            .and_modify(|module| {
                for (exported_name, used_by_list) in module.named_export_table.iter_mut() {
                    match exported_name == SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT {
                        true => (),
                        false => match used_by_list {
                            Some(used_by_list) => used_by_list.push(used_by.clone()),
                            None => *used_by_list = Some(vec![used_by.clone()]),
                        },
                    }
                }
            });
    }

    pub fn from(depend_on_graph: &DependOnGraph) -> Self {
        let mut used_by_graph = Self::new(depend_on_graph);
        for (module_id, parsed_module) in depend_on_graph.parsed_modules_table.iter() {
            for (
                symbol_name,
                ModuleScopedVariable {
                    depend_on,
                    import_from,
                },
            ) in parsed_module.local_variable_table.iter()
            {
                if let Some(depend_on) = depend_on {
                    let used_by = UsedBy::Itself(UsedByType::LocalVar(symbol_name.to_owned()));
                    for depend_on_name in depend_on.iter() {
                        used_by_graph.add_used_by_to_local_variable(
                            module_id,
                            depend_on_name,
                            used_by.clone(),
                        );
                    }
                }
                if let Some(FromOtherModule { from, from_type }) = import_from {
                    let used_by = UsedBy::Other(UsedByOther {
                        by: module_id.to_owned(),
                        by_type: UsedByType::LocalVar(symbol_name.to_owned()),
                    });
                    match from_type {
                        FromType::Named(exported_name) => {
                            used_by_graph.add_used_by_to_named_export(
                                from,
                                exported_name,
                                used_by.clone(),
                            );
                        }
                        FromType::Default => {
                            used_by_graph.add_used_by_to_default_export(from, used_by.clone());
                        }
                        FromType::Namespace => {
                            used_by_graph.add_used_by_to_all_named_exports(from, used_by.clone());
                        }
                    }
                }
            }
            for (exported_name, module_export) in parsed_module.named_export_table.iter() {
                match module_export {
                    ModuleExport::Local(symbol_name) => {
                        let used_by =
                            UsedBy::Itself(UsedByType::NamedExport(exported_name.to_owned()));
                        used_by_graph.add_used_by_to_local_variable(
                            module_id,
                            symbol_name,
                            used_by.clone(),
                        );
                    }
                    ModuleExport::ReExportFrom(FromOtherModule { from, from_type }) => {
                        let used_by = UsedBy::Other(UsedByOther {
                            by: module_id.to_owned(),
                            by_type: UsedByType::NamedExport(exported_name.to_owned()),
                        });
                        match from_type {
                            FromType::Named(exported_name) => {
                                used_by_graph.add_used_by_to_named_export(
                                    from,
                                    exported_name,
                                    used_by.clone(),
                                );
                            }
                            FromType::Default => {
                                used_by_graph.add_used_by_to_default_export(from, used_by.clone());
                            }
                            FromType::Namespace => {
                                used_by_graph
                                    .add_used_by_to_all_named_exports(from, used_by.clone());
                            }
                        }
                    }
                }
            }
            if let Some(default_export) = parsed_module.default_export.as_ref() {
                match default_export {
                    ModuleExport::Local(symbol_name) => {
                        let used_by = UsedBy::Itself(UsedByType::DefaultExport);
                        used_by_graph.add_used_by_to_local_variable(
                            module_id,
                            symbol_name,
                            used_by.clone(),
                        );
                    }
                    ModuleExport::ReExportFrom(FromOtherModule { from, from_type }) => {
                        let used_by = UsedBy::Other(UsedByOther {
                            by: module_id.to_owned(),
                            by_type: UsedByType::DefaultExport,
                        });
                        match from_type {
                            FromType::Named(exported_name) => {
                                used_by_graph.add_used_by_to_named_export(
                                    from,
                                    exported_name,
                                    used_by.clone(),
                                );
                            }
                            FromType::Default => {
                                used_by_graph.add_used_by_to_default_export(from, used_by.clone());
                            }
                            FromType::Namespace => {
                                unreachable!("can't not export namespace from other module as default export")
                            }
                        }
                    }
                }
            }
        }
        used_by_graph
    }

    pub fn export(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn import(exported: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(exported)?)
    }
}
