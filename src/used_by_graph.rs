use std::collections::HashMap;

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

pub struct UsedByGraph {
    pub modules: HashMap<String, Module>,
}

#[derive(Debug)]
pub struct Module {
    pub local_variable_table: HashMap<String, Option<Vec<UsedBy>>>,
    pub named_export_table: HashMap<String, Option<Vec<UsedBy>>>,
    pub default_export: Option<Vec<UsedBy>>,
}

#[derive(Debug)]
pub enum UsedBy {
    Itself(UsedByType),
    Other(UsedByOther),
}

#[derive(Debug)]
pub struct UsedByOther {
    pub by: String,
    pub by_type: UsedByType,
}

#[derive(Debug)]
pub enum UsedByType {
    NamedExport(String),
    DefaultExport,
    LocalVar(String),
}
