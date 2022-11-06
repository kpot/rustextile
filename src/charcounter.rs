use std::ops::{Index, IndexMut};

pub(crate) struct CharCounter<const N: usize> {
    chars: [char; N],
    data: [Option<usize>; N],
}

impl <const N: usize> CharCounter<N> {
    pub fn new(chars: [char; N]) -> Self {
        Self {
            chars,
            data: [None; N]
        }
    }

    pub fn dec(&mut self, c: char) {
        let data = &mut self[c];
        *data = match *data {
            Some(v) => Some(if v > 0 { v - 1 } else { 0 }),
            None => panic!("Trying to decrease an unitialized counter: '{}'", c),
        };
    }
}

impl <const N:usize> Index<char> for CharCounter<N> {
    type Output = Option<usize>;

    fn index(&self, index: char) -> &Self::Output {
        for (c, d) in self.chars.iter().zip(self.data.iter()) {
            if *c == index {
                return d;
            }
        }
        panic!("Counter has encountered an uncountable character");
    }
}


impl <const N:usize> IndexMut<char> for CharCounter<N> {
    fn index_mut(&mut self, index: char) -> &mut Self::Output {
        for (c, d) in self.chars.iter().zip(self.data.iter_mut()) {
            if *c == index {
                return d;
            }
        }
        panic!("Counter has encountered an uncountable character");
    }
}


#[cfg(test)]
mod test {
    use super::CharCounter;

    #[test]
    fn test_char_counter() {
        let mut c = CharCounter::new(['[', ']', '(', ')']);
        assert_eq!(c.chars.len(), 4);
        assert_eq!(c['['], None);
        c['['] = Some(1);
        c[']'] = Some(2);
        assert_eq!(c['['], Some(1));
        assert_eq!(c[']'], Some(2));
        c.dec('[');
        assert_eq!(c['['], Some(0));
    }

    #[test]
    #[should_panic(expected="Counter has encountered an uncountable character")]
    fn test_invalid_char_counter() {
        let c = CharCounter::new(['[', ']', '(', ')']);
        assert_eq!(c['x'], None)
    }
}
