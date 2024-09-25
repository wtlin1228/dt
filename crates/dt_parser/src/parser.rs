use super::{
    to_symbol_name::ToSymbolName,
    types::ParsedModule,
    visitors::{
        construct_symbol_dependency::SymbolDependencyVisitor,
        extract_module_scopped_symbols::ModuleScoppedSymbolsVisitor,
    },
};

use anyhow::Context;
use std::path::Path;
use swc_core::{
    common::{
        errors::{ColorConfig, Handler},
        sync::Lrc,
        Globals, Mark, SourceFile, SourceMap, GLOBALS,
    },
    ecma::{
        transforms::base::resolver,
        visit::{FoldWith, VisitWith},
    },
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

pub fn parse(module_path: &str) -> anyhow::Result<ParsedModule> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm
        .load_file(Path::new(module_path))
        .context(format!("failed to load {:?}", module_path))?;
    parse_module(module_path, cm, fm)
}

fn parse_module(
    module_src: &str,
    cm: Lrc<SourceMap>,
    fm: Lrc<SourceFile>,
) -> anyhow::Result<ParsedModule> {
    let handler: Handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: false,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: true,
        }),
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }
    let module: swc_core::ecma::ast::Module = parser
        .parse_module()
        .map_err(|e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("failed to parser module");

    let module = GLOBALS.set(&Globals::new(), || {
        // ref: https://rustdoc.swc.rs/swc_ecma_transforms_base/fn.resolver.html
        module.fold_with(&mut resolver(Mark::new(), Mark::new(), true))
    });

    let mut symbol_visitor = ModuleScoppedSymbolsVisitor::new();
    module.visit_with(&mut symbol_visitor);

    let mut symbol_dependency_visitor = SymbolDependencyVisitor::new(symbol_visitor.tracked_ids);
    module.visit_with(&mut symbol_dependency_visitor);

    let mut parsed_module = ParsedModule {
        canonical_path: module_src.to_string(),
        local_variable_table: symbol_visitor.local_variable_table,
        named_export_table: symbol_visitor.named_export_table,
        default_export: symbol_visitor.default_export,
        re_export_star_from: match symbol_visitor.re_exporting_all_from.len() > 0 {
            true => Some(symbol_visitor.re_exporting_all_from),
            false => None,
        },
    };

    for (key, value) in symbol_dependency_visitor.dependency.iter() {
        if value.len() == 0 {
            continue;
        }

        let mut depend_on = Vec::with_capacity(value.len());
        for d in value.iter() {
            depend_on.push(d.to_symbol_name());
        }
        depend_on.sort_unstable();

        let local_variable = parsed_module
            .local_variable_table
            .get_mut(&key.to_symbol_name())
            .context(format!("local variable {} not found", key.to_symbol_name()))?;
        local_variable.depend_on = Some(depend_on);
    }

    Ok(parsed_module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
        types::{FromOtherModule, FromType, ModuleExport, ModuleScopedVariable},
    };
    use dt_test_utils::assert_hash_map;
    use swc_core::common::FileName;

    #[test]
    fn test_empty_input() {
        let cm: Lrc<SourceMap> = Default::default();
        let fm: Lrc<SourceFile> =
            cm.new_source_file(Lrc::new(FileName::Custom("test.js".into())), "".into());
        let module: ParsedModule = parse_module("test.js", cm, fm).unwrap();

        assert_eq!(module.canonical_path, "test.js");
        assert_eq!(module.local_variable_table.len(), 0);
        assert_eq!(module.named_export_table.len(), 0);
        assert!(module.default_export.is_none());
        assert!(module.re_export_star_from.is_none());
    }

    #[test]
    fn test_anonymous_default_export_function() {
        let cm: Lrc<SourceMap> = Default::default();
        let fm: Lrc<SourceFile> = cm.new_source_file(
            Lrc::new(FileName::Custom("test.js".into())),
            r#"
            let name1, name2;
            export default function () {
                let useName1 = name1;
                console.log(name2);
            }
            "#
            .into(),
        );
        let module: ParsedModule = parse_module("test.js", cm, fm).unwrap();

        assert_eq!(module.canonical_path, "test.js");
        assert_hash_map!(
            module.local_variable_table,
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
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: Some(vec![String::from("name1"), String::from("name2")]),
                    import_from: None
                }
            ),
        );
        assert_eq!(module.named_export_table.len(), 0);
        assert_eq!(
            module.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert!(module.re_export_star_from.is_none());
    }

    #[test]
    fn test_anonymous_default_export_class() {
        let cm: Lrc<SourceMap> = Default::default();
        let fm: Lrc<SourceFile> = cm.new_source_file(
            Lrc::new(FileName::Custom("test.js".into())),
            r#"
            let name1, name2;
            export default class {
                method() {
                    let useName1 = name1;
                    console.log(name2);
                }
            }
            "#
            .into(),
        );
        let module: ParsedModule = parse_module("test.js", cm, fm).unwrap();

        assert_eq!(module.canonical_path, "test.js");
        assert_hash_map!(
            module.local_variable_table,
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
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: Some(vec![String::from("name1"), String::from("name2")]),
                    import_from: None
                }
            ),
        );
        assert_eq!(module.named_export_table.len(), 0);
        assert_eq!(
            module.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert!(module.re_export_star_from.is_none());
    }

    #[test]
    fn test_anonymous_default_export_object() {
        let cm: Lrc<SourceMap> = Default::default();
        let fm: Lrc<SourceFile> = cm.new_source_file(
            Lrc::new(FileName::Custom("test.js".into())),
            r#"
            let name1, name2;
            export default { name1, name2 };
            "#
            .into(),
        );
        let module: ParsedModule = parse_module("test.js", cm, fm).unwrap();

        assert_eq!(module.canonical_path, "test.js");
        assert_hash_map!(
            module.local_variable_table,
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
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: Some(vec![String::from("name1"), String::from("name2")]),
                    import_from: None
                }
            ),
        );
        assert_eq!(module.named_export_table.len(), 0);
        assert_eq!(
            module.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert!(module.re_export_star_from.is_none());
    }

    #[test]
    fn test_anonymous_default_export_array() {
        let cm: Lrc<SourceMap> = Default::default();
        let fm: Lrc<SourceFile> = cm.new_source_file(
            Lrc::new(FileName::Custom("test.js".into())),
            r#"
            let name1, name2;
            export default [name1, name2];
            "#
            .into(),
        );
        let module: ParsedModule = parse_module("test.js", cm, fm).unwrap();

        assert_eq!(module.canonical_path, "test.js");
        assert_hash_map!(
            module.local_variable_table,
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
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
                ModuleScopedVariable {
                    depend_on: Some(vec![String::from("name1"), String::from("name2")]),
                    import_from: None
                }
            ),
        );
        assert_eq!(module.named_export_table.len(), 0);
        assert_eq!(
            module.default_export,
            Some(ModuleExport::Local(
                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()
            ))
        );
        assert!(module.re_export_star_from.is_none());
    }

    #[test]
    fn test_complex_input() {
        let cm: Lrc<SourceMap> = Default::default();
        let fm: Lrc<SourceFile> = cm.new_source_file(
            Lrc::new(FileName::Custom("test.js".into())),
            r#"
            import Kirby, { Power, Pink as KirbyPink, Puffy } from './kirby';
            import * as Hawk from './hawk';
            const sugar = '', salt = '';
            const cruet = [sugar, salt];
            export class PicnicBox {
                constructor() {
                    this.cruet = cruet;
                    this.sandwich = 'beef sandwich';
                    this.cookie = { color: KirbyPink, texture: Puffy };
                }
            }
            const deliverPicnicBox = (location) => {
                Kirby.bring(new PicnicBox())
                Kirby.goto(location);
                Kirby.put()
            }
            function WelcomeMessage() {
                return "Welcome 🤗 Kirby is delivering your picnic box 👜";
            }
            export { WelcomeMessage as welcome };
            export function InvitationCard() {
                const [opened, setOpened] = React.useState(false);
                if (!opened) {
                    return (
                        <Hawk.PigNose 
                            onPush={() => {
                                setOpened(true);
                                deliverPicnicBox();
                            }} 
                        />
                    )
                } else {
                    return <WelcomeMessage />
                }
            }
            export default InvitationCard;
            export * from './wild';
            export * as Wild from './wild';
            export * from './happy';
            "#
            .into(),
        );
        let module: ParsedModule = parse_module("test.js", cm, fm).unwrap();

        assert_eq!(module.canonical_path, "test.js");
        assert_hash_map!(
            module.local_variable_table,
            (
                "Kirby",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("./kirby"),
                        from_type: FromType::Default
                    })
                }
            ),
            (
                "Power",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("./kirby"),
                        from_type: FromType::Named(String::from("Power"))
                    })
                }
            ),
            (
                "KirbyPink",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("./kirby"),
                        from_type: FromType::Named(String::from("Pink"))
                    })
                }
            ),
            (
                "Puffy",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("./kirby"),
                        from_type: FromType::Named(String::from("Puffy"))
                    })
                }
            ),
            (
                "Hawk",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: Some(FromOtherModule {
                        from: String::from("./hawk"),
                        from_type: FromType::Namespace
                    })
                }
            ),
            (
                "sugar",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "salt",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "cruet",
                ModuleScopedVariable {
                    depend_on: Some(vec![String::from("salt"), String::from("sugar")]),
                    import_from: None
                }
            ),
            (
                "PicnicBox",
                ModuleScopedVariable {
                    depend_on: Some(vec![
                        String::from("KirbyPink"),
                        String::from("Puffy"),
                        String::from("cruet"),
                    ]),
                    import_from: None
                }
            ),
            (
                "deliverPicnicBox",
                ModuleScopedVariable {
                    depend_on: Some(vec![String::from("Kirby"), String::from("PicnicBox")]),
                    import_from: None
                }
            ),
            (
                "WelcomeMessage",
                ModuleScopedVariable {
                    depend_on: None,
                    import_from: None
                }
            ),
            (
                "InvitationCard",
                ModuleScopedVariable {
                    depend_on: Some(vec![
                        String::from("Hawk"),
                        String::from("WelcomeMessage"),
                        String::from("deliverPicnicBox")
                    ]),
                    import_from: None
                }
            ),
        );
        assert_hash_map!(
            module.named_export_table,
            ("PicnicBox", ModuleExport::Local(String::from("PicnicBox"))),
            (
                "welcome",
                ModuleExport::Local(String::from("WelcomeMessage"))
            ),
            (
                "Wild",
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("./wild"),
                    from_type: FromType::Namespace,
                })
            ),
            (
                "InvitationCard",
                ModuleExport::Local(String::from("InvitationCard"))
            )
        );
        assert_eq!(
            module.default_export,
            Some(ModuleExport::Local(String::from("InvitationCard")))
        );
        assert_eq!(module.re_export_star_from.unwrap(), ["./wild", "./happy"]);
    }
}
