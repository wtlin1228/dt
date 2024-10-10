use super::core;
use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::Module;

pub struct I18nToSymbol {
    pub table: HashMap<String, HashMap<String, HashSet<String>>>,
}

impl I18nToSymbol {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn collect_i18n_usage(
        &mut self,
        module_path: &str,
        module_ast: &Module,
    ) -> anyhow::Result<HashMap<String, HashSet<String>>> {
        let i18n_usage = core::collect_translation(module_ast)?;
        for (symbol, i18n_keys) in i18n_usage.iter() {
            for i18n_key in i18n_keys.iter() {
                if !self.table.contains_key(i18n_key) {
                    self.table.insert(i18n_key.to_owned(), HashMap::new());
                }
                if !self.table.get(i18n_key).unwrap().contains_key(module_path) {
                    self.table
                        .get_mut(i18n_key)
                        .unwrap()
                        .insert(module_path.to_string(), HashSet::new());
                }
                self.table
                    .get_mut(i18n_key)
                    .unwrap()
                    .get_mut(module_path)
                    .unwrap()
                    .insert(symbol.to_owned());
            }
        }
        Ok(i18n_usage)
    }
}
