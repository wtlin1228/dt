use anyhow::bail;
use swc_core::{
    common::{sync::Lrc, FileName, Globals, Mark, SourceMap, GLOBALS},
    ecma::{ast::*, transforms::base::resolver, visit::FoldWith},
};
use swc_ecma_parser::{parse_file_as_module, Syntax, TsSyntax};

pub fn parse_module(input: &str) -> anyhow::Result<Module> {
    let cm: Lrc<SourceMap> = Default::default();
    let module = match parse_file_as_module(
        &cm.new_source_file(Lrc::new(FileName::Custom("test.js".into())), input.into()),
        Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: true,
            no_early_errors: true,
            ..Default::default()
        }),
        EsVersion::latest(),
        None,
        &mut Vec::new(),
    ) {
        Ok(module) => module,
        Err(_) => bail!("failed to parse module"),
    };

    // This is how swc manages identifiers. ref: https://rustdoc.swc.rs/swc_ecma_transforms/fn.resolver.html
    let module = GLOBALS.set(&Globals::new(), move || {
        module.fold_with(&mut resolver(Mark::new(), Mark::new(), true))
    });

    Ok(module)
}
