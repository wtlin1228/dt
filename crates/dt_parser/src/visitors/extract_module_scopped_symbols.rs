use crate::{
    anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
    to_symbol_name::ToSymbolName,
    types::{FromOtherModule, FromType, ModuleExport, ModuleScopedVariable},
};
use std::collections::{HashMap, HashSet};
use swc_core::ecma::{ast, visit::Visit};

#[derive(Debug)]
pub struct ModuleScoppedSymbolsVisitor {
    pub re_exporting_all_from: Vec<String>,
    pub named_export_table: HashMap<String, ModuleExport>,
    pub default_export: Option<ModuleExport>,

    // variable name = `Id` without `SyntaxContext`
    pub local_variable_table: HashMap<String, ModuleScopedVariable>,

    // Can't just use variable name to build the dependency graph.
    // We need the `SyntaxContext` in `Id.1` when looking into each declaration.
    //
    // ```js
    // let a#1 = 5
    // {
    //     let a#2 = 3;
    // }
    // ```
    //
    // ref: https://rustdoc.swc.rs/swc_core/ecma/ast/struct.Ident.html
    pub tracked_ids: HashSet<ast::Id>,
}

impl ModuleScoppedSymbolsVisitor {
    pub fn new() -> Self {
        Self {
            re_exporting_all_from: vec![],
            named_export_table: HashMap::new(),
            default_export: None,
            local_variable_table: HashMap::new(),
            tracked_ids: HashSet::new(),
        }
    }

    fn track_id(&mut self, ident: &ast::Ident) {
        let id = ident.to_id();
        assert!(
            !self.tracked_ids.contains(&id),
            "It's impossible to track the same id {} twice. There is high possibility that your JS/TS program has bug.",
            id.0
        );
        self.tracked_ids.insert(id);
    }

    fn add_re_exporting_all_from(&mut self, from: String) {
        self.re_exporting_all_from.push(from);
    }

    fn add_re_exporting_all_as_namespace_from(
        &mut self,
        namespace_ident: &ast::Ident,
        from: String,
    ) {
        assert_eq!(
            self.named_export_table
                .contains_key(namespace_ident.to_symbol_name().as_str()),
            false,
            "module can't export the same name twice"
        );
        self.named_export_table.insert(
            namespace_ident.to_symbol_name(),
            ModuleExport::ReExportFrom(FromOtherModule {
                from,
                from_type: FromType::Namespace,
            }),
        );
    }

    fn add_module_scoped_variable(
        &mut self,
        ident: &ast::Ident,
        depend_on: Option<Vec<String>>,
        import_from: Option<FromOtherModule>,
    ) {
        self.local_variable_table.insert(
            ident.to_symbol_name(),
            ModuleScopedVariable {
                depend_on,
                import_from,
            },
        );
    }

    fn named_export_local_var(&mut self, export_ident: &ast::Ident, local_var_ident: &ast::Ident) {
        assert_eq!(
            self.named_export_table
                .contains_key(export_ident.to_symbol_name().as_str()),
            false,
            "module can't export the same name twice"
        );
        self.named_export_table.insert(
            export_ident.to_symbol_name(),
            ModuleExport::Local(local_var_ident.to_symbol_name()),
        );
    }

    fn named_export_from_other_module(
        &mut self,
        export_ident: &ast::Ident,
        original_ident: &ast::Ident,
        from: String,
    ) {
        assert_eq!(
            self.named_export_table
                .contains_key(export_ident.to_symbol_name().as_str()),
            false,
            "module can't export the same name twice"
        );
        self.named_export_table.insert(
            export_ident.to_symbol_name(),
            ModuleExport::ReExportFrom(FromOtherModule {
                from,
                from_type: FromType::Named(original_ident.to_symbol_name()),
            }),
        );
    }

    fn set_default_export(&mut self, module_export: ModuleExport) {
        assert!(
            self.default_export.is_none(),
            "module can't export default twice"
        );
        self.default_export = Some(module_export);
    }
}

impl Visit for ModuleScoppedSymbolsVisitor {
    fn visit_module(&mut self, n: &ast::Module) {
        for module_item in &n.body {
            match module_item {
                ast::ModuleItem::ModuleDecl(module_decl) => match module_decl {
                    ast::ModuleDecl::Import(ast::ImportDecl {
                        specifiers, src, ..
                    }) => {
                        let import_from_path = src.value.as_str();
                        for specifier in specifiers.iter() {
                            match specifier {
                                ast::ImportSpecifier::Named(ast::ImportNamedSpecifier {
                                    local,
                                    imported,
                                    ..
                                }) => match imported {
                                    Some(module_export_name) => match module_export_name {
                                        ast::ModuleExportName::Ident(imported_ident) => {
                                            match imported_ident.to_symbol_name().as_str() {
                                                // import { default as alias1 } from 'module-name';
                                                "default" => {
                                                    self.track_id(local);
                                                    self.add_module_scoped_variable(
                                                        local,
                                                        None,
                                                        Some(FromOtherModule {
                                                            from: import_from_path.to_string(),
                                                            from_type: FromType::Default,
                                                        }),
                                                    );
                                                }
                                                // import { export1 as alias1 } from 'module-name';
                                                _ => {
                                                    self.track_id(local);
                                                    self.add_module_scoped_variable(
                                                        local,
                                                        None,
                                                        Some(FromOtherModule {
                                                            from: import_from_path.to_string(),
                                                            from_type: FromType::Named(
                                                                imported_ident.to_symbol_name(),
                                                            ),
                                                        }),
                                                    );
                                                }
                                            }
                                        }
                                        ast::ModuleExportName::Str(_) => (),
                                    },
                                    // import { export1 } from 'module-name';
                                    None => {
                                        self.track_id(local);
                                        self.add_module_scoped_variable(
                                            local,
                                            None,
                                            Some(FromOtherModule {
                                                from: import_from_path.to_string(),
                                                from_type: FromType::Named(local.to_symbol_name()),
                                            }),
                                        );
                                    }
                                },
                                // import defaultExport from 'module-name';
                                ast::ImportSpecifier::Default(ast::ImportDefaultSpecifier {
                                    local,
                                    ..
                                }) => {
                                    self.track_id(local);
                                    self.add_module_scoped_variable(
                                        local,
                                        None,
                                        Some(FromOtherModule {
                                            from: import_from_path.to_string(),
                                            from_type: FromType::Default,
                                        }),
                                    );
                                }
                                // import * as name from 'module-name';
                                ast::ImportSpecifier::Namespace(ast::ImportStarAsSpecifier {
                                    local,
                                    ..
                                }) => {
                                    self.track_id(local);
                                    self.add_module_scoped_variable(
                                        local,
                                        None,
                                        Some(FromOtherModule {
                                            from: import_from_path.to_string(),
                                            from_type: FromType::Namespace,
                                        }),
                                    );
                                }
                            }
                        }
                    }
                    ast::ModuleDecl::ExportDecl(ast::ExportDecl { decl, .. }) => match decl {
                        // export class ClassName { /* … */ }
                        ast::Decl::Class(ast::ClassDecl { ident, .. }) => {
                            self.track_id(ident);
                            self.add_module_scoped_variable(ident, None, None);
                            self.named_export_local_var(ident, ident);
                        }
                        // export function functionName() { /* … */ }
                        // export function* generatorFunctionName() { /* … */ }
                        ast::Decl::Fn(ast::FnDecl { ident, .. }) => {
                            self.track_id(ident);
                            self.add_module_scoped_variable(ident, None, None);
                            self.named_export_local_var(ident, ident);
                        }
                        ast::Decl::Var(var_decl) => {
                            for decl in var_decl.decls.iter() {
                                match &decl.name {
                                    // export let name1, name2/*, … */; // also var
                                    // export const name1 = 1, name2 = 2/*, … */; // also var, let
                                    ast::Pat::Ident(ast::BindingIdent { id, .. }) => {
                                        self.track_id(id);
                                        self.add_module_scoped_variable(id, None, None);
                                        self.named_export_local_var(id, id);
                                    }
                                    // [Not Support Yet] export const [ name1, name2 ] = array;
                                    ast::Pat::Array(_) => (),
                                    ast::Pat::Rest(_) => (),
                                    // [Not Support Yet] export const { name1, name2: bar } = o;
                                    ast::Pat::Object(_) => (),
                                    ast::Pat::Assign(_) => (),
                                    ast::Pat::Invalid(_) => (),
                                    ast::Pat::Expr(_) => (),
                                }
                            }
                        }
                        ast::Decl::Using(_) => (),
                        ast::Decl::TsInterface(_) => (),
                        ast::Decl::TsTypeAlias(_) => (),
                        ast::Decl::TsEnum(_) => (),
                        ast::Decl::TsModule(_) => (),
                    },
                    ast::ModuleDecl::ExportNamed(ast::NamedExport {
                        specifiers, src, ..
                    }) => {
                        match src {
                            Some(src) => {
                                let import_from_path = &src.value;
                                for specifier in specifiers.iter() {
                                    match specifier {
                                        // export * as name1 from 'module-name';
                                        ast::ExportSpecifier::Namespace(
                                            ast::ExportNamespaceSpecifier { name, .. },
                                        ) => match name {
                                            ast::ModuleExportName::Ident(namespace_ident) => self
                                                .add_re_exporting_all_as_namespace_from(
                                                    namespace_ident,
                                                    import_from_path.to_string(),
                                                ),
                                            ast::ModuleExportName::Str(_) => (),
                                        },
                                        ast::ExportSpecifier::Default(_) => (),
                                        ast::ExportSpecifier::Named(
                                            ast::ExportNamedSpecifier { orig, exported, .. },
                                        ) => match (orig, exported) {
                                            // export { name1, /* …, */ nameN } from 'module-name';
                                            // export { default, /* …, */ } from 'module-name';
                                            (ast::ModuleExportName::Ident(ident), None) => {
                                                match ident.to_symbol_name().as_str() {
                                                    "default" => {
                                                        assert!(self.default_export.is_none());
                                                        self.default_export =
                                                            Some(ModuleExport::ReExportFrom(
                                                                FromOtherModule {
                                                                    from: import_from_path
                                                                        .to_string(),
                                                                    from_type: FromType::Default,
                                                                },
                                                            ))
                                                    }
                                                    _ => self.named_export_from_other_module(
                                                        ident,
                                                        ident,
                                                        import_from_path.to_string(),
                                                    ),
                                                }
                                            }
                                            // export { import1 as name1, import2 as name2, /* …, */ importN as nameN } from 'module-name';
                                            // export { default as name1 } from 'module-name';
                                            (
                                                ast::ModuleExportName::Ident(orig_ident),
                                                Some(ast::ModuleExportName::Ident(export_ident)),
                                            ) => match orig_ident.to_symbol_name().as_str() {
                                                "default" => {
                                                    assert_eq!(
                                                        self.named_export_table.contains_key(
                                                            export_ident.to_symbol_name().as_str()
                                                        ),
                                                        false,
                                                        "module can't export the same name twice"
                                                    );
                                                    self.named_export_table.insert(
                                                        export_ident.to_symbol_name(),
                                                        ModuleExport::ReExportFrom(
                                                            FromOtherModule {
                                                                from: import_from_path.to_string(),
                                                                from_type: FromType::Default,
                                                            },
                                                        ),
                                                    );
                                                }
                                                _ => self.named_export_from_other_module(
                                                    export_ident,
                                                    orig_ident,
                                                    import_from_path.to_string(),
                                                ),
                                            },
                                            (_, _) => (),
                                        },
                                    }
                                }
                            }
                            None => {
                                for specifier in specifiers.iter() {
                                    match specifier {
                                        ast::ExportSpecifier::Namespace(_) => (),
                                        ast::ExportSpecifier::Default(_) => (),
                                        ast::ExportSpecifier::Named(
                                            ast::ExportNamedSpecifier { orig, exported, .. },
                                        ) => {
                                            match (orig, exported) {
                                                // export { name1, /* …, */ nameN };
                                                (ast::ModuleExportName::Ident(ident), None) => {
                                                    self.named_export_local_var(ident, ident);
                                                }
                                                // export { variable1 as name1, variable2 as name2, /* …, */ variableN as nameN };
                                                // export { name1 as default /*, … */ };
                                                (
                                                    ast::ModuleExportName::Ident(orig_ident),
                                                    Some(ast::ModuleExportName::Ident(
                                                        export_ident,
                                                    )),
                                                ) => {
                                                    match export_ident.to_symbol_name().as_str() {
                                                        "default" => self.set_default_export(
                                                            ModuleExport::Local(
                                                                orig_ident.to_symbol_name(),
                                                            ),
                                                        ),
                                                        _ => self.named_export_local_var(
                                                            export_ident,
                                                            orig_ident,
                                                        ),
                                                    };
                                                }
                                                // [Not Support Yet] export { variable1 as 'string name' };
                                                (_, _) => (),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ast::ModuleDecl::ExportDefaultDecl(ast::ExportDefaultDecl { decl, .. }) => {
                        match decl {
                            ast::DefaultDecl::Class(ast::ClassExpr { ident, .. }) => match ident {
                                // export default class ClassName { /* … */ }
                                Some(ident) => {
                                    self.track_id(ident);
                                    self.add_module_scoped_variable(ident, None, None);
                                    self.set_default_export(ModuleExport::Local(
                                        ident.to_symbol_name(),
                                    ))
                                }
                                // export default class { /* … */ }
                                // Use `SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT` for anonymous case.
                                // This symbol name shouldn't be tracked since this symbol cannot be used else where
                                // in this module.
                                None => {
                                    self.local_variable_table.insert(
                                        SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                        ModuleScopedVariable {
                                            depend_on: None,
                                            import_from: None,
                                        },
                                    );
                                    self.set_default_export(ModuleExport::Local(
                                        SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                    ))
                                }
                            },
                            ast::DefaultDecl::Fn(ast::FnExpr { ident, .. }) => match ident {
                                // export default function functionName() { /* … */ }
                                Some(ident) => {
                                    self.track_id(ident);
                                    self.add_module_scoped_variable(ident, None, None);
                                    self.set_default_export(ModuleExport::Local(
                                        ident.to_symbol_name(),
                                    ))
                                }
                                // export default function () { /* … */ }
                                // Use `SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT` for anonymous case.
                                // This symbol name shouldn't be tracked since this symbol cannot be used else where
                                // in this module.
                                None => {
                                    self.local_variable_table.insert(
                                        SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                        ModuleScopedVariable {
                                            depend_on: None,
                                            import_from: None,
                                        },
                                    );
                                    self.set_default_export(ModuleExport::Local(
                                        SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                    ))
                                }
                            },
                            ast::DefaultDecl::TsInterfaceDecl(_) => (),
                        }
                    }
                    ast::ModuleDecl::ExportDefaultExpr(ast::ExportDefaultExpr { expr, .. }) => {
                        match &**expr {
                            ast::Expr::Ident(ident) => {
                                self.set_default_export(ModuleExport::Local(ident.to_symbol_name()))
                            }
                            // export default [name1, name2, /* …, */ nameN];
                            // Use `SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT` for anonymous case.
                            // This symbol name shouldn't be tracked since this symbol cannot be used else where
                            // in this module.
                            ast::Expr::Array(_) => {
                                self.local_variable_table.insert(
                                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                    ModuleScopedVariable {
                                        depend_on: None,
                                        import_from: None,
                                    },
                                );
                                self.set_default_export(ModuleExport::Local(
                                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                ));
                            }
                            // export default { name1, name2, /* …, */ nameN };
                            // Use `SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT` for anonymous case.
                            // This symbol name shouldn't be tracked since this symbol cannot be used else where
                            // in this module.
                            ast::Expr::Object(_) => {
                                self.local_variable_table.insert(
                                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                    ModuleScopedVariable {
                                        depend_on: None,
                                        import_from: None,
                                    },
                                );
                                self.set_default_export(ModuleExport::Local(
                                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                ));
                            }
                            // export default () => { /* … */ };
                            // Use `SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT` for anonymous case.
                            // This symbol name shouldn't be tracked since this symbol cannot be used else where
                            // in this module.
                            ast::Expr::Arrow(_) => {
                                self.local_variable_table.insert(
                                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                    ModuleScopedVariable {
                                        depend_on: None,
                                        import_from: None,
                                    },
                                );
                                self.set_default_export(ModuleExport::Local(
                                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                ));
                            }
                            _ => (),
                        }
                    }
                    ast::ModuleDecl::ExportAll(ast::ExportAll { src, .. }) => {
                        self.add_re_exporting_all_from(src.value.to_string());
                    }
                    ast::ModuleDecl::TsImportEquals(_) => (),
                    ast::ModuleDecl::TsExportAssignment(_) => (),
                    ast::ModuleDecl::TsNamespaceExport(_) => (),
                },
                ast::ModuleItem::Stmt(stmt) => match stmt {
                    ast::Stmt::Decl(decl) => match decl {
                        // class name { /* … */ }
                        // class name extends otherName { /* … */ }
                        ast::Decl::Class(ast::ClassDecl { ident, .. }) => {
                            self.track_id(ident);
                            self.add_module_scoped_variable(ident, None, None);
                        }
                        // function name(param0) { /* … */ }
                        // function* name(param0) { /* … */ }
                        // async function name(param0) { /* … */ }
                        // async function* name(param0) { /* … */ }
                        ast::Decl::Fn(ast::FnDecl { ident, .. }) => {
                            self.track_id(ident);
                            self.add_module_scoped_variable(ident, None, None);
                        }
                        ast::Decl::Var(var_decl) => {
                            for decl in var_decl.decls.iter() {
                                match &decl.name {
                                    // let name1;
                                    // let name1 = value1;
                                    // let name1 = value1, name2 = value2;
                                    // let name1, name2 = value2;
                                    // let name1 = value1, name2, /* …, */ nameN = valueN;
                                    // const name1 = value1;
                                    // const name1 = value1, name2 = value2;
                                    // const name1 = value1, name2 = value2, /* …, */ nameN = valueN;
                                    ast::Pat::Ident(ast::BindingIdent { id, .. }) => {
                                        self.track_id(id);
                                        self.add_module_scoped_variable(id, None, None);
                                    }
                                    ast::Pat::Array(_) => (),
                                    ast::Pat::Rest(_) => (),
                                    ast::Pat::Object(_) => (),
                                    ast::Pat::Assign(_) => (),
                                    ast::Pat::Invalid(_) => (),
                                    ast::Pat::Expr(_) => (),
                                }
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dt_test_utils::{assert_hash_map, parse_module};
    use swc_core::ecma::visit::VisitWith;

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

    #[test]
    fn test_statements_are_handled() {
        let inputs = vec![
            // Exporting declarations
            r#"export let name1, name2/*, … */; // also var"#,
            r#"export const name1 = 1, name2 = 2/*, … */; // also var, let"#,
            r#"export function functionName() { /* … */ }"#,
            r#"export class ClassName { /* … */ }"#,
            r#"export function* generatorFunctionName() { /* … */ }"#,
            r#"export const { name1, name2: bar } = o;"#, // Not Support Yet
            r#"export const [ name1, name2 ] = array;"#,  // Not Support Yet
            // Export list
            r#"export { name1, /* …, */ nameN };"#,
            r#"export { variable1 as name1, variable2 as name2, /* …, */ variableN as nameN };"#,
            r#"export { variable1 as 'string name' };"#, // Not Support Yet
            r#"export { name1 as default /*, … */ };"#,
            // Default exports
            r#"export default expression;"#,
            r#"export default [name1, name2, /* …, */ nameN];"#,
            r#"export default { name1, name2, /* …, */ nameN };"#,
            r#"export default () => { /* … */ };"#,
            r#"export default function functionName() { /* … */ }"#,
            r#"export default class ClassName { /* … */ }"#,
            r#"export default function* generatorFunctionName() { /* … */ }"#,
            r#"export default function () { /* … */ }"#,
            r#"export default class { /* … */ }"#,
            r#"export default function* () { /* … */ }"#,
            // Aggregating modules
            r#"export * from 'module-name';"#,
            r#"export * as name1 from 'module-name';"#,
            r#"export { name1, /* …, */ nameN } from 'module-name';"#,
            r#"export { import1 as name1, import2 as name2, /* …, */ variableN as nameN } from 'module-name';"#,
            r#"export { default, /* …, */ } from 'module-name';"#,
            r#"export { default as name1 } from 'module-name';"#,
            // Imports
            r#"import defaultExport from 'module-name';"#,
            r#"import * as name from 'module-name';"#,
            r#"import { export1 } from 'module-name';"#,
            r#"import { export1 as alias1 } from 'module-name';"#,
            r#"import { default as alias } from 'module-name';"#,
            r#"import { export1, export2 } from 'module-name';"#,
            r#"import { export1, export2 as alias2, /* … */ } from 'module-name';"#,
            r#"import { 'string name' as alias } from 'module-name';"#, // Not Support Yet
            r#"import defaultExport, { export1, /* … */ } from 'module-name';"#,
            r#"import defaultExport, * as name from 'module-name';"#,
            r#"import 'module-name';"#,
            // Declaring variables
            r#"let name1;"#,
            r#"let name1 = value1;"#,
            r#"let name1 = value1, name2 = value2;"#,
            r#"let name1, name2 = value2;"#,
            r#"let name1 = value1, name2, /* …, */ nameN = valueN;"#,
            r#"const name1 = value1;"#,
            r#"const name1 = value1, name2 = value2;"#,
            r#"const name1 = value1, name2 = value2, /* …, */ nameN = valueN;"#,
            // Functions and classes
            r#"function name(param0) { /* … */ }"#,
            r#"function* name(param0) { /* … */ }"#,
            r#"async function name(param0) { /* … */ }"#,
            r#"async function* name(param0) { /* … */ }"#,
            r#"class name { /* … */ }"#,
            r#"class name extends otherName { /* … */ }"#,
        ];
        inputs.iter().for_each(|&input| {
            let mut visitor = ModuleScoppedSymbolsVisitor::new();
            let module = parse_module(input).unwrap();
            module.visit_with(&mut visitor);
        });
    }

    #[test]
    fn test_exporting_declaration_let() {
        let input = r#"export let name1, name2/*, … */; // also var"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            ("name1", ModuleExport::Local(String::from("name1"))),
            ("name2", ModuleExport::Local(String::from("name2"))),
        );
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "name2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1", "name2"]);
    }

    #[test]
    fn test_exporting_declaration_const() {
        let input = r#"export const name1 = 1, name2 = 2/*, … */; // also var, let"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            ("name1", ModuleExport::Local(String::from("name1"))),
            ("name2", ModuleExport::Local(String::from("name2"))),
        );
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "name2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1", "name2"]);
    }

    #[test]
    fn test_exporting_declaration_function() {
        let input = r#"export function functionName() { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            (
                "functionName",
                ModuleExport::Local(String::from("functionName"))
            ),
        );
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "functionName",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["functionName"]);
    }

    #[test]
    fn test_exporting_declaration_class() {
        let input = r#"export class ClassName { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            ("ClassName", ModuleExport::Local(String::from("ClassName"))),
        );
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "ClassName",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["ClassName"]);
    }

    #[test]
    fn test_exporting_declaration_generator() {
        let input = r#"export function* generatorFunctionName() { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            (
                "generatorFunctionName",
                ModuleExport::Local(String::from("generatorFunctionName"))
            ),
        );
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "generatorFunctionName",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["generatorFunctionName"]);
    }

    #[test]
    fn test_exporting_list_named() {
        let input = r#"export { name1, /* …, */ nameN };"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            ("name1", ModuleExport::Local(String::from("name1"))),
            ("nameN", ModuleExport::Local(String::from("nameN"))),
        );
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_list_named_alias() {
        let input =
            r#"export { variable1 as name1, variable2 as name2, /* …, */ variableN as nameN };"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            ("name1", ModuleExport::Local(String::from("variable1"))),
            ("name2", ModuleExport::Local(String::from("variable2"))),
            ("nameN", ModuleExport::Local(String::from("variableN"))),
        );
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_list_default() {
        let input = r#"export { name1 as default /*, … */ };"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(String::from("name1")))
        );
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_default_expression() {
        let input = r#"export default expression;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(String::from("expression")))
        );
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_default_array() {
        let input = r#"export default [name1, name2, /* …, */ nameN];"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_default_object() {
        let input = r#"export default { name1, name2, /* …, */ nameN };"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_default_arrow_function() {
        let input = r#"export default () => { /* … */ };"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_default_function() {
        let input = r#"export default function functionName() { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(String::from("functionName")))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "functionName",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["functionName"]);
    }

    #[test]
    fn test_exporting_default_class() {
        let input = r#"export default class ClassName { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(String::from("ClassName")))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "ClassName",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["ClassName"]);
    }

    #[test]
    fn test_exporting_default_generator_function() {
        let input = r#"export default function* generatorFunctionName() { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(String::from("generatorFunctionName")))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "generatorFunctionName",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["generatorFunctionName"]);
    }

    #[test]
    fn test_exporting_default_anonymous_function() {
        let input = r#"export default function () { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_default_anonymous_class() {
        let input = r#"export default class { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_exporting_default_anonymous_generator_function() {
        let input = r#"export default function* () { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert_hash_map!(
            visitor.local_variable_table,
            (
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_aggregating_modules_re_export_all_from_other_module() {
        let input = r#"export * from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from, ["module-name"]);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_aggregating_modules_re_export_all_as_namespace_from_other_module() {
        let input = r#"export * as name1 from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            (
                "name1",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("module-name"),
                    from_type: FromType::Namespace
                })
            ),
        );
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_aggregating_modules_re_export_named_from_other_module() {
        let input = r#"export { name1, /* …, */ nameN } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            (
                "name1",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("module-name"),
                    from_type: FromType::Named(String::from("name1"))
                })
            ),
            (
                "nameN",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("module-name"),
                    from_type: FromType::Named(String::from("nameN"))
                })
            ),
        );
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_aggregating_modules_re_export_named_alias_from_other_module() {
        let input = r#"export { import1 as name1, import2 as name2, /* …, */ importN as nameN } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            (
                "name1",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("module-name"),
                    from_type: FromType::Named(String::from("import1"))
                })
            ),
            (
                "name2",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("module-name"),
                    from_type: FromType::Named(String::from("import2"))
                })
            ),
            (
                "nameN",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("module-name"),
                    from_type: FromType::Named(String::from("importN"))
                })
            ),
        );
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_aggregating_modules_re_export_default_from_other_module() {
        let input = r#"export { default, /* …, */ } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert_eq!(
            visitor.default_export,
            Some(ModuleExport::ReExportFrom(FromOtherModule {
                from: String::from("module-name"),
                from_type: FromType::Default
            }))
        );
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_aggregating_modules_re_export_default_as_named_from_other_module() {
        let input = r#"export { default as name1 } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_hash_map!(
            visitor.named_export_table,
            (
                "name1",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("module-name"),
                    from_type: FromType::Default
                })
            ),
        );
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_import_default() {
        let input = r#"import defaultExport from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "defaultExport",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Default
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["defaultExport"]);
    }

    #[test]
    fn test_import_namespace() {
        let input = r#"import * as name from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Namespace
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name"]);
    }

    #[test]
    fn test_import_named() {
        let input = r#"import { export1 } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "export1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Named(String::from("export1"))
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["export1"]);
    }

    #[test]
    fn test_import_named_alias() {
        let input = r#"import { export1 as alias1 } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "alias1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Named(String::from("export1"))
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["alias1"]);
    }

    #[test]
    fn test_import_default_alias() {
        let input = r#"import { default as alias } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "alias",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Default
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["alias"]);
    }

    #[test]
    fn test_import_named_multiple() {
        let input = r#"import { export1, export2 } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "export1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Named(String::from("export1"))
                    })
                }
            ),
            (
                "export2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Named(String::from("export2"))
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["export1", "export2"]);
    }

    #[test]
    fn test_import_named_alias_multiple() {
        let input = r#"import { export1, export2 as alias2, /* … */ } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "export1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Named(String::from("export1"))
                    })
                }
            ),
            (
                "alias2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Named(String::from("export2"))
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["export1", "alias2"]);
    }

    #[test]
    fn test_import_named_default_multiple() {
        let input = r#"import defaultExport, { export1, /* … */ } from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "defaultExport",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Default
                    })
                }
            ),
            (
                "export1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Named(String::from("export1"))
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["defaultExport", "export1"]);
    }

    #[test]
    fn test_import_default_namespace_multiple() {
        let input = r#"import defaultExport, * as name from 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "defaultExport",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Default
                    })
                }
            ),
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("module-name"),
                        from_type: FromType::Namespace
                    })
                }
            ),
        );
        assert_tracked_ids!(visitor, ["defaultExport", "name"]);
    }

    #[test]
    fn test_import_for_side_effect() {
        let input = r#"import 'module-name';"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_declaring_variable_let() {
        let input = r#"let name1;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1"]);
    }

    #[test]
    fn test_declaring_variable_let_with_init() {
        let input = r#"let name1 = value1;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1"]);
    }

    #[test]
    fn test_declaring_variable_let_with_init_multiple() {
        let input = r#"let name1 = value1, name2 = value2;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "name2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1", "name2"]);
    }

    #[test]
    fn test_declaring_variable_let_with_init_multiple_combined() {
        let input = r#"let name1, name2 = value2;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "name2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1", "name2"]);
    }

    #[test]
    fn test_declaring_variable_let_multiple_with_comment() {
        let input = r#"let name1 = value1, name2, /* …, */ nameN = valueN;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "name2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "nameN",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1", "name2", "nameN"]);
    }

    #[test]
    fn test_declaring_variable_const_with_init() {
        let input = r#"const name1 = value1;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1"]);
    }

    #[test]
    fn test_declaring_variable_const_with_init_multiple() {
        let input = r#"const name1 = value1, name2 = value2;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "name2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1", "name2"]);
    }

    #[test]
    fn test_declaring_variable_const_with_init_multiple_with_comment() {
        let input = r#"const name1 = value1, name2 = value2, /* …, */ nameN = valueN;"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name1",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "name2",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "nameN",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name1", "name2", "nameN"]);
    }

    #[test]
    fn test_declaring_function() {
        let input = r#"function name(param0) { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name"]);
    }

    #[test]
    fn test_declaring_generator_function() {
        let input = r#"function* name(param0) { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name"]);
    }

    #[test]
    fn test_declaring_async_function() {
        let input = r#"async function name(param0) { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name"]);
    }

    #[test]
    fn test_declaring_async_generator_function() {
        let input = r#"async function* name(param0) { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name"]);
    }

    #[test]
    fn test_declaring_class() {
        let input = r#"class name { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name"]);
    }

    #[test]
    fn test_declaring_class_extend() {
        let input = r#"class name extends otherName { /* … */ }"#;
        let mut visitor = ModuleScoppedSymbolsVisitor::new();
        let module = parse_module(input).unwrap();
        module.visit_with(&mut visitor);

        assert_eq!(visitor.re_exporting_all_from.len(), 0);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_hash_map!(
            visitor.local_variable_table,
            (
                "name",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
        );
        assert_tracked_ids!(visitor, ["name"]);
    }
}
