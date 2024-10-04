use anyhow::Context;
use dt_parser::types::{FromOtherModule, FromType, ModuleExport, SymbolDependency};
use dt_path_resolver::PathResolver;
use std::collections::HashMap;

#[derive(Debug)]
pub struct DependOnGraph {
    pub table: HashMap<String, SymbolDependency>,
    path_resolver: PathResolver,
}

impl DependOnGraph {
    pub fn new(root: &str) -> Self {
        Self {
            table: HashMap::new(),
            path_resolver: PathResolver::new(root),
        }
    }

    fn handle_re_export_star_from(
        &mut self,
        symbol_dependency: &mut SymbolDependency,
    ) -> anyhow::Result<()> {
        if symbol_dependency.re_export_star_from.is_some() {
            let re_export_star_from = symbol_dependency.re_export_star_from.take().unwrap();
            for from_path in re_export_star_from.iter() {
                let resolved_path = self
                    .path_resolver
                    .resolve_path(&symbol_dependency.canonical_path, from_path)
                    .context(format!(
                        "resolve path {} from current path {} failed",
                        from_path, symbol_dependency.canonical_path,
                    ))?;
                let imported_module = self
                    .table
                    .get(&resolved_path)
                    .context(format!("imported module {} not exists", resolved_path))?;
                for (key, _) in imported_module.named_export_table.iter() {
                    assert_eq!(
                        symbol_dependency.named_export_table.contains_key(key),
                        false,
                        "named export {} conflicts",
                        key
                    );
                    symbol_dependency.named_export_table.insert(
                        key.to_string(),
                        ModuleExport::ReExportFrom(FromOtherModule {
                            from: resolved_path.clone(),
                            from_type: FromType::Named(key.to_string()),
                        }),
                    );
                }
            }
        }
        Ok(())
    }

    fn canonicalize_import_paths(
        &mut self,
        symbol_dependency: &mut SymbolDependency,
    ) -> anyhow::Result<()> {
        for (_, value) in symbol_dependency.local_variable_table.iter_mut() {
            match value.import_from {
                Some(ref mut from_other_module) => {
                    match self
                        .path_resolver
                        .resolve_path(&symbol_dependency.canonical_path, &from_other_module.from)
                    {
                        Ok(resolved_path) => from_other_module.from = resolved_path,
                        Err(_) => (),
                    }
                }
                None => (),
            }
        }
        for (_, value) in symbol_dependency.named_export_table.iter_mut() {
            match value {
                ModuleExport::Local(_) => (),
                ModuleExport::ReExportFrom(ref mut from_other_module) => {
                    match self
                        .path_resolver
                        .resolve_path(&symbol_dependency.canonical_path, &from_other_module.from)
                    {
                        Ok(resolved_path) => from_other_module.from = resolved_path,
                        Err(_) => (),
                    }
                }
            }
        }
        if let Some(default_export) = symbol_dependency.default_export.as_mut() {
            match default_export {
                ModuleExport::Local(_) => (),
                ModuleExport::ReExportFrom(ref mut from_other_module) => {
                    match self
                        .path_resolver
                        .resolve_path(&symbol_dependency.canonical_path, &from_other_module.from)
                    {
                        Ok(resolved_path) => from_other_module.from = resolved_path,
                        Err(_) => (),
                    }
                }
            }
        }
        Ok(())
    }

    pub fn add_symbol_dependency(
        &mut self,
        mut symbol_dependency: SymbolDependency,
    ) -> anyhow::Result<()> {
        assert_eq!(
            self.table.contains_key(&symbol_dependency.canonical_path),
            false,
            "can't add the same module twice {}",
            symbol_dependency.canonical_path
        );
        self.canonicalize_import_paths(&mut symbol_dependency)?;
        self.handle_re_export_star_from(&mut symbol_dependency)?;
        self.table.insert(
            symbol_dependency.canonical_path.to_owned(),
            symbol_dependency,
        );
        Ok(())
    }
}
