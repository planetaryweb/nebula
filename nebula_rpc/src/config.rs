use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use serde::Deserialize;

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
        inner3.insert(THIRD_LEVEL_KEY.to_string(), Value::LeafSingle(FOURTH_LEVEL_VAL.to_string()));
        inner3.insert(THIRD_LEVEL_VAL_KEY.to_string(), Value::LeafSingle(THIRD_LEVEL_VAL.to_string()));
        let mut inner2 = Config::new();
        inner2.insert(SECOND_LEVEL_KEY.to_string(), Value::Node(inner3));
        let mut config = Config::new();
        config.insert(TOP_LEVEL_KEY.to_string(), Value::Node(inner2));
        config.insert(TOP_LEVEL_VAL_KEY.to_string(), Value::LeafSingle(TOP_LEVEL_VAL.to_string()));
        config
    }

    #[test]
    fn get_path_top_level_key() {
        let config = get_config();
        let top_val = config.get_path::<String>(TOP_LEVEL_VAL_KEY).expect("top-level key should not error");
        assert_eq!(top_val, Some(&Value::LeafSingle(TOP_LEVEL_VAL.to_string())));
    }

    #[test]
    fn get_path_nested() {
        let config = get_config();
        let val = config.get_path::<String>(FOURTH_LEVEL_VAL_PATH).expect("nested existing key should not error");
        assert_eq!(val, Some(&Value::LeafSingle(FOURTH_LEVEL_VAL.to_string())));
    }

    #[test]
    fn get_path_int_from_str() {
        let config = get_config();
        let val = config.get_path::<String>(THIRD_LEVEL_VAL_PATH).expect("nested existing key should not error");
        assert_eq!(val, Some(&Value::LeafSingle(THIRD_LEVEL_INT.to_string())));
    }

    #[test]
    fn get_path_missing_key_is_none() {
        let config = get_config();
        let result = config.get_path::<String>(NONEXISTENT_KEY).expect("missing key should return Ok(None), not an error");
        assert_eq!(result, None);
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum Value {
    LeafSingle(String),
    LeafList(Vec<String>),
    Node(Config),
}

pub type Config = HashMap<String, Value>;

#[derive(Debug)]
pub enum PathError<U> {
    EndedEarly(String),
    IsList,
    IsMap,
    IsSingle,
    Parse(U),
}

impl<U: fmt::Display> fmt::Display for PathError<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndedEarly(path) => write!(f, "failed to read entire path: {}", path),
            Self::IsList => write!(f, "expected a list"),
            Self::IsMap => write!(f, "expected a map"),
            Self::IsSingle => write!(f, "expected a single string/number/etc. value"),
            Self::Parse(err) => write!(f, "parse error: {}", err),
        }
    }
}

pub trait ConfigExt {
    fn get_path<U>(&self, key: &str) -> Result<Option<&Value>, PathError<U>>;

    fn get_path_list<T, U, V>(&self, key: &str) -> Result<Option<V>, PathError<U>>
        where T: FromStr<Err = U>, U: std::fmt::Display + ToString, V: std::iter::FromIterator<T>;

    fn get_path_single<T, U>(&self, key: &str) -> Result<Option<T>, PathError<U>>
        where T: FromStr<Err = U>, U: std::fmt::Display + ToString;
}

impl ConfigExt for Config {
    fn get_path<U>(&self, key: &str) -> Result<Option<&Value>, PathError<U>> {
        let path = key.split('.');
        let mut cfg = self;
        let mut v = None;

        for seg in path.clone() {
            if let Some(val) = v {
                match val {
                    &Value::Node(_) => {},
                    _ => return Err(PathError::EndedEarly(path.collect::<Vec<&str>>().join("."))),
                }
            }

            match cfg.get(seg) {
                None => return Ok(None),
                Some(val) => {
                    if let Value::Node(conf) = val {
                        cfg = &conf
                    }
                    v = Some(val);
                }
            }
        }

        Ok(v)
    }

    fn get_path_list<T, U, V>(&self, key: &str) -> Result<Option<V>, PathError<U>>
        where T: FromStr<Err = U>, U: std::fmt::Display + ToString, V: std::iter::FromIterator<T> {
        self.get_path(key)?.map(|val| match val {
            Value::LeafSingle(_) => Err(PathError::IsSingle),
            Value::Node(_) => Err(PathError::IsMap),
            Value::LeafList(list) => list.into_iter()
                .map(|txt| txt.parse::<T>().map_err(PathError::Parse))
                .collect::<Result<V, PathError<U>>>()
        }).transpose()
    }

    fn get_path_single<T, U>(&self, key: &str) -> Result<Option<T>, PathError<U>>
        where T: FromStr<Err = U>, U: std::fmt::Display + ToString {
        self.get_path(key)?.map(|val| match val {
            Value::LeafSingle(txt) => txt.parse().map_err(PathError::Parse),
            Value::LeafList(_) => Err(PathError::IsList),
            Value::Node(_) => Err(PathError::IsMap),
        }).transpose()
    }
}
