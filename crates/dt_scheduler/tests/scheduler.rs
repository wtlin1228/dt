use dt_scheduler::ParserCandidateScheduler;
use std::{collections::HashSet, path::PathBuf};

#[test]
fn topological_order() {
    let root = "tests/fixture";
    let mut scheduler = ParserCandidateScheduler::new(root);

    let mut not_parsed = HashSet::from(
        [
            "blocker.js",
            "reexport/default-alias.js",
            "reexport/wildcard-alias.js",
            "reexport/named-alias.js",
            "reexport/named.js",
            "reexport/default.js",
            "non-blocker.js",
            "import/side-effect.js",
            "import/default-alias.js",
            "import/named-alias.js",
            "import/named.js",
            "import/default.js",
            "reexport/wildcard.js",
            "import/namespace.js",
            "index.js",
        ]
        .map(|s| PathBuf::from(root).join(s).canonicalize().unwrap()),
    );

    let wildcard_reexport = PathBuf::from(root)
        .join("reexport/wildcard.js")
        .canonicalize()
        .unwrap();
    let namespace_import = PathBuf::from(root)
        .join("import/namespace.js")
        .canonicalize()
        .unwrap();
    let blocker = PathBuf::from(root)
        .join("blocker.js")
        .canonicalize()
        .unwrap();

    assert_eq!(
        scheduler.get_total_remaining_candidate_count(),
        not_parsed.len()
    );
    for _ in 0..not_parsed.len() {
        let candidate = scheduler.get_one_candidate().unwrap();
        assert!(not_parsed.contains(&candidate));

        // `reexport/wildcard.js` and `import/namespace.js` are blocked by `blocker.js`
        if candidate == blocker {
            assert!(not_parsed.contains(&wildcard_reexport));
            assert!(not_parsed.contains(&namespace_import));
        }

        assert!(not_parsed.remove(&candidate));
        scheduler.mark_candidate_as_parsed(candidate);
    }
    assert_eq!(scheduler.get_total_remaining_candidate_count(), 0);
}
