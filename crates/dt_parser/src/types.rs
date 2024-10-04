use std::collections::HashMap;

#[derive(Debug)]
pub struct SymbolDependency {
    pub canonical_path: String,
    pub local_variable_table: HashMap<String, ModuleScopedVariable>,
    pub named_export_table: HashMap<String, ModuleExport>,
    pub default_export: Option<ModuleExport>,
    pub re_export_star_from: Option<Vec<String>>,
}

#[derive(Debug, PartialEq)]
pub struct ModuleScopedVariable {
    pub depend_on: Option<Vec<String>>,
    pub import_from: Option<FromOtherModule>,
}

#[derive(Debug, PartialEq)]
pub enum ModuleExport {
    Local(String),
    ReExportFrom(FromOtherModule),
}

#[derive(Debug, PartialEq)]
pub struct FromOtherModule {
    pub from: String,
    pub from_type: FromType,
}

#[derive(Debug, PartialEq)]
pub enum FromType {
    // Used in those cases:
    // - import { A } from 'some-module'
    // - import { a as A } from 'some-module'
    // - export { A } from 'some-module'
    // - export { a as A } from 'some-module'
    Named(String),

    // Used in those cases:
    // - import A from 'some-module'
    // - export { default as A } from 'some-module'
    // - export { default } from 'some-module'
    Default,

    // Used in those cases:
    // - import * as A from 'some-module'
    // - export * as A from 'some-module'
    Namespace,
}
