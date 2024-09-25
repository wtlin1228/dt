#[macro_export]
macro_rules! assert_hash_map {
    ($hash_map:expr, $(($key:expr, $value:expr)),* $(,)?) => {{
        let mut count = 0;
        $(
            count += 1;
            assert!($hash_map.contains_key($key), "missing key: {}", $key);
            assert_eq!($hash_map.get($key).unwrap(), &$value, "value mismatch for key {}", $key);
        )*
        assert_eq!($hash_map.len(), count, "entry count mismatch");
    }};
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn it_works() {
        let hash_map = HashMap::from([("a", "apple"), ("k", "kirby")]);
        assert_hash_map!(hash_map, ("a", "apple"), ("k", "kirby"));
    }

    #[test]
    #[should_panic = "missing key: h"]
    fn missing_key() {
        let hash_map = HashMap::from([("a", "apple"), ("k", "kirby")]);
        assert_hash_map!(hash_map, ("a", "apple"), ("k", "kirby"), ("h", "hawk"));
    }

    #[test]
    #[should_panic = "entry count mismatch"]
    fn count_mismatch() {
        let hash_map = HashMap::from([("a", "apple"), ("k", "kirby")]);
        assert_hash_map!(hash_map, ("k", "kirby"),);
    }

    #[test]
    #[should_panic = "value mismatch for key a"]
    fn value_mismatch() {
        let hash_map = HashMap::from([("a", "apple"), ("k", "kirby")]);
        assert_hash_map!(hash_map, ("a", "abby"), ("k", "kirby"),);
    }
}
