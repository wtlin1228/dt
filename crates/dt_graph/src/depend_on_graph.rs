use anyhow::Context;
use dt_parser::types::{FromOtherModule, FromType, ModuleExport, ParsedModule};
use dt_path_resolver::PathResolver;
use std::collections::HashMap;

#[derive(Debug)]
pub struct DependOnGraph {
    pub parsed_modules_table: HashMap<String, ParsedModule>,
    path_resolver: PathResolver,
}

impl DependOnGraph {
    pub fn new(root: &str) -> Self {
        Self {
            parsed_modules_table: HashMap::new(),
            path_resolver: PathResolver::new(root),
        }
    }

    fn handle_re_export_star_from(
        &mut self,
        parsed_module: &mut ParsedModule,
    ) -> anyhow::Result<()> {
        if parsed_module.re_export_star_from.is_some() {
            let re_export_star_from = parsed_module.re_export_star_from.take().unwrap();
            for from_path in re_export_star_from.iter() {
                let resolved_path = self
                    .path_resolver
                    .resolve_path(&parsed_module.canonical_path, from_path)
                    .context(format!(
                        "resolve path {} from current path {} failed",
                        from_path, parsed_module.canonical_path,
                    ))?;
                let imported_module = self
                    .parsed_modules_table
                    .get(&resolved_path)
                    .context(format!("imported module {} not exists", resolved_path))?;
                for (key, _) in imported_module.named_export_table.iter() {
                    assert_eq!(
                        parsed_module.named_export_table.contains_key(key),
                        false,
                        "named export {} conflicts",
                        key
                    );
                    parsed_module.named_export_table.insert(
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
        parsed_module: &mut ParsedModule,
    ) -> anyhow::Result<()> {
        for (_, value) in parsed_module.local_variable_table.iter_mut() {
            match value.import_from {
                Some(ref mut from_other_module) => {
                    match self
                        .path_resolver
                        .resolve_path(&parsed_module.canonical_path, &from_other_module.from)
                    {
                        Ok(resolved_path) => from_other_module.from = resolved_path,
                        Err(_) => (),
                    }
                }
                None => (),
            }
        }
        for (_, value) in parsed_module.named_export_table.iter_mut() {
            match value {
                ModuleExport::Local(_) => (),
                ModuleExport::ReExportFrom(ref mut from_other_module) => {
                    match self
                        .path_resolver
                        .resolve_path(&parsed_module.canonical_path, &from_other_module.from)
                    {
                        Ok(resolved_path) => from_other_module.from = resolved_path,
                        Err(_) => (),
                    }
                }
            }
        }
        Ok(())
    }

    pub fn add_parsed_module(&mut self, mut parsed_module: ParsedModule) -> anyhow::Result<()> {
        assert_eq!(
            self.parsed_modules_table
                .contains_key(&parsed_module.canonical_path),
            false,
            "can't add the same module twice {}",
            parsed_module.canonical_path
        );
        self.canonicalize_import_paths(&mut parsed_module)?;
        self.handle_re_export_star_from(&mut parsed_module)?;
        self.parsed_modules_table
            .insert(parsed_module.canonical_path.to_owned(), parsed_module);
        Ok(())
    }
}
