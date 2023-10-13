use std::collections::HashMap;
use bitvec::vec::BitVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Sym(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Code {
    length: u8,
    bits: u16,
}

#[derive(Debug)]
struct HuffmanCodeMap {
    code_to_sym: HashMap<Code, Sym>,
    sym_to_code: HashMap<Sym, Code>,
}

impl HuffmanCodeMap {
    fn from_symbol_depths(symbol_depths: &[(Sym, u8)]) -> Self {
        // Is it sorted increasing by (depth, symbol) ?
        assert!(symbol_depths.windows(2).all(|pair| {
            let (Sym(s1), d1) = pair[0];
            let (Sym(s2), d2) = pair[1];
            (d1, s1) < (d2, s2)
        }));

        let mut code_to_sym = HashMap::new();
        let mut sym_to_code = HashMap::new();

        let mut prev_depth = 0;
        let mut code = 0u16;
        for &(symbol, depth) in symbol_depths {
            if depth > prev_depth {
                // Append a '0' bit.
                code <<= 1;

            }

            code_to_sym.insert(
                Code {
                    length: depth,
                    bits: code,
                },
                symbol,
            );
            sym_to_code.insert(
                symbol,
                Code {
                    length: depth,
                    bits: code,
                },
            );


            code += 1;
            prev_depth = depth;
        }

        Self { code_to_sym, sym_to_code }
    }

    fn encode_string(&self, string_slice: &[Sym]) -> BitVec {
        let bitvec = BitVec::new();
        for &sym in string_slice {
            let code = self.sym_to_code[sym];
            // Code { length: 3, bits: 1 } -> 001
            // push from the most significant bit into bitvec
            // find the most signicant 1 bit in Code.bits (this will give us how many bits are needed to represent that number)
            // take the difference between Code.length and the number of bits to represent bits
            // that's how many 0s we need to push into bitvec
            // then we can push 

        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example() {
        let symbol_depths = [
            (b'a', 2),
            (b'b', 2),
            (b'c', 3),
            (b'e', 3),
            (b'g', 3),
            (b'd', 4),
            (b'f', 4),
        ]
        .map(|(b, d)| (Sym(b as u16), d));
        let actual = HuffmanCodeMap::from_symbol_depths(&symbol_depths);

        let expected: HashMap<_, _> = [
            (2, 0b_00, b'a'),
            (2, 0b_01, b'b'),
            (3, 0b_100, b'c'),
            (3, 0b_101, b'e'),
            (3, 0b_110, b'g'),
            (4, 0b_1110, b'd'),
            (4, 0b_1111, b'f'),
        ]
        .into_iter()
        .map(|(length, bits, b)| (Code { length, bits }, Sym(b as u16)))
        .collect();

        dbg!(&actual);

        assert_eq!(actual.code_to_sym, expected);
    }
}
