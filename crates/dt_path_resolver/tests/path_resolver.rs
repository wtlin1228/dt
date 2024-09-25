use dt_path_resolver::{PathResolver, ToCanonicalString};
use std::path::PathBuf;

macro_rules! assert_resolved {
    ($base:expr, $(($current:expr, $import:expr) => $resolved:expr),* $(,)?) => {{
        let resolver = PathResolver::new($base);
        $(
            let resolved = resolver.resolve_path($current, $import).unwrap();
            assert_eq!(
                resolved,
                PathBuf::from($resolved).to_canonical_string().unwrap()
            );
        )*
    }};
}

#[test]
fn resolve_relative_path() {
    assert_resolved!(
        "tests/fixture",
        ("tests/fixture/index.js", "./index-js/a") => "tests/fixture/index-js/a/index.js",
        ("tests/fixture/index.js", "./index-ts/a") => "tests/fixture/index-ts/a/index.ts",
        ("tests/fixture/index.js", "./js/a")       => "tests/fixture/js/a.js",
        ("tests/fixture/index.js", "./ts/a")       => "tests/fixture/ts/a.ts",
        ("tests/fixture/index.js", "./jsx/a")      => "tests/fixture/jsx/a.jsx",
        ("tests/fixture/index.js", "./tsx/a")      => "tests/fixture/tsx/a.tsx",
    );
}

#[test]
fn resolve_alias_path() {
    assert_resolved!(
        "tests/fixture",
        ("tests/fixture/index.js", "index-js/a") => "tests/fixture/index-js/a/index.js",
        ("tests/fixture/index.js", "index-ts/a") => "tests/fixture/index-ts/a/index.ts",
        ("tests/fixture/index.js", "js/a")       => "tests/fixture/js/a.js",
        ("tests/fixture/index.js", "ts/a")       => "tests/fixture/ts/a.ts",
        ("tests/fixture/index.js", "jsx/a")      => "tests/fixture/jsx/a.jsx",
        ("tests/fixture/index.js", "tsx/a")      => "tests/fixture/tsx/a.tsx",
    );
}
