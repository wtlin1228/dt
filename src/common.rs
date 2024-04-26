use std::collections::HashMap;

#[derive(Debug)]
pub struct ParsedModule {
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

#[cfg(test)]
#[macro_export]
macro_rules! assert_hash_map {
    ($hash_map:expr, $(($key:expr, $value:expr)),*) => {{
        let mut count = 0;
        $(
            count += 1;
            assert_eq!($hash_map.get($key).unwrap(), &$value);
        )*
        assert_eq!($hash_map.len(), count);
    }};

    ($hash_map:expr, $(($key:expr, $value:expr),)*) => {{
        $crate::assert_hash_map!($hash_map, $(($key, $value)),*)
    }};
}

#[cfg(test)]
#[macro_export]
macro_rules! assert_tracked_ids {
    ($visitor:expr, $expect:expr) => {{
        let mut tracked_ids: Vec<&str> = $visitor
            .tracked_ids
            .iter()
            .map(|(atom, _)| atom.as_str())
            .collect();
        tracked_ids.sort();
        let mut expect = $expect;
        expect.sort();
        assert_eq!(tracked_ids, expect);
    }};
}
