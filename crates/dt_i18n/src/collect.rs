use super::core;
use anyhow::{bail, Context};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
use swc_core::{
    common::{sync::Lrc, Globals, Mark, SourceMap, GLOBALS},
    ecma::{ast::*, transforms::base::resolver, visit::FoldWith},
};
use swc_ecma_parser::{parse_file_as_module, Syntax, TsSyntax};

pub fn collect_all_translation_usage(
    root: &str,
) -> anyhow::Result<HashMap<String, HashMap<String, HashSet<String>>>> {
    // {
    //   "i18n.bird" => {
    //     "module path 1" => ["A", "B"],
    //     "module path 2" => ["Foo", "Bar"],
    //   },
    //   "i18n.cat" => {
    //     "module path 1" => ["A", "B", "C"],
    //   },
    // }
    let mut res: HashMap<String, HashMap<String, HashSet<String>>> = HashMap::new();
    let all_paths = collect_all_paths(&PathBuf::from(root))?;
    for path in all_paths.iter() {
        let path_str = path.to_str().context("&PathBuf -> &str")?;
        // - module path 1
        //   {
        //     "A" => ["i18n.bird", "i18n.cat"],
        //     "B" => ["i18n.bird", "i18n.cat"]
        //     "C" => ["i18n.cat"]
        //   }
        // - module path 2
        //   {
        //     "Foo" => ["i18n.bird"]
        //     "Bar" => ["i18n.cat"]
        //   }
        let translate_usage = get_translation_usage(path)?;
        for (symbol, translation_keys) in translate_usage {
            for translation_key in translation_keys.iter() {
                if !res.contains_key(translation_key) {
                    res.insert(translation_key.to_owned(), HashMap::new());
                }
                if !res.get(translation_key).unwrap().contains_key(path_str) {
                    res.get_mut(translation_key)
                        .unwrap()
                        .insert(path_str.to_string(), HashSet::new());
                }
                res.get_mut(translation_key)
                    .unwrap()
                    .get_mut(path_str)
                    .unwrap()
                    .insert(symbol.to_owned());
            }
        }
    }

    Ok(res)
}

fn collect_all_paths(root: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let path = root.canonicalize()?;
    let mut paths = vec![];

    if path.is_dir() {
        for entry in path.read_dir()? {
            if let Ok(entry) = entry {
                paths.append(&mut collect_all_paths(&entry.path())?);
            }
        }
        return Ok(paths);
    }

    let path_str = path.to_str().context("path to str")?;
    if path_str.ends_with(".js")
        || path_str.ends_with(".jsx")
        || path_str.ends_with(".ts")
        || path_str.ends_with(".tsx")
    {
        if !path_str.ends_with(".spec.js")
            && !path_str.ends_with(".spec.jsx")
            && !path_str.ends_with(".spec.ts")
            && !path_str.ends_with(".spec.tsx")
            && !path_str.ends_with(".test.js")
            && !path_str.ends_with(".test.jsx")
            && !path_str.ends_with(".test.ts")
            && !path_str.ends_with(".test.tsx")
        {
            paths.push(path.clone())
        }
    }
    Ok(paths)
}

fn get_translation_usage(path: &PathBuf) -> anyhow::Result<HashMap<String, HashSet<String>>> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm
        .load_file(Path::new(path))
        .context(format!("failed to load {:?}", path))?;

    let module = match parse_file_as_module(
        &fm,
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
        Ok(v) => v,
        // We are not testing parser
        Err(..) => bail!("failed to parse {:?}", path),
    };

    // This is how swc manages identifiers. ref: https://rustdoc.swc.rs/swc_ecma_transforms/fn.resolver.html
    let module = GLOBALS.set(&Globals::new(), move || {
        module.fold_with(&mut resolver(Mark::new(), Mark::new(), true))
    });

    Ok(core::collect_translation(&module)?)
}
