use num_traits::FromPrimitive;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
pub use std::sync::Arc;
pub use ze_reflection_derive::*;

static INTERNAL_TYPE_DATABASE: Lazy<RwLock<HashMap<&'static str, Arc<TypeDescription>>>> =
    Lazy::new(Default::default);

pub struct MetaAttribute {
    name: String,
    value: Option<MetaAttributeValue>,
}

impl MetaAttribute {
    pub fn value(&self) -> &Option<MetaAttributeValue> {
        &self.value
    }
}

pub struct MetaAttributeList {
    inner: Vec<MetaAttribute>,
}

impl MetaAttributeList {
    pub fn new(inner: Vec<MetaAttribute>) -> Self {
        Self { inner }
    }

    pub fn attribute(&self, name: &str) -> Option<&MetaAttribute> {
        self.inner.iter().find(|attr| attr.name == name)
    }

    pub fn has_attribute(&self, name: &str) -> bool {
        self.inner.iter().any(|attr| attr.name == name)
    }
}

impl MetaAttribute {
    pub fn new(name: String, value: Option<MetaAttributeValue>) -> Self {
        Self { name, value }
    }
}

pub enum MetaAttributeValue {
    Value(String),
    List(MetaAttributeList),
}

pub struct Field {
    name: String,
    offset_in_bytes: usize,
    ty: Arc<TypeDescription>,
    meta_attributes: MetaAttributeList,
}

impl Field {
    pub fn new(
        name: String,
        offset_in_bytes: usize,
        ty: Arc<TypeDescription>,
        meta_attributes: MetaAttributeList,
    ) -> Self {
        Self {
            name,
            offset_in_bytes,
            ty,
            meta_attributes,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn offset_in_bytes(&self) -> usize {
        self.offset_in_bytes
    }

    pub fn ty(&self) -> &Arc<TypeDescription> {
        &self.ty
    }

    pub fn attributes(&self) -> &MetaAttributeList {
        &self.meta_attributes
    }
}

#[derive(Default)]
pub struct StructDescription {
    fields: Vec<Field>,
}

impl StructDescription {
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &Vec<Field> {
        &self.fields
    }
}

pub struct Variant {
    name: String,
    _ty: Option<Arc<TypeDescription>>,
    discriminant: u128,
}

impl Variant {
    pub fn new(name: String, ty: Option<Arc<TypeDescription>>, discriminant: u128) -> Self {
        Self {
            name,
            _ty: ty,
            discriminant,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn discriminant(&self) -> u128 {
        self.discriminant
    }
}

pub struct EnumDescription {
    variants: Vec<Variant>,
    variant_of_ptr_func: fn(*const u8) -> u128,
    set_variant_of_ptr_func: fn(*mut u8, u128),
}

impl EnumDescription {
    pub fn new(
        variants: Vec<Variant>,
        variant_of_ptr_func: fn(*const u8) -> u128,
        set_variant_of_ptr_func: fn(*mut u8, u128),
    ) -> Self {
        Self {
            variants,
            variant_of_ptr_func,
            set_variant_of_ptr_func,
        }
    }

    pub fn variant_of_ptr(&self, value: *const u8) -> Option<&Variant> {
        let discriminant = (self.variant_of_ptr_func)(value);
        self.variants
            .iter()
            .find(|variant| variant.discriminant == discriminant)
    }

    pub fn set_variant_of_ptr(&self, ptr: *mut u8, value: u128) {
        (self.set_variant_of_ptr_func)(ptr, value)
    }

    pub fn variants(&self) -> &Vec<Variant> {
        &self.variants
    }
}

#[derive(Debug)]
pub enum PrimitiveType {
    Char,
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
    F32,
    F64,
}

pub enum TypeDataDescription {
    Primitive(PrimitiveType),
    Struct(StructDescription),
    Enum(EnumDescription),
}

pub struct TypeDescription {
    name: String,
    _size_in_bytes: usize,
    _alignment_in_bytes: usize,
    data: TypeDataDescription,
}

impl TypeDescription {
    pub fn new(
        name: String,
        size_in_bytes: usize,
        alignment_in_bytes: usize,
        data: TypeDataDescription,
    ) -> Self {
        Self {
            name,
            _size_in_bytes: size_in_bytes,
            _alignment_in_bytes: alignment_in_bytes,
            data,
        }
    }

    pub fn of<T: Reflectable>() -> Arc<TypeDescription> {
        T::type_desc()
    }

    pub fn get_or_create<T: Reflectable, F: FnOnce() -> TypeDescription>(
        f: F,
    ) -> Arc<TypeDescription> {
        let key = std::any::type_name::<T>();
        let types = INTERNAL_TYPE_DATABASE.read();
        if let Some(desc) = types.get(key) {
            desc.clone()
        } else {
            drop(types);
            let desc = Arc::new(f());
            INTERNAL_TYPE_DATABASE.write().insert(key, desc.clone());
            desc
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data(&self) -> &TypeDataDescription {
        &self.data
    }

    pub fn data_as_struct(&self) -> &StructDescription {
        if let TypeDataDescription::Struct(s) = &self.data {
            s
        } else {
            panic!("Not a struct!");
        }
    }
}

/// Base trait for type that has reflection informations
pub trait Reflectable {
    fn type_desc() -> Arc<TypeDescription>;
}

/// Trait for enum reflectable types that are fieldless
pub trait FieldlessEnum: Reflectable + FromPrimitive + Copy + Clone {}

#[macro_export]
macro_rules! ze_reflection_offset_of {
    ($ty:ident, $field:tt) => {{
        let x = core::mem::MaybeUninit::<$ty>::uninit();
        let ptr = x.as_ptr();
        let field_ptr = unsafe { core::ptr::addr_of!((*ptr).$field) };

        (field_ptr as usize) - (ptr as usize)
    }};
}

// Reflectable implementations for primitive types
macro_rules! ze_reflection_impl_primitive {
    ($ty:ident, $primitive_type:ident) => {
        impl Reflectable for $ty {
            fn type_desc() -> Arc<TypeDescription> {
                TypeDescription::get_or_create::<$ty, _>(|| {
                    TypeDescription::new(
                        stringify!($ty).to_string(),
                        std::mem::size_of::<$ty>(),
                        std::mem::align_of::<$ty>(),
                        TypeDataDescription::Primitive(PrimitiveType::$primitive_type),
                    )
                })
            }
        }
    };
}

ze_reflection_impl_primitive!(char, Char);
ze_reflection_impl_primitive!(bool, Bool);

ze_reflection_impl_primitive!(u8, U8);
ze_reflection_impl_primitive!(u16, U16);
ze_reflection_impl_primitive!(u32, U32);
ze_reflection_impl_primitive!(u64, U64);
ze_reflection_impl_primitive!(u128, U128);
ze_reflection_impl_primitive!(usize, USize);

ze_reflection_impl_primitive!(i8, I8);
ze_reflection_impl_primitive!(i16, I16);
ze_reflection_impl_primitive!(i32, I32);
ze_reflection_impl_primitive!(i64, I64);
ze_reflection_impl_primitive!(i128, I128);
ze_reflection_impl_primitive!(isize, ISize);

ze_reflection_impl_primitive!(f32, F32);
ze_reflection_impl_primitive!(f64, F64);
