use dt_graph::depend_on_graph::DependOnGraph;
use dt_parser::types::{
    FromOtherModule, FromType, ModuleExport, ModuleScopedVariable, ParsedModule,
};
use dt_path_resolver::ToCanonicalString;
use dt_test_utils::assert_hash_map;
use std::{collections::HashMap, path::PathBuf};

#[test]
fn add_two_modules() {
    let root = "tests/fixture/depend_on";

    let canonical_path_hawk = PathBuf::from(root)
        .join("hawk.js")
        .to_canonical_string()
        .unwrap();
    let canonical_path_red_demon = PathBuf::from(root)
        .join("red-demon.js")
        .to_canonical_string()
        .unwrap();
    let canonical_path_grey_demon = PathBuf::from(root)
        .join("grey-demon.js")
        .to_canonical_string()
        .unwrap();
    let canonical_path_kirby = PathBuf::from(root)
        .join("kirby.js")
        .to_canonical_string()
        .unwrap();

    let mut dt = DependOnGraph::new(root);
    let hawk = ParsedModule {
        canonical_path: canonical_path_hawk.clone(),
        local_variable_table: HashMap::from([(
            String::from("RedDemon"),
            ModuleScopedVariable {
                depend_on: None,
                import_from: Some(FromOtherModule {
                    from: String::from("red-demon"),
                    from_type: FromType::Default,
                }),
            },
        )]),
        named_export_table: HashMap::from([
            (
                String::from("HawkRedDemon"),
                ModuleExport::Local(String::from("RedDemon")),
            ),
            (
                String::from("HawkGreyDemon"),
                ModuleExport::ReExportFrom(FromOtherModule {
                    from: String::from("grey-demon"),
                    from_type: FromType::Named(String::from("GreyDemon")),
                }),
            ),
        ]),
        default_export: None,
        re_export_star_from: None,
    };
    dt.add_parsed_module(hawk).unwrap();
    assert_eq!(dt.parsed_modules_table.len(), 1);
    let hawk = dt.parsed_modules_table.get(&canonical_path_hawk).unwrap();
    assert_hash_map!(
        hawk.local_variable_table,
        (
            "RedDemon",
            ModuleScopedVariable {
                depend_on: None,
                import_from: Some(FromOtherModule {
                    from: canonical_path_red_demon.clone(),
                    from_type: FromType::Default,
                }),
            }
        ),
    );
    assert_hash_map!(
        hawk.named_export_table,
        (
            "HawkRedDemon",
            ModuleExport::Local(String::from("RedDemon"))
        ),
        (
            "HawkGreyDemon",
            ModuleExport::ReExportFrom(FromOtherModule {
                from: canonical_path_grey_demon.clone(),
                from_type: FromType::Named(String::from("GreyDemon"))
            })
        )
    );

    let kirby = ParsedModule {
        canonical_path: canonical_path_kirby.clone(),
        local_variable_table: HashMap::new(),
        named_export_table: HashMap::new(),
        default_export: None,
        re_export_star_from: Some(vec![String::from("hawk")]),
    };
    dt.add_parsed_module(kirby).unwrap();
    assert_eq!(dt.parsed_modules_table.len(), 2);
    let kirby = dt.parsed_modules_table.get(&canonical_path_kirby).unwrap();
    assert_eq!(kirby.local_variable_table.len(), 0);
    assert_eq!(kirby.re_export_star_from, None);
    assert_hash_map!(
        kirby.named_export_table,
        (
            "HawkRedDemon",
            ModuleExport::ReExportFrom(FromOtherModule {
                from: canonical_path_hawk.clone(),
                from_type: FromType::Named(String::from("HawkRedDemon"))
            })
        ),
        (
            "HawkGreyDemon",
            ModuleExport::ReExportFrom(FromOtherModule {
                from: canonical_path_hawk.clone(),
                from_type: FromType::Named(String::from("HawkGreyDemon"))
            })
        )
    );
}
