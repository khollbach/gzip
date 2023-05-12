use std::collections::HashMap;

type Code = Vec<bool>;

pub(crate) trait HuffmanEncoding {
    fn add_code(&mut self, code: Code, val: u8);
    fn lookup(&self, code: &Code) -> Option<u8>;
}

pub struct HuffmanMap {
    map: HashMap<Code, u8>,
}

impl HuffmanMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl HuffmanEncoding for HuffmanMap {
    fn add_code(&mut self, code: Code, val: u8) {
        self.map.insert(code, val);
    }

    fn lookup(&self, code: &Code) -> Option<u8> {
        self.map.get(code).copied()
    }
}
