use anyhow::Context;
use std::collections::HashMap;

use super::{
    common::{FromOtherModule, FromType, ModuleExport, ParsedModule},
    path_resolver::ResolvePath,
};

#[derive(Debug)]
pub struct DependencyTracker {
    pub parsed_modules_table: HashMap<String, ParsedModule>,
}

impl DependencyTracker {
    pub fn new() -> Self {
        Self {
            parsed_modules_table: HashMap::new(),
        }
    }

    fn handle_re_export_star_from(
        &mut self,
        parsed_module: &mut ParsedModule,
        path_resolver: &impl ResolvePath,
    ) -> anyhow::Result<()> {
        if parsed_module.re_export_star_from.is_some() {
            let re_export_star_from = parsed_module.re_export_star_from.take().unwrap();
            for from_path in re_export_star_from.iter() {
                let resolved_path = path_resolver
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
        path_resolver: &impl ResolvePath,
    ) -> anyhow::Result<()> {
        for (_, value) in parsed_module.local_variable_table.iter_mut() {
            match value.import_from {
                Some(ref mut from_other_module) => {
                    match path_resolver
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
                    match path_resolver
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

    pub fn add_parsed_module(
        &mut self,
        mut parsed_module: ParsedModule,
        path_resolver: &impl ResolvePath,
    ) -> anyhow::Result<()> {
        assert_eq!(
            self.parsed_modules_table
                .contains_key(&parsed_module.canonical_path),
            false,
            "can't add the same module twice {}",
            parsed_module.canonical_path
        );
        self.canonicalize_import_paths(&mut parsed_module, path_resolver)?;
        self.handle_re_export_star_from(&mut parsed_module, path_resolver)?;
        self.parsed_modules_table
            .insert(parsed_module.canonical_path.to_owned(), parsed_module);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::{assert_hash_map, common::ModuleScopedVariable};

    struct SimplePathResolver;
    impl ResolvePath for SimplePathResolver {
        fn resolve_path(&self, _current_path: &str, import_src: &str) -> anyhow::Result<String> {
            Ok(import_src.to_string())
        }
    }

    #[test]
    fn it_works() {
        let mut dt = DependencyTracker::new();
        let path_resolver = SimplePathResolver;
        let hawk = ParsedModule {
            canonical_path: String::from("src/hawk"),
            local_variable_table: HashMap::from([(
                String::from("RedDemon"),
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("src/red-demon"),
                        from_type: FromType::Default,
                    }),
                },
            )]),
            named_export_table: HashMap::from([
                (
                    String::from("HawkRedDemon"),
                    ModuleExport::Local(String::from("RedDemon")),
                ),
                (
                    String::from("HawkGreyDemon"),
                    ModuleExport::ReExportFrom(FromOtherModule {
                        from: String::from("src/grey-demon"),
                        from_type: FromType::Named(String::from("GreyDemon")),
                    }),
                ),
            ]),
            default_export: None,
            re_export_star_from: None,
        };
        dt.add_parsed_module(hawk, &path_resolver).unwrap();
        assert_eq!(dt.parsed_modules_table.len(), 1);
        let hawk = dt.parsed_modules_table.get("src/hawk").unwrap();
        assert_hash_map!(
            hawk.local_variable_table,
            (
                "RedDemon",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("src/red-demon"),
                        from_type: FromType::Default,
                    }),
                }
            ),
        );
        assert_hash_map!(
            hawk.named_export_table,
            (
                "HawkRedDemon",
                ModuleExport::Local(String::from("RedDemon"))
            ),
            (
                "HawkGreyDemon",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("src/grey-demon"),
                    from_type: FromType::Named(String::from("GreyDemon"))
                })
            )
        );

        let kirby = ParsedModule {
            canonical_path: String::from("src/kirby"),
            local_variable_table: HashMap::new(),
            named_export_table: HashMap::new(),
            default_export: None,
            re_export_star_from: Some(vec![String::from("src/hawk")]),
        };
        dt.add_parsed_module(kirby, &path_resolver).unwrap();
        assert_eq!(dt.parsed_modules_table.len(), 2);
        let kirby = dt.parsed_modules_table.get("src/kirby").unwrap();
        assert_eq!(kirby.local_variable_table.len(), 0);
        assert_eq!(kirby.re_export_star_from, None);
        assert_hash_map!(
            kirby.named_export_table,
            (
                "HawkRedDemon",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("src/hawk"),
                    from_type: FromType::Named(String::from("HawkRedDemon"))
                })
            ),
            (
                "HawkGreyDemon",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("src/hawk"),
                    from_type: FromType::Named(String::from("HawkGreyDemon"))
                })
            )
        );
    }
}
