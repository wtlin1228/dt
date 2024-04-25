use super::{
    types::{FromOtherModule, FromType, ModuleExport, ModuleScopedVariable},
    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
};

use std::collections::{HashMap, HashSet};
use swc_core::ecma::{ast, visit::Visit};

trait ToSymbolName {
    fn to_symbol_name(&self) -> String;
}

impl ToSymbolName for ast::Ident {
    fn to_symbol_name(&self) -> String {
        self.to_id().0.to_string()
    }
}

#[derive(Debug)]
pub struct ModuleSymbolVisitor {
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

impl ModuleSymbolVisitor {
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

impl Visit for ModuleSymbolVisitor {
    fn visit_module(&mut self, n: &ast::Module) {
        println!("{:#?}", n);
        for module_item in &n.body {
            match module_item {
                ast::ModuleItem::ModuleDecl(module_decl) => match module_decl {
                    ast::ModuleDecl::Import(_) => todo!(),
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
                        ast::Decl::Using(_) => todo!(),
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
                                            ast::ModuleExportName::Str(_) => todo!(),
                                        },
                                        ast::ExportSpecifier::Default(_) => todo!(),
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
                                        ast::ExportSpecifier::Namespace(_) => todo!(),
                                        ast::ExportSpecifier::Default(_) => todo!(),
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
                                                (_, _) => todo!(),
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
                ast::ModuleItem::Stmt(_) => todo!(),
            }
        }
    }
}

#[cfg(test)]
macro_rules! parse_with_visitor {
    ($input:expr, $visitor:expr) => {
        let cm: Lrc<SourceMap> = Default::default();
        let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

        let fm = cm.new_source_file(FileName::Custom("test.js".into()), $input.into());

        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: false,
                dts: false,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: true,
            }),
            Default::default(),
            StringInput::from(&*fm),
            None,
        );

        let mut parser = Parser::new_from(lexer);

        for e in parser.take_errors() {
            e.into_diagnostic(&handler).emit();
        }

        let module = parser
            .parse_module()
            .map_err(|e| {
                // Unrecoverable fatal error occurred
                e.into_diagnostic(&handler).emit()
            })
            .expect("failed to parse module");

        GLOBALS.set(&Globals::new(), || {
            let module = module.fold_with(&mut resolver(Mark::new(), Mark::new(), true));
            module.visit_with(&mut $visitor);
        });
    };
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
macro_rules! assert_tracked_ids {
    ($visitor:expr, $expect:expr) => {
        let mut tracked_ids: Vec<&str> = $visitor
            .tracked_ids
            .iter()
            .map(|(atom, _)| atom.as_str())
            .collect();
        tracked_ids.sort();
        $expect.sort();
        assert_eq!(tracked_ids, $expect);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    use swc_core::{
        common::{
            errors::{ColorConfig, Handler},
            sync::Lrc,
            FileName, Globals, Mark, SourceMap, GLOBALS,
        },
        ecma::{
            transforms::base::resolver,
            visit::{FoldWith, VisitWith},
        },
    };
    use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

    #[ignore]
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
            //  Imports
            r#"import defaultExport from 'module-name';"#,
            r#"import * as name from 'module-name';"#,
            r#"import { export1 } from 'module-name';"#,
            r#"import { export1 as alias1 } from 'module-name';"#,
            r#"import { default as alias } from 'module-name';"#,
            r#"import { export1, export2 } from 'module-name';"#,
            r#"import { export1, export2 as alias2, /* … */ } from 'module-name';"#,
            r#"import { 'string name' as alias } from 'module-name';"#,
            r#"import defaultExport, { export1, /* … */ } from 'module-name';"#,
            r#"import defaultExport, * as name from 'module-name';"#,
            r#"import 'module-name';"#,
        ];
        inputs.iter().for_each(|&input| {
            let mut visitor = ModuleSymbolVisitor::new();
            parse_with_visitor![input, visitor];
        });
    }

    #[test]
    fn test_exporting_declaration_let() {
        let input = r#"export let name1, name2/*, … */; // also var"#;
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
    fn test_exporting_default_function() {
        let input = r#"export default function functionName() { /* … */ }"#;
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

        assert_eq!(visitor.re_exporting_all_from, ["module-name"]);
        assert_eq!(visitor.named_export_table.len(), 0);
        assert!(visitor.default_export.is_none());
        assert_eq!(visitor.local_variable_table.len(), 0);
        assert_eq!(visitor.tracked_ids.len(), 0);
    }

    #[test]
    fn test_aggregating_modules_re_export_all_as_namespace_from_other_module() {
        let input = r#"export * as name1 from 'module-name';"#;
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
        let mut visitor = ModuleSymbolVisitor::new();
        parse_with_visitor![input, visitor];

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
}
