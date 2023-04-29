use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FullName {
    pub first: String,
    pub last: String,
}

impl FullName {
    pub fn new(first: String, last: String) -> Self {
        Self { first, last }
    }

    pub fn try_new(first: Option<String>, last: Option<String>) -> Option<Self> {
        match (first, last) {
            (Some(first), Some(last)) => Some(Self::new(first, last)),
            _ => None,
        }
    }

    pub fn try_from_str(name: &str) -> Option<Self> {
        let mut split = name.split_whitespace();
        match (split.next(), split.next()) {
            (Some(first), Some(last)) => Some(Self::new(first.to_string(), last.to_string())),
            (Some(first), None) => Some(Self::new(first.to_string(), String::new())),
            (None, Some(last)) => Some(Self::new(String::new(), last.to_string())),
            (None, None) => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.first.is_empty() && self.last.is_empty()
    }

    pub fn name(&self) -> String {
        if self.is_empty() {
            return String::from("Unknown user name");
        }
        format!("{} {}", self.first, self.last)
    }
}
