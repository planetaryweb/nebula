use std::collections::HashMap;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub enum Value {
    Leaf(String),
    Node(Config),
}

pub type Config = HashMap<String, Value>;

pub trait ConfigExt {
    fn get_path(&self, key: &str) -> Option<&str>;
}

impl ConfigExt for Config {
    fn get_path(&self, key: &str) -> Option<&str> {
        let path = key.split('.');
        let mut cfg = self;
        let mut txt = None;

        for seg in path {
            match cfg.get(seg) {
                None => return None,
                Some(val) => match val {
                    Value::Leaf(text) => txt = Some(text.as_ref()),
                    Value::Node(conf) => cfg = conf
                }
            }
        }

        txt
    }
}
