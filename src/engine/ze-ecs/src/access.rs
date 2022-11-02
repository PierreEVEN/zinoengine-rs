use bitvec::vec::BitVec;

/// Track read/writes accesses to a data structure
#[derive(Default, Clone, Debug)]
pub struct Access {
    readen_elements: BitVec,
    written_elements: BitVec,
}

impl Access {
    pub fn add_read(&mut self, index: usize) {
        self.grow(index);
        self.readen_elements.resize(index + 1, false);
        self.readen_elements.set(index, true);
    }

    pub fn add_write(&mut self, index: usize) {
        self.grow(index);
        self.readen_elements.set(index, true);
        self.written_elements.set(index, true);
    }

    pub fn clear(&mut self) {
        self.readen_elements.clear();
        self.written_elements.clear();
    }

    pub fn union(&mut self, other: &Self) {
        self.resize(other.len());
        self.readen_elements |= &other.readen_elements;
        self.written_elements |= &other.written_elements;
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.readen_elements
            .iter()
            .zip(&other.written_elements)
            .all(|(a, b)| !(*a & *b))
            && self
                .written_elements
                .iter()
                .zip(&other.readen_elements)
                .all(|(a, b)| !(*a & *b))
    }

    pub fn resize(&mut self, new_len: usize) {
        self.readen_elements.resize(new_len, false);
        self.written_elements.resize(new_len, false);
    }

    pub fn grow(&mut self, additional: usize) {
        self.resize(self.readen_elements.len() + additional + 1)
    }

    pub fn len(&self) -> usize {
        self.readen_elements.len()
    }

    pub fn has_read(&self, index: usize) -> bool {
        self.readen_elements[index]
    }

    pub fn has_write(&self, index: usize) -> bool {
        self.written_elements[index]
    }
}

#[cfg(test)]
mod tests {
    use crate::access::Access;

    #[test]
    fn is_disjoint() {
        let mut a = Access::default();
        let mut b = Access::default();

        a.add_read(0);
        a.add_read(1);
        a.add_read(2);
        a.add_write(3);

        b.add_write(0);
        b.add_read(1);
        b.add_write(2);
        b.add_write(3);

        assert!(!a.is_disjoint(&b));
    }

    #[test]
    fn union() {
        let mut a = Access::default();
        let mut b = Access::default();

        a.add_read(0);
        a.add_read(1);
        a.add_read(2);
        a.add_write(3);
        a.add_read(4);

        b.add_write(0);
        b.add_read(1);
        b.add_write(2);
        b.add_write(3);
        b.add_read(4);

        a.union(&b);

        assert!(a.has_write(0));
        assert!(a.has_read(1));
        assert!(a.has_write(2));
        assert!(a.has_write(3));
        assert!(a.has_read(4));
    }
}
