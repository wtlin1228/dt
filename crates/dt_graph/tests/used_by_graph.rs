use dt_graph::{
    depend_on_graph::DependOnGraph,
    used_by_graph::{UsedBy, UsedByGraph, UsedByOther, UsedByType},
};
use dt_parser::{anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT, parse};
use dt_path_resolver::ToCanonicalString;
use std::path::PathBuf;

macro_rules! s {
    ($s:expr) => {{
        $s.to_string()
    }};
}

macro_rules! assert_used_by_vec {
    ($left:expr, $right:expr) => {
        let mut left = $left.clone();
        // need to sort the used-by vector since the order is undetermined
        left.sort();
        assert_eq!(left, $right, "used-by vector mismatch");
    };
}

macro_rules! assert_used_by_table {
    ($table:expr, $(($key:expr, $expected:expr)),* $(,)?) => {{
        let mut count = 0;
        $(
            count += 1;
            assert!($table.contains_key($key), "missing key: {}", $key);
            match $expected {
                Some(expected_used_by_vec) => {
                    let used_by_vec = $table.get($key).unwrap().as_ref().unwrap();
                    assert_used_by_vec!(used_by_vec, expected_used_by_vec);
                },
                None => {
                    assert!($table.get($key).unwrap().is_none());
                }
            }
        )*
        assert_eq!($table.len(), count, "entry count mismatch");
    }};
}

#[test]
fn picnic_time() {
    let root = "tests/fixture/used_by";
    let mut depend_on_graph = DependOnGraph::new(root);
    let [happy_path, hawk_path, kirby_path, wild_path, picnic_time_path] = [
        "happy.js",
        "hawk.js",
        "kirby.js",
        "wild.js",
        "PicnicTime.js",
    ]
    .map(|path| {
        PathBuf::from(root)
            .join(path)
            .to_canonical_string()
            .unwrap()
    });
    depend_on_graph
        .add_parsed_module(parse(&happy_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&hawk_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&kirby_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&wild_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&picnic_time_path).unwrap())
        .unwrap();
    let used_by_graph = UsedByGraph::from(&depend_on_graph);

    assert_eq!(used_by_graph.modules.len(), 5);

    // happy.js is an empty file
    let happy_module = used_by_graph.modules.get(&happy_path).unwrap();
    assert_eq!(happy_module.local_variable_table.len(), 0);
    assert_eq!(happy_module.named_export_table.len(), 0);
    assert!(happy_module.default_export.is_none());

    let hawk_module = used_by_graph.modules.get(&hawk_path).unwrap();
    assert_used_by_table!(
        hawk_module.local_variable_table,
        (
            "PigNose",
            Some(vec![UsedBy::Itself(UsedByType::NamedExport(s!("PigNose")))])
        ),
        (
            "Pink",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Pink"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("PigNose"))),
            ])
        )
    );
    assert_used_by_table!(
        hawk_module.named_export_table,
        (
            "PigNose",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Hawk")),
            })])
        ),
        (
            "Pink",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Hawk")),
            })])
        )
    );
    assert!(hawk_module.default_export.is_none());

    let kirby_module = used_by_graph.modules.get(&kirby_path).unwrap();
    assert_used_by_table!(
        kirby_module.local_variable_table,
        (
            "Power",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Power"))),
                UsedBy::Itself(UsedByType::LocalVar(
                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                )),
            ])
        ),
        (
            "Pink",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Pink"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("Power"))),
            ])
        ),
        (
            "Puffy",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Puffy"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("Power"))),
            ])
        ),
        (
            SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
            Some(vec![UsedBy::Itself(UsedByType::DefaultExport)])
        )
    );
    assert_used_by_table!(
        kirby_module.named_export_table,
        (
            "Power",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Power")),
            })])
        ),
        (
            "Pink",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("KirbyPink")),
            })])
        ),
        (
            "Puffy",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Puffy")),
            })])
        )
    );
    assert_used_by_vec!(
        kirby_module.default_export.as_ref().unwrap(),
        vec![UsedBy::Other(UsedByOther {
            by: picnic_time_path.to_owned(),
            by_type: UsedByType::LocalVar(s!("Kirby")),
        })]
    );

    let wild_module = used_by_graph.modules.get(&wild_path).unwrap();
    assert_used_by_table!(
        wild_module.local_variable_table,
        (
            "ZigZagWild",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("ZigZagWild"))),
                UsedBy::Itself(UsedByType::DefaultExport),
            ])
        ),
        (
            "GuruguruWild",
            Some(vec![UsedBy::Itself(UsedByType::NamedExport(s!(
                "GuruguruWild"
            )))])
        ),
    );
    assert_used_by_table!(
        wild_module.named_export_table,
        (
            "ZigZagWild",
            Some(vec![
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("Wild")),
                }),
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("ZigZagWild")),
                }),
            ])
        ),
        (
            "GuruguruWild",
            Some(vec![
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("GuruguruWild")),
                }),
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("Wild")),
                }),
            ])
        )
    );
    assert!(wild_module.default_export.is_none());

    let picnic_time_module = used_by_graph.modules.get(&picnic_time_path).unwrap();
    assert_used_by_table!(
        picnic_time_module.local_variable_table,
        // import Kirby, { Power, Pink as KirbyPink, Puffy } from './kirby';
        (
            "Kirby",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                "deliverPicnicBox"
            )))])
        ),
        ("Power", None::<Vec<UsedBy>>),
        (
            "KirbyPink",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))])
        ),
        (
            "Puffy",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))])
        ),
        // import * as Hawk from './hawk';
        (
            "Hawk",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                "InvitationCard"
            )))])
        ),
        // declare locally
        (
            "sugar",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("cruet")))])
        ),
        (
            "salt",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("cruet")))])
        ),
        (
            "cruet",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))])
        ),
        (
            "PicnicBox",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("PicnicBox"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("deliverPicnicBox"))),
            ])
        ),
        (
            "deliverPicnicBox",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                "InvitationCard"
            )))])
        ),
        (
            "WelcomeMessage",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("welcome"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("InvitationCard"))),
            ])
        ),
        (
            "InvitationCard",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("InvitationCard"))),
                UsedBy::Itself(UsedByType::DefaultExport),
            ])
        ),
    );
    assert_used_by_table!(
        picnic_time_module.named_export_table,
        // export local variables
        ("PicnicBox", None::<Vec<UsedBy>>),
        ("welcome", None::<Vec<UsedBy>>),
        ("InvitationCard", None::<Vec<UsedBy>>),
        // export * from './wild';
        ("ZigZagWild", None::<Vec<UsedBy>>),
        ("GuruguruWild", None::<Vec<UsedBy>>),
        // export * as Wild from './wild';
        ("Wild", None::<Vec<UsedBy>>),
    );
    assert!(picnic_time_module.default_export.is_none());
}

#[test]
fn export_and_import() {
    let root = "tests/fixture/used_by";
    let mut depend_on_graph = DependOnGraph::new(root);
    let [happy_path, hawk_path, kirby_path, wild_path, picnic_time_path] = [
        "happy.js",
        "hawk.js",
        "kirby.js",
        "wild.js",
        "PicnicTime.js",
    ]
    .map(|path| {
        PathBuf::from(root)
            .join(path)
            .to_canonical_string()
            .unwrap()
    });
    depend_on_graph
        .add_parsed_module(parse(&happy_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&hawk_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&kirby_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&wild_path).unwrap())
        .unwrap();
    depend_on_graph
        .add_parsed_module(parse(&picnic_time_path).unwrap())
        .unwrap();
    let picnic_time_graph = UsedByGraph::from(&depend_on_graph);

    // test the export and import here 👇, others are the same as picnic_time test
    let exported = picnic_time_graph.export().unwrap();
    let used_by_graph = UsedByGraph::import(&exported).unwrap();

    assert_eq!(used_by_graph.modules.len(), 5);

    // happy.js is an empty file
    let happy_module = used_by_graph.modules.get(&happy_path).unwrap();
    assert_eq!(happy_module.local_variable_table.len(), 0);
    assert_eq!(happy_module.named_export_table.len(), 0);
    assert!(happy_module.default_export.is_none());

    let hawk_module = used_by_graph.modules.get(&hawk_path).unwrap();
    assert_used_by_table!(
        hawk_module.local_variable_table,
        (
            "PigNose",
            Some(vec![UsedBy::Itself(UsedByType::NamedExport(s!("PigNose")))])
        ),
        (
            "Pink",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Pink"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("PigNose"))),
            ])
        )
    );
    assert_used_by_table!(
        hawk_module.named_export_table,
        (
            "PigNose",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Hawk")),
            })])
        ),
        (
            "Pink",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Hawk")),
            })])
        )
    );
    assert!(hawk_module.default_export.is_none());

    let kirby_module = used_by_graph.modules.get(&kirby_path).unwrap();
    assert_used_by_table!(
        kirby_module.local_variable_table,
        (
            "Power",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Power"))),
                UsedBy::Itself(UsedByType::LocalVar(
                    SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                )),
            ])
        ),
        (
            "Pink",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Pink"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("Power"))),
            ])
        ),
        (
            "Puffy",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("Puffy"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("Power"))),
            ])
        ),
        (
            SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT,
            Some(vec![UsedBy::Itself(UsedByType::DefaultExport)])
        )
    );
    assert_used_by_table!(
        kirby_module.named_export_table,
        (
            "Power",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Power")),
            })])
        ),
        (
            "Pink",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("KirbyPink")),
            })])
        ),
        (
            "Puffy",
            Some(vec![UsedBy::Other(UsedByOther {
                by: picnic_time_path.to_owned(),
                by_type: UsedByType::LocalVar(s!("Puffy")),
            })])
        )
    );
    assert_used_by_vec!(
        kirby_module.default_export.as_ref().unwrap(),
        vec![UsedBy::Other(UsedByOther {
            by: picnic_time_path.to_owned(),
            by_type: UsedByType::LocalVar(s!("Kirby")),
        })]
    );

    let wild_module = used_by_graph.modules.get(&wild_path).unwrap();
    assert_used_by_table!(
        wild_module.local_variable_table,
        (
            "ZigZagWild",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("ZigZagWild"))),
                UsedBy::Itself(UsedByType::DefaultExport),
            ])
        ),
        (
            "GuruguruWild",
            Some(vec![UsedBy::Itself(UsedByType::NamedExport(s!(
                "GuruguruWild"
            )))])
        ),
    );
    assert_used_by_table!(
        wild_module.named_export_table,
        (
            "ZigZagWild",
            Some(vec![
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("Wild")),
                }),
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("ZigZagWild")),
                }),
            ])
        ),
        (
            "GuruguruWild",
            Some(vec![
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("GuruguruWild")),
                }),
                UsedBy::Other(UsedByOther {
                    by: picnic_time_path.to_owned(),
                    by_type: UsedByType::NamedExport(s!("Wild")),
                }),
            ])
        )
    );
    assert!(wild_module.default_export.is_none());

    let picnic_time_module = used_by_graph.modules.get(&picnic_time_path).unwrap();
    assert_used_by_table!(
        picnic_time_module.local_variable_table,
        // import Kirby, { Power, Pink as KirbyPink, Puffy } from './kirby';
        (
            "Kirby",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                "deliverPicnicBox"
            )))])
        ),
        ("Power", None::<Vec<UsedBy>>),
        (
            "KirbyPink",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))])
        ),
        (
            "Puffy",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))])
        ),
        // import * as Hawk from './hawk';
        (
            "Hawk",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                "InvitationCard"
            )))])
        ),
        // declare locally
        (
            "sugar",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("cruet")))])
        ),
        (
            "salt",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("cruet")))])
        ),
        (
            "cruet",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))])
        ),
        (
            "PicnicBox",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("PicnicBox"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("deliverPicnicBox"))),
            ])
        ),
        (
            "deliverPicnicBox",
            Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                "InvitationCard"
            )))])
        ),
        (
            "WelcomeMessage",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("welcome"))),
                UsedBy::Itself(UsedByType::LocalVar(s!("InvitationCard"))),
            ])
        ),
        (
            "InvitationCard",
            Some(vec![
                UsedBy::Itself(UsedByType::NamedExport(s!("InvitationCard"))),
                UsedBy::Itself(UsedByType::DefaultExport),
            ])
        ),
    );
    assert_used_by_table!(
        picnic_time_module.named_export_table,
        // export local variables
        ("PicnicBox", None::<Vec<UsedBy>>),
        ("welcome", None::<Vec<UsedBy>>),
        ("InvitationCard", None::<Vec<UsedBy>>),
        // export * from './wild';
        ("ZigZagWild", None::<Vec<UsedBy>>),
        ("GuruguruWild", None::<Vec<UsedBy>>),
        // export * as Wild from './wild';
        ("Wild", None::<Vec<UsedBy>>),
    );
    assert!(picnic_time_module.default_export.is_none());
}
