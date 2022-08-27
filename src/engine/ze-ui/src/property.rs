/// An property that can be binded to a constant or closure
pub enum Property<T> {
    Constant(T),
}

impl<T> Property<T> {
    pub fn new_constant(constant: T) -> Self {
        Self::Constant(constant)
    }

    pub fn get(&self) -> &T {
        match self {
            Property::Constant(value) => &value,
        }
    }
}

impl<T: Default> Default for Property<T> {
    fn default() -> Self {
        Self::Constant(T::default())
    }
}

impl<T> From<T> for Property<T> {
    fn from(value: T) -> Self {
        Self::Constant(value)
    }
}
