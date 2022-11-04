macro_rules! impl_component {
    ($type_name:ident -> $($component_name:ident),*) => {
        #[repr(C)]
        #[derive(Debug, Default, Copy, Clone, PartialEq)]
        pub struct $type_name<T> {
            $(pub $component_name: T),*
        }
    };
}

#[macro_export]
macro_rules! impl_component_deref {
    ($type_name:ident $R:literal -> $target_name:ident) => {
        impl<T: MatrixNumber> Deref for $type_name<T, $R> {
            type Target = $target_name<T>;

            #[inline]
            fn deref(&self) -> &Self::Target {
                unsafe { &*(self as *const Self as *const Self::Target) }
            }
        }

        impl<T: MatrixNumber> DerefMut for $type_name<T, $R> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { &mut *(self as *mut Self as *mut Self::Target) }
            }
        }
    };
}

impl_component!(X -> x);
impl_component!(XY -> x, y);
impl_component!(XYZ -> x, y, z);
impl_component!(XYZW -> x, y, z, w);
