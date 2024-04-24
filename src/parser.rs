use super::types::ParsedModule;

use std::collections::HashMap;
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
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

pub fn parse_module(
    module_src: &str,
    cm: Lrc<SourceMap>,
    fm: Lrc<SourceFile>,
) -> anyhow::Result<ParsedModule> {
    let handler: Handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
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

    let mut parsed_module = ParsedModule {
        canonical_path: module_src.to_string(),
        local_variable_table: HashMap::new(),
        named_export_table: HashMap::new(),
        default_export: None,
    };

    GLOBALS.set(&Globals::new(), || {
        // ref: https://rustdoc.swc.rs/swc_ecma_transforms_base/fn.resolver.html
        let module: swc_core::ecma::ast::Module =
            module.fold_with(&mut resolver(Mark::new(), Mark::new(), true));
    });

    Ok(parsed_module)
}

#[cfg(test)]
mod tests {
    use swc_core::common::FileName;

    use super::*;

    #[test]
    fn test_parse_module() {
        let cm: Lrc<SourceMap> = Default::default();
        let fm: Lrc<SourceFile> = cm.new_source_file(FileName::Custom("test.js".into()), "".into());
        let module: ParsedModule = parse_module("test.js", cm, fm).unwrap();

        assert_eq!(module.canonical_path, "test.js");
        assert_eq!(module.local_variable_table.len(), 0);
        assert_eq!(module.named_export_table.len(), 0);
        assert!(module.default_export.is_none());
    }
}
