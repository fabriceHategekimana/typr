use serde::Serialize;
use crate::Lang;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArgumentValue(pub String, pub Lang);

impl ArgumentValue {
    pub fn to_r(&self) -> String {
        format!("{} = {}", self.0, self.1.to_r())
    }

    pub fn get_argument(&self) -> String {
        self.0.clone()
    }

    pub fn get_value(&self) -> Lang {
        self.1.clone()
    }
}

impl fmt::Display for ArgumentValue {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[var('{}'),{}]", self.0, self.1)       
    }
}

