use std::path::PathBuf;

use dt_scheduler::ParserCandidateScheduler;

#[test]
fn topological_order() {
    let root = "tests/fixture";
    let mut scheduler = ParserCandidateScheduler::new(root);

    let topological_order = [
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
    ];

    assert_eq!(
        scheduler.get_total_remaining_candidate_count(),
        topological_order.len()
    );
    for expected in topological_order {
        let candidate = scheduler.get_one_candidate().unwrap();
        assert_eq!(
            candidate,
            PathBuf::from(root).join(expected).canonicalize().unwrap()
        );
        scheduler.mark_candidate_as_parsed(candidate);
    }
    assert_eq!(scheduler.get_total_remaining_candidate_count(), 0);
}
