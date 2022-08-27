use proc_macro::*;
use quote::{quote};
use syn::*;
use syn::__private::TokenStream2;

#[proc_macro_derive(Reflectable, attributes(ze_reflect))]
pub fn reflect_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let type_name = &ast.ident;

    match &ast.data {
        Data::Struct(data) => {
            let fields = data
                .fields
                .iter()
                .filter(|field| {
                    // Check if this field is reflectable
                    for attribute in field.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
                        if attribute.path().is_ident("ze_reflect") {
                            return true;
                        }
                    }

                    false
                })
                .map(|field| {
                    let name_ident = field.ident.as_ref().unwrap();
                    let name = field.ident.as_ref().unwrap().to_string();
                    let ty = &field.ty;
                    let mut attributes : Vec<TokenStream2> = vec![];
                    for root_attr in &field.attrs {
                        let meta = root_attr.parse_meta().unwrap();
                        if meta.path().is_ident("ze_reflect") {
                            fn parse_attribute(attributes: &mut Vec<TokenStream2>, attribute: &Meta, is_ze_reflect: bool) {
                                match attribute {
                                    Meta::Path(path) => {
                                        if !is_ze_reflect {
                                            let name = path.get_ident().unwrap().to_string();
                                            attributes.push(quote! { MetaAttribute::new((#name).to_string(), None) });
                                        }
                                    }
                                    Meta::List(list) => {
                                        let name = list.path.get_ident().unwrap().to_string();
                                        let mut nested_attributes = vec![];
                                        for attr in &list.nested {
                                            if let NestedMeta::Meta(meta) = attr {
                                                parse_attribute(&mut nested_attributes, meta, false);
                                            }
                                        }

                                        if is_ze_reflect {
                                            attributes.push(quote! { #(#nested_attributes),* });
                                        } else {
                                            attributes.push(quote! { MetaAttribute::new((#name).to_string(), Some(
                                                MetaAttributeValue::List(MetaAttributeList::new(vec![#(#nested_attributes),*])))) });
                                        }
                                    }
                                    Meta::NameValue(nv) => {
                                        if !is_ze_reflect {
                                            let name = nv.path.get_ident().unwrap().to_string();
                                            let value = &nv.lit;
                                            attributes.push(quote! { MetaAttribute::new((#name).to_string(), 
                                              Some(MetaAttributeValue::Value((#value).to_string()))) });
                                        }
                                    }
                                }
                            }
                            
                            parse_attribute(&mut attributes, &root_attr.parse_meta().unwrap(), true);
                            break;
                        }
                    }
                    
                    quote! {
                        Field::new((#name).to_string(), ze_reflection::ze_reflection_offset_of!(#type_name, #name_ident), TypeDescription::of::<#ty>(),
                            MetaAttributeList::new(vec![#(#attributes),*]))
                    }
                });

            let generated_trait = quote! {
                impl ze_reflection::Reflectable for #type_name {
                    fn type_desc() -> Arc<ze_reflection::TypeDescription> {
                        ze_reflection::TypeDescription::get_or_create::<#type_name, _>(||
                            ze_reflection::TypeDescription::new(
                                std::any::type_name::<#type_name>().to_string(),
                                std::mem::size_of::<#type_name>(),
                                std::mem::align_of::<#type_name>(),
                                TypeDataDescription::Struct(ze_reflection::StructDescription::new(vec![#(#fields),*]))))
                    }
                }
            };

            generated_trait.into()
        }
        Data::Enum(data) => {
            let fieldless = !data.variants.iter().any(|variant| {
                !variant.fields.is_empty()
            });
            
            let variants = data.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let name = variant.ident.to_string();
                quote! {
                    Variant::new((#name).to_string(), None, #type_name::#ident as u128)
                }
            });
            
            let fieldless = if fieldless { 
                quote! { impl ze_reflection::FieldlessEnum for #type_name {} } }
                else {
                    quote! {}
                };

            let generated_trait = quote! {
                impl ze_reflection::Reflectable for #type_name {
                    fn type_desc() -> Arc<ze_reflection::TypeDescription> {
                        ze_reflection::TypeDescription::get_or_create::<#type_name, _>(||
                            ze_reflection::TypeDescription::new(
                                std::any::type_name::<#type_name>().to_string(),
                                std::mem::size_of::<#type_name>(),
                                std::mem::align_of::<#type_name>(),
                                TypeDataDescription::Enum(ze_reflection::EnumDescription::new(vec![#(#variants),*], 
                                |value| {
                                        let value = unsafe { (value as *const #type_name).as_ref().unwrap_unchecked() };
                                        *value as u128
                                    },
                                |ptr, value| {
                                        let ptr = unsafe { (ptr as *mut #type_name).as_mut().unwrap_unchecked() };
                                        *ptr = FromPrimitive::from_u128(value).unwrap()
                                    }))))
                    }
                }
                
                #fieldless
            };

            generated_trait.into()
        }
        Data::Union(_) => panic!("ze-reflection doesn't support unions"),
    }
}
