use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    const NONEXISTENT_KEY: &str = "jabberwocky";
    const TOP_LEVEL_KEY: &str = "foo";
    const TOP_LEVEL_VAL_KEY: &str = "top";
    const TOP_LEVEL_VAL: &str = "top val";
    const SECOND_LEVEL_KEY: &str = "bar";
    const THIRD_LEVEL_KEY: &str = "baz";
    const THIRD_LEVEL_VAL_KEY: &str = "val";
    const THIRD_LEVEL_VAL_PATH: &str = "foo.bar.val";
    const THIRD_LEVEL_VAL: &str = "12";
    const THIRD_LEVEL_INT: i32 = 12;
    const FOURTH_LEVEL_VAL_PATH: &str = "foo.bar.baz";
    const FOURTH_LEVEL_VAL: &str = "quux";

    fn get_config() -> Config {
        let mut inner3 = Config::new();
        inner3.insert(
            THIRD_LEVEL_KEY.to_string(),
            Value::Leaf(FOURTH_LEVEL_VAL.to_string()),
        );
        inner3.insert(
            THIRD_LEVEL_VAL_KEY.to_string(),
            Value::Leaf(THIRD_LEVEL_VAL.to_string()),
        );
        let mut inner2 = Config::new();
        inner2.insert(SECOND_LEVEL_KEY.to_string(), Value::Node(inner3));
        let mut config = Config::new();
        config.insert(TOP_LEVEL_KEY.to_string(), Value::Node(inner2));
        config.insert(
            TOP_LEVEL_VAL_KEY.to_string(),
            Value::Leaf(TOP_LEVEL_VAL.to_string()),
        );
        config
    }

    #[test]
    fn get_path_top_level_key() {
        let config = get_config();
        let top_val = config
            .get_path(TOP_LEVEL_VAL_KEY)
            .expect("top-level key should not error");
        assert_eq!(top_val, Some(TOP_LEVEL_VAL.to_string()));
    }

    #[test]
    fn get_path_nested() {
        let config = get_config();
        let val = config
            .get_path(FOURTH_LEVEL_VAL_PATH)
            .expect("nested existing key should not error");
        assert_eq!(val, Some(FOURTH_LEVEL_VAL.to_string()));
    }

    #[test]
    fn get_path_int_from_str() {
        let config = get_config();
        let val = config
            .get_path(THIRD_LEVEL_VAL_PATH)
            .expect("nested existing key should not error");
        assert_eq!(val, Some(THIRD_LEVEL_INT));
    }

    #[test]
    fn get_path_missing_key_is_none() {
        let config = get_config();
        let result: Option<String> = config
            .get_path(NONEXISTENT_KEY)
            .expect("missing key should return Ok(None), not an error");
        assert_eq!(result, None);
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum Value {
    Leaf(String),
    Node(Config),
}

pub type Config = HashMap<String, Value>;

pub trait ConfigExt {
    fn get_path<T, U>(&self, key: &str) -> Result<Option<T>, String>
    where
        T: FromStr<Err = U>,
        U: std::fmt::Display + ToString;
}

impl ConfigExt for Config {
    fn get_path<T, U>(&self, key: &str) -> Result<Option<T>, String>
    where
        T: FromStr<Err = U>,
        U: std::fmt::Display + ToString,
    {
        let path = key.split('.');
        let mut cfg = self;
        let mut txt = None;

        for seg in path {
            match cfg.get(seg) {
                None => return Ok(None),
                Some(val) => match val {
                    Value::Leaf(text) => txt = Some(text.as_ref()),
                    Value::Node(conf) => cfg = conf,
                },
            }
        }

        txt.map(str::parse)
            .transpose()
            .map_err(|err: U| err.to_string())
    }
}
