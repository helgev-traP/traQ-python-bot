use std::collections::HashMap;

use regex::Captures;

pub struct Parser {
    regex: HashMap<String, regex::Regex>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            regex: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: impl Into<String>, pattern: impl AsRef<str>) {
        self.regex
            .insert(name.into(), regex::Regex::new(pattern.as_ref()).unwrap());
    }

    pub fn remove(&mut self, name: impl AsRef<str>) -> Option<regex::Regex> {
        self.regex.remove(name.as_ref())
    }

    /// Parse the source string and return ({pattern_name}, {captures})
    pub fn parse<'a>(&self, src: &'a impl AsRef<str>) -> Option<(String, Captures<'a>)> {
        for (name, re) in &self.regex {
            if let Some(caps) = re.captures(src.as_ref()) {
                return Some((name.to_string(), caps));
            }
        }
        None
    }
}
