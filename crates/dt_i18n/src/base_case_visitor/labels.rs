use anyhow::{bail, Context};
use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::*;

#[derive(Debug, PartialEq)]
pub enum TranslateObjectValue {
    String(String),
    NestedLabels(LABELS),
}

impl TranslateObjectValue {
    pub fn get_string(&self) -> anyhow::Result<&str> {
        match self {
            TranslateObjectValue::String(s) => Ok(s),
            TranslateObjectValue::NestedLabels(_) => bail!("it's a nested labels"),
        }
    }

    pub fn get_labels(&self) -> anyhow::Result<&LABELS> {
        match self {
            TranslateObjectValue::String(_) => bail!("it's a string"),
            TranslateObjectValue::NestedLabels(labels) => Ok(labels),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LABELS {
    Object(HashMap<String, TranslateObjectValue>),

    // If we found the object has computed keys, just collect all lokalise keys into a vector.
    // Because usually we'll use partial or all of them in the runtime.
    Computed(HashSet<String>),
}

impl LABELS {
    pub fn get_object(&self) -> anyhow::Result<&HashMap<String, TranslateObjectValue>> {
        match self {
            LABELS::Object(hash_map) => Ok(hash_map),
            LABELS::Computed(_) => bail!("it's a computed"),
        }
    }

    pub fn get_computed(&self) -> anyhow::Result<&HashSet<String>> {
        match self {
            LABELS::Object(_) => bail!("it's an object"),
            LABELS::Computed(hash_set) => Ok(hash_set),
        }
    }

    // follow the path, then collect all the nested keys
    pub fn get_translation_keys_for_member_expr(
        &self,
        member_expr: &MemberExpr,
    ) -> anyhow::Result<HashSet<String>> {
        let mut obj: &Box<Expr> = &member_expr.obj;
        let mut prop_chain: Vec<&MemberProp> = vec![&member_expr.prop];
        loop {
            match &**obj {
                Expr::Member(member_expr) => {
                    prop_chain.push(&member_expr.prop);
                    obj = &member_expr.obj;
                }
                Expr::Ident(_) => break,
                _ => bail!("member.obj can only be member_expr and ident"),
            }
        }

        let mut labels = self;
        for prop in prop_chain.iter().rev() {
            match prop {
                MemberProp::Ident(ident_name) => {
                    let sym = ident_name.sym.as_str();
                    match labels {
                        LABELS::Object(hash_map) => {
                            let next = hash_map
                                .get(sym)
                                .context(format!("failed to access {}", sym))?;
                            match next {
                                TranslateObjectValue::String(s) => {
                                    // the simplest case, return directly
                                    return Ok(HashSet::from([s.to_owned()]));
                                }
                                TranslateObjectValue::NestedLabels(nested_labels) => {
                                    labels = nested_labels
                                }
                            }
                        }
                        LABELS::Computed(_) => bail!("try to access computed with ident"),
                    }
                }
                MemberProp::Computed(_) => {
                    // once a computed prop found, stop and collect all the translation keys
                    break;
                }
                MemberProp::PrivateName(_) => unimplemented!("what is private name? 🧐"),
            }
        }

        let mut to_collect = vec![labels];
        let mut keys = HashSet::new();
        while to_collect.len() > 0 {
            let mut to_collect_next: Vec<&LABELS> = vec![];
            for labels in to_collect {
                match labels {
                    LABELS::Object(hash_map) => {
                        for v in hash_map.values() {
                            match v {
                                TranslateObjectValue::String(s) => {
                                    keys.insert(s.to_owned());
                                }
                                TranslateObjectValue::NestedLabels(nested_labels) => {
                                    to_collect_next.push(nested_labels)
                                }
                            }
                        }
                    }
                    LABELS::Computed(hash_set) => {
                        keys.extend(hash_set.clone());
                    }
                }
            }
            to_collect = to_collect_next;
        }

        Ok(keys)
    }
}

fn flatten_translation_keys(object_lit: &ObjectLit) -> anyhow::Result<HashSet<String>> {
    let mut translation_keys = HashSet::new();
    for prop_or_spread in object_lit.props.iter() {
        match prop_or_spread {
            PropOrSpread::Prop(prop) => match &**prop {
                Prop::KeyValue(key_value_prop) => match &*key_value_prop.value {
                    Expr::Object(object_lit) => {
                        translation_keys.extend(flatten_translation_keys(object_lit)?);
                    }
                    Expr::Lit(lit) => match lit {
                        Lit::Str(Str { value, .. }) => {
                            translation_keys.insert(value.to_string());
                        }
                        _ => bail!("value can only be string and object literal"),
                    },
                    Expr::Array(array_lit) => {
                        let lazy_key = get_lazy_key_from_array_literal(array_lit)?;
                        translation_keys.insert(lazy_key);
                    }
                    _ => bail!("value can only be string and object literal"),
                },
                _ => bail!("only key-value prop is allowed"),
            },
            PropOrSpread::Spread(_) => bail!("spread is not allowed"),
        }
    }
    Ok(translation_keys)
}

fn insert_key_value_into_labels(
    labels: &mut HashMap<String, TranslateObjectValue>,
    key: String,
    key_value_prop: &KeyValueProp,
) -> anyhow::Result<()> {
    labels.insert(
        key,
        match &*key_value_prop.value {
            Expr::Object(object_lit) => {
                TranslateObjectValue::NestedLabels(collect_labels_from_object_literal(object_lit)?)
            }
            Expr::Lit(lit) => match lit {
                Lit::Str(Str { value, .. }) => TranslateObjectValue::String(value.to_string()),
                _ => bail!("value can only be string and object literal"),
            },
            Expr::Array(array_lit) => {
                let lazy_key = get_lazy_key_from_array_literal(array_lit)?;
                TranslateObjectValue::String(lazy_key)
            }
            _ => bail!("value can only be string and object literal"),
        },
    );
    Ok(())
}

pub fn collect_labels_from_object_literal(object_lit: &ObjectLit) -> anyhow::Result<LABELS> {
    let mut labels = HashMap::new();
    let mut translation_keys = HashSet::new();
    let mut has_computed_key = false;
    for prop_or_spread in object_lit.props.iter() {
        match prop_or_spread {
            PropOrSpread::Prop(prop) => match &**prop {
                Prop::KeyValue(key_value_prop) => match &key_value_prop.key {
                    PropName::Str(s) => {
                        if has_computed_key {
                            bail!(
                                "mixing string with computed keys is not allowed: {}",
                                s.value
                            );
                        }
                        insert_key_value_into_labels(
                            &mut labels,
                            s.value.to_string(),
                            key_value_prop,
                        )?;
                    }
                    PropName::Num(n) => {
                        if has_computed_key {
                            bail!(
                                "mixing number with computed keys is not allowed: {}",
                                n.value
                            );
                        }
                        insert_key_value_into_labels(
                            &mut labels,
                            n.value.to_string(),
                            key_value_prop,
                        )?;
                    }
                    PropName::Ident(id) => {
                        if has_computed_key {
                            bail!("mixing ident with computed keys is not allowed: {}", id.sym);
                        }
                        insert_key_value_into_labels(
                            &mut labels,
                            id.sym.to_string(),
                            key_value_prop,
                        )?;
                    }
                    PropName::Computed(_) => {
                        if labels.len() != 0 {
                            bail!("mixing string and computed keys is not allowed");
                        }
                        has_computed_key = true;
                        match &*key_value_prop.value {
                            Expr::Object(object_lit) => {
                                translation_keys.extend(flatten_translation_keys(object_lit)?);
                            }
                            Expr::Lit(lit) => match lit {
                                Lit::Str(Str { value, .. }) => {
                                    translation_keys.insert(value.to_string());
                                }
                                _ => bail!("value can only be string and object literal"),
                            },
                            Expr::Array(array_lit) => {
                                let lazy_key = get_lazy_key_from_array_literal(array_lit)?;
                                translation_keys.insert(lazy_key);
                            }
                            _ => bail!("value can only be string and object literal"),
                        }
                    }
                    _ => bail!("key can only be string or computed"),
                },
                _ => bail!("only key-value prop is allowed"),
            },
            PropOrSpread::Spread(_) => bail!("spread is not allowed"),
        }
    }

    Ok(match has_computed_key {
        true => LABELS::Computed(translation_keys),
        false => LABELS::Object(labels),
    })
}

fn get_lazy_key_from_array_literal(array_lit: &ArrayLit) -> anyhow::Result<String> {
    if array_lit.elems.len() != 2 {
        bail!("array lit can only be ['<i18n key>', 'lazy']");
    }
    match &*(array_lit.elems[1].as_ref().unwrap().expr) {
        Expr::Lit(lit) => match lit {
            Lit::Str(Str { value, .. }) => {
                if value.to_string() != "lazy" {
                    bail!("array lit can only be ['<i18n key>', 'lazy']");
                }
            }
            _ => bail!("array lit can only be ['<i18n key>', 'lazy']"),
        },
        _ => bail!("array lit can only be ['<i18n key>', 'lazy']"),
    }
    match &*(array_lit.elems[0].as_ref().unwrap().expr) {
        Expr::Lit(lit) => match lit {
            Lit::Str(Str { value, .. }) => return Ok(value.to_string()),
            _ => bail!("array lit can only be ['<i18n key>', 'lazy']"),
        },
        _ => bail!("array lit can only be ['<i18n key>', 'lazy']"),
    }
}

#[cfg(test)]
mod extract_labels_tests {
    use super::*;
    use anyhow::Context;
    use dt_test_utils::parse_module;
    use swc_core::ecma::visit::{Visit, VisitWith};

    struct Visitor {
        object_lit: Option<ObjectLit>,
    }
    impl Visitor {
        pub fn new() -> Self {
            Self { object_lit: None }
        }
    }
    impl Visit for Visitor {
        fn visit_object_lit(&mut self, node: &ObjectLit) {
            self.object_lit = Some(node.clone());
        }
    }

    fn parse_object_lit(input: &str) -> anyhow::Result<ObjectLit> {
        let input = format!("const obj = {}", input);
        let module = parse_module(&input)?;
        let mut visitor = Visitor::new();
        module.visit_with(&mut visitor);
        Ok(visitor.object_lit.context("failed to get object literal")?)
    }

    #[test]
    fn empty_object() {
        let object_lit = parse_object_lit(
            r#"
            {}
            "#,
        )
        .unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        assert_eq!(labels, LABELS::Object(HashMap::new()));
    }

    #[test]
    fn simple_object() {
        let object_lit = parse_object_lit(
            r#"
            {
                bird: "i18n.bird",
                cat: "i18n.cat",
                dog: "i18n.dog",
            }
            "#,
        )
        .unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        let object = labels.get_object().unwrap();
        assert_eq!(
            object.get("bird").unwrap().get_string().unwrap(),
            "i18n.bird"
        );
        assert_eq!(object.get("cat").unwrap().get_string().unwrap(), "i18n.cat");
        assert_eq!(object.get("dog").unwrap().get_string().unwrap(), "i18n.dog");
    }

    #[test]
    fn simple_computed() {
        let object_lit = parse_object_lit(
            r#"
            {
                [PET.bird]: "i18n.bird",
                [PET.cat]: "i18n.cat",
                [PET.dog]: "i18n.dog",
            }
            "#,
        )
        .unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        let computed = labels.get_computed().unwrap();
        assert!(computed.contains("i18n.bird"));
        assert!(computed.contains("i18n.cat"));
        assert!(computed.contains("i18n.dog"));
    }

    #[test]
    #[should_panic(expected = "mixing string and computed keys is not allowed")]
    fn mixed_object_computed() {
        let object_lit = parse_object_lit(
            r#"
            {
                bird: "i18n.bird",
                cat: "i18n.cat",
                dog: "i18n.dog",
                [PET.bird]: "i18n.bird",
                [PET.cat]: "i18n.cat",
                [PET.dog]: "i18n.dog",
            }
            "#,
        )
        .unwrap();
        collect_labels_from_object_literal(&object_lit).unwrap();
    }

    #[test]
    fn nested_object() {
        let object_lit = parse_object_lit(
            r#"
            {
                fly: {
                    bird: "i18n.bird",
                },
                walk: {
                    cat: "i18n.cat",
                    dog: "i18n.dog",
                },
            }
            "#,
        )
        .unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        let object = labels.get_object().unwrap();
        let fly_object = object
            .get("fly")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_object()
            .unwrap();
        let walk_object = object
            .get("walk")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_object()
            .unwrap();
        assert_eq!(
            fly_object.get("bird").unwrap().get_string().unwrap(),
            "i18n.bird"
        );
        assert_eq!(
            walk_object.get("cat").unwrap().get_string().unwrap(),
            "i18n.cat"
        );
        assert_eq!(
            walk_object.get("dog").unwrap().get_string().unwrap(),
            "i18n.dog"
        );
    }

    #[test]
    fn nested_computed() {
        let object_lit = parse_object_lit(
            r#"
            {
                [PET.bird]: {
                    name: "i18n.bird",
                    size: {
                        [SIZE.samll]: "i18n.bird.small",
                        [SIZE.large]: "i18n.bird.large",
                    },
                },
                [PET.cat]: {
                    name: "i18n.cat",
                    size: {
                        [SIZE.samll]: "i18n.cat.small",
                        [SIZE.large]: "i18n.cat.large",
                    },
                },
                [PET.dog]: {
                    name: "i18n.dog",
                    size: {
                        [SIZE.samll]: "i18n.dog.small",
                        [SIZE.large]: "i18n.dog.large",
                    },
                },
            }
            "#,
        )
        .unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        let computed = labels.get_computed().unwrap();
        assert!(computed.contains("i18n.bird"));
        assert!(computed.contains("i18n.bird.small"));
        assert!(computed.contains("i18n.bird.large"));
        assert!(computed.contains("i18n.cat"));
        assert!(computed.contains("i18n.cat.small"));
        assert!(computed.contains("i18n.cat.large"));
        assert!(computed.contains("i18n.dog"));
        assert!(computed.contains("i18n.dog.small"));
        assert!(computed.contains("i18n.dog.large"));
    }

    #[test]
    fn lazy() {
        let object_lit = parse_object_lit(
            r#"
            {
                bird: ["i18n.bird", "lazy"],
                size: {
                    [SIZE.samll]: ["i18n.bird.small", "lazy"],
                    [SIZE.large]: ["i18n.bird.large", "lazy"],
                },
            }
            "#,
        )
        .unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        let object = labels.get_object().unwrap();
        assert_eq!(
            object.get("bird").unwrap().get_string().unwrap(),
            "i18n.bird"
        );
        let size_computed = object
            .get("size")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_computed()
            .unwrap();
        assert!(size_computed.contains("i18n.bird.small"));
        assert!(size_computed.contains("i18n.bird.large"));
    }

    #[test]
    #[should_panic(expected = "array lit can only be ['<i18n key>', 'lazy']")]
    fn lazy_wrong_format_1() {
        let object_lit = parse_object_lit(
            r#"
            {
                bird: ["i18n.bird"],
            }
            "#,
        )
        .unwrap();
        collect_labels_from_object_literal(&object_lit).unwrap();
    }

    #[test]
    #[should_panic(expected = "array lit can only be ['<i18n key>', 'lazy']")]
    fn lazy_wrong_format_2() {
        let object_lit = parse_object_lit(
            r#"
            {
                bird: ["i18n.bird", "kirby"],
            }
            "#,
        )
        .unwrap();
        collect_labels_from_object_literal(&object_lit).unwrap();
    }

    #[test]
    #[should_panic(expected = "array lit can only be ['<i18n key>', 'lazy']")]
    fn lazy_wrong_format_3() {
        let object_lit = parse_object_lit(
            r#"
            {
                bird: ["i18n.bird", "lazy", "kirby"],
            }
            "#,
        )
        .unwrap();
        collect_labels_from_object_literal(&object_lit).unwrap();
    }

    #[test]
    fn complex() {
        let object_lit = parse_object_lit(
            r#"
            {
                title: "i18n.pet.party",
                desc: ["i18n.pet.party.desc", "lazy"],
                bird: {
                    name: "i18n.bird",
                    desc: ["i18n.bird.desc", "lazy"],
                    size: {
                        [SIZE.samll]: "i18n.bird.small",
                        [SIZE.large]: ["i18n.bird.large", "lazy"],
                    },
                },
                cat: {
                    name: "i18n.cat",
                    desc: ["i18n.cat.desc", "lazy"],
                    size: {
                        [SIZE.samll]: "i18n.cat.small",
                        [SIZE.large]: ["i18n.cat.large", "lazy"],
                    },
                },
                dog: {
                    name: "i18n.dog",
                    desc: ["i18n.dog.desc", "lazy"],
                    size: {
                        [SIZE.samll]: "i18n.dog.small",
                        [SIZE.large]: ["i18n.dog.large", "lazy"],
                    },
                },
            }
            "#,
        )
        .unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        let object = labels.get_object().unwrap();
        assert_eq!(
            object.get("title").unwrap().get_string().unwrap(),
            "i18n.pet.party"
        );
        assert_eq!(
            object.get("desc").unwrap().get_string().unwrap(),
            "i18n.pet.party.desc"
        );

        let bird_object = object
            .get("bird")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_object()
            .unwrap();
        assert_eq!(
            bird_object.get("name").unwrap().get_string().unwrap(),
            "i18n.bird"
        );
        assert_eq!(
            bird_object.get("desc").unwrap().get_string().unwrap(),
            "i18n.bird.desc"
        );
        let bird_size_computed = bird_object
            .get("size")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_computed()
            .unwrap();
        assert!(bird_size_computed.contains("i18n.bird.small"));
        assert!(bird_size_computed.contains("i18n.bird.large"));

        let cat_object = object
            .get("cat")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_object()
            .unwrap();
        assert_eq!(
            cat_object.get("name").unwrap().get_string().unwrap(),
            "i18n.cat"
        );
        assert_eq!(
            cat_object.get("desc").unwrap().get_string().unwrap(),
            "i18n.cat.desc"
        );
        let cat_size_computed = cat_object
            .get("size")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_computed()
            .unwrap();
        assert!(cat_size_computed.contains("i18n.cat.small"));
        assert!(cat_size_computed.contains("i18n.cat.large"));

        let dog_object = object
            .get("dog")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_object()
            .unwrap();
        assert_eq!(
            dog_object.get("name").unwrap().get_string().unwrap(),
            "i18n.dog"
        );
        assert_eq!(
            dog_object.get("desc").unwrap().get_string().unwrap(),
            "i18n.dog.desc"
        );
        let dog_size_computed = dog_object
            .get("size")
            .unwrap()
            .get_labels()
            .unwrap()
            .get_computed()
            .unwrap();
        assert!(dog_size_computed.contains("i18n.dog.small"));
        assert!(dog_size_computed.contains("i18n.dog.large"));
    }
}

#[cfg(test)]
mod access_labels_tests {
    use super::*;
    use anyhow::Context;
    use dt_test_utils::parse_module;
    use swc_core::ecma::visit::{Visit, VisitWith};

    struct ObjectLitVisitor {
        object_lit: Option<ObjectLit>,
    }
    impl ObjectLitVisitor {
        pub fn new() -> Self {
            Self { object_lit: None }
        }
    }
    impl Visit for ObjectLitVisitor {
        fn visit_object_lit(&mut self, node: &ObjectLit) {
            self.object_lit = Some(node.clone());
        }
    }
    fn parse_object_lit(input: &str) -> anyhow::Result<ObjectLit> {
        let module = parse_module(&input)?;
        let mut object_lit_visitor = ObjectLitVisitor::new();
        module.visit_with(&mut object_lit_visitor);
        Ok(object_lit_visitor
            .object_lit
            .context("failed to get object literal")?)
    }

    struct MemberExprVisitor {
        member_expr: Option<MemberExpr>,
    }
    impl MemberExprVisitor {
        pub fn new() -> Self {
            Self { member_expr: None }
        }
    }
    impl Visit for MemberExprVisitor {
        fn visit_member_expr(&mut self, node: &MemberExpr) {
            self.member_expr = Some(node.clone());
        }
    }
    fn parse_member_expr(input: &str) -> anyhow::Result<MemberExpr> {
        let input = format!("const foo = {}", input);
        let module = parse_module(&input)?;
        let mut member_expr_visitor = MemberExprVisitor::new();
        module.visit_with(&mut member_expr_visitor);
        Ok(member_expr_visitor
            .member_expr
            .context("failed to get member expression")?)
    }

    macro_rules! assert_keys {
        ($labels:expr, $($member_expr:expr => $expected_keys:expr),* $(,)?) => {{
            let object_lit = parse_object_lit($labels).unwrap();
            let labels = collect_labels_from_object_literal(&object_lit).unwrap();

            $(
                let member_expr = parse_member_expr($member_expr).unwrap();
                let keys = labels
                    .get_translation_keys_for_member_expr(&member_expr)
                    .unwrap();

                assert_eq!(keys.len(), $expected_keys.len(), "keys count mismatch");
                for &expected_key in $expected_keys.iter() {
                    assert!(keys.contains(expected_key), "missing key: {}", expected_key);
                }
            )*
        }};
    }

    #[test]
    #[should_panic(expected = "failed to access a")]
    fn access_invalid_prop() {
        let object_lit = parse_object_lit(
            r#"
            const LABELS = {}
            "#,
        )
        .unwrap();
        let member_expr = parse_member_expr("LABELS.a.b.c").unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        labels
            .get_translation_keys_for_member_expr(&member_expr)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "try to access computed with ident")]
    fn access_computed_with_ident() {
        let object_lit = parse_object_lit(
            r#"
            const LABELS = {
                [PET.bird]: "i18n.bird",
                [PET.cat]: "i18n.cat",
                [PET.dog]: "i18n.dog",
            }
            "#,
        )
        .unwrap();
        let member_expr = parse_member_expr("LABELS.bird").unwrap();
        let labels = collect_labels_from_object_literal(&object_lit).unwrap();
        labels
            .get_translation_keys_for_member_expr(&member_expr)
            .unwrap();
    }

    #[test]
    fn simple_object() {
        assert_keys!(
            r#"
            const LABELS = {
                bird: "i18n.bird",
                cat: "i18n.cat",
                dog: "i18n.dog",
            };
            "#,
            "LABELS.bird" => ["i18n.bird"],
            "LABELS.cat" => ["i18n.cat"],
            "LABELS.dog" => ["i18n.dog"],
            "LABELS[type]" => ["i18n.bird", "i18n.cat", "i18n.dog"]
        );
    }

    #[test]
    fn simple_computed() {
        assert_keys!(
            r#"
            const LABELS = {
                [PET.bird]: "i18n.bird",
                [PET.cat]: "i18n.cat",
                [PET.dog]: "i18n.dog",
            }
            "#,
            "LABELS[type]" => ["i18n.bird", "i18n.cat", "i18n.dog"],
        );
    }

    #[test]
    fn complex() {
        assert_keys!(
            r#"
            const LABELS = {
                title: "i18n.pet.party",
                desc: ["i18n.pet.party.desc", "lazy"],
                bird: {
                    name: "i18n.bird",
                    desc: ["i18n.bird.desc", "lazy"],
                    size: {
                        [SIZE.samll]: "i18n.bird.small",
                        [SIZE.large]: ["i18n.bird.large", "lazy"],
                    },
                },
                cat: {
                    name: "i18n.cat",
                    desc: ["i18n.cat.desc", "lazy"],
                    size: {
                        [SIZE.samll]: "i18n.cat.small",
                        [SIZE.large]: ["i18n.cat.large", "lazy"],
                    },
                },
                dog: {
                    name: "i18n.dog",
                    desc: ["i18n.dog.desc", "lazy"],
                    size: {
                        [SIZE.samll]: "i18n.dog.small",
                        [SIZE.large]: ["i18n.dog.large", "lazy"],
                    },
                },
            }
            "#,
            "LABELS.title" => ["i18n.pet.party"],
            "LABELS.desc" => ["i18n.pet.party.desc"],

            "LABELS.bird.name" => ["i18n.bird"],
            "LABELS.bird.desc({ name: 'シマエナガ' })" => ["i18n.bird.desc"],
            "LABELS.bird.size" => ["i18n.bird.small", "i18n.bird.large"],
            "LABELS.bird.size[type]" => ["i18n.bird.small", "i18n.bird.large"],
            "LABELS.bird.size[type]({ name: 'シマエナガ' })" => ["i18n.bird.small", "i18n.bird.large"],
            "LABELS.bird" => [ "i18n.bird", "i18n.bird.desc", "i18n.bird.small", "i18n.bird.large"],

            "LABELS.cat.name" => ["i18n.cat"],
            "LABELS.cat.desc({ name: '貓咪' })" => ["i18n.cat.desc"],
            "LABELS.cat.size" => ["i18n.cat.small", "i18n.cat.large"],
            "LABELS.cat.size[type]" => ["i18n.cat.small", "i18n.cat.large"],
            "LABELS.cat.size[type]({ name: '貓咪' })" => ["i18n.cat.small", "i18n.cat.large"],
            "LABELS.cat" => [ "i18n.cat", "i18n.cat.desc", "i18n.cat.small", "i18n.cat.large"],

            "LABELS.dog.name" => ["i18n.dog"],
            "LABELS.dog.desc({ name: 'Oatchi' })" => ["i18n.dog.desc"],
            "LABELS.dog.size" => ["i18n.dog.small", "i18n.dog.large"],
            "LABELS.dog.size[type]" => ["i18n.dog.small", "i18n.dog.large"],
            "LABELS.dog.size[type]({ name: 'Oatchi' })" => ["i18n.dog.small", "i18n.dog.large"],
            "LABELS.dog" => [ "i18n.dog", "i18n.dog.desc", "i18n.dog.small", "i18n.dog.large"],

            "LABELS[type]" => [
                "i18n.pet.party",
                "i18n.pet.party.desc",
                "i18n.bird",
                "i18n.bird.desc",
                "i18n.bird.small",
                "i18n.bird.large",
                "i18n.cat",
                "i18n.cat.desc",
                "i18n.cat.small",
                "i18n.cat.large",
                "i18n.dog",
                "i18n.dog.desc",
                "i18n.dog.small",
                "i18n.dog.large",
            ],

            "LABELS[type].size.small" => [
                "i18n.pet.party",
                "i18n.pet.party.desc",
                "i18n.bird",
                "i18n.bird.desc",
                "i18n.bird.small",
                "i18n.bird.large",
                "i18n.cat",
                "i18n.cat.desc",
                "i18n.cat.small",
                "i18n.cat.large",
                "i18n.dog",
                "i18n.dog.desc",
                "i18n.dog.small",
                "i18n.dog.large",
            ]
        );
    }
}
