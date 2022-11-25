use proc_macro::*;
use quote::quote;
use std::str::FromStr;
use syn::*;
use uuid::Uuid;

#[proc_macro_derive(TypeUuid, attributes(type_uuid))]
pub fn type_uuid_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let type_name = &ast.ident;

    for attribute in ast.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
        if let Meta::NameValue(name) = attribute {
            if name
                .path
                .get_ident()
                .map(|ident| ident == "type_uuid")
                .unwrap_or(false)
            {
                let str = match name.lit {
                    Lit::Str(str) => str.value(),
                    _ => panic!("type_uuid must be a valid string: #[type_uuid = \"...\""),
                };

                // Verify UUID is correctly formatted
                let uuid = Uuid::from_str(&str)
                    .expect("Invalid UUID provided")
                    .as_u128();

                let bytes = uuid.to_le_bytes();
                let bytes = bytes
                    .iter()
                    .map(|byte| format!("{:#X}", byte))
                    .map(|byte| parse_str::<LitInt>(&byte).unwrap());

                let generated_trait = quote! {
                    impl ze_core::type_uuid::TypeUuid for #type_name {
                        fn type_uuid() -> uuid::Uuid {
                            const UUID : uuid::Uuid = Uuid::from_bytes_le([#(#bytes),*]);
                            UUID
                        }
                    }
                };

                return generated_trait.into();
            }
        }
    }

    panic!("Missing #[type_uuid] attribute")
}
