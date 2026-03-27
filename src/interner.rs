use std::collections::HashMap;
use std::sync::Arc;
use crate::types::StringId;

pub struct Interner {
    map: HashMap<Arc<str>, StringId>,
    vec: Vec<Arc<str>>,
}

impl Interner {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            vec: Vec::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.map.get(s) {
            return id;
        }

        let id = self.vec.len() as StringId;
        let rc_string: Arc<str> = Arc::from(s);

        self.vec.push(rc_string.clone());
        self.map.insert(rc_string, id);

        id
    }

    pub fn resolve(&self, id: StringId) -> Option<&str> {
        self.vec.get(id as usize).map(|rc| rc.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_same_string() {
        let mut i = Interner::new();
        let a = i.intern("hello");
        let b = i.intern("hello");
        assert_eq!(a, b);
    }

    #[test]
    fn test_intern_different_strings() {
        let mut i = Interner::new();
        let a = i.intern("hello");
        let b = i.intern("world");
        assert_ne!(a, b);
    }

    #[test]
    fn test_resolve() {
        let mut i = Interner::new();
        let id = i.intern("test");
        let s = i.resolve(id).unwrap();
        assert_eq!(s, "test");
    }
}
