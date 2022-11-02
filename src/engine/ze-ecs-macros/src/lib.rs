extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Data, DeriveInput, Ident, LitInt, Token};

#[proc_macro_derive(Component)]
pub fn component_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let type_name = &ast.ident;

    let out = quote! {
        impl Component for #type_name {
            fn component_id() -> ComponentId {
                static ID: usize = ze_ecs::component::COMPONENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
                ID
            }
        }
    };

    out.into()
}

/// Repeat a macro targeting a tuple recursively
/// Inspired from bevy_ecs
#[proc_macro]
pub fn repeat_tuples(input: TokenStream) -> TokenStream {
    struct RecursiveTuples {
        m: Ident,
        num: usize,
        idents: Vec<Ident>,
    }

    impl Parse for RecursiveTuples {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let m = input.parse::<Ident>()?;
            input.parse::<Token!(,)>()?;
            let num = input.parse::<LitInt>()?.base10_parse()?;
            input.parse::<Token!(,)>()?;
            let mut idents = vec![input.parse::<Ident>()?];
            while input.parse::<Token!(,)>().is_ok() {
                idents.push(input.parse::<Ident>()?);
            }
            Ok(RecursiveTuples { m, num, idents })
        }
    }

    let input = parse_macro_input!(input as RecursiveTuples);
    let mut tuples = Vec::with_capacity(input.num);
    for i in 0..input.num {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));

        tuples.push(quote! {
            (#(#idents),*)
        });
    }

    let m = &input.m;
    let invocations = (0..input.num).map(|i| {
        let ident_tuples = &tuples[0..i];
        if ident_tuples.len() > 1 {
            quote! {
                #m!(#(#ident_tuples),*);
            }
        } else {
            quote! {}
        }
    });

    let out = quote! {
        #(
            #invocations
        )*
    };
    out.into()
}

#[proc_macro]
pub fn repeat_tuples_no_skip(input: TokenStream) -> TokenStream {
    struct RecursiveTuples {
        m: Ident,
        num: usize,
        idents: Vec<Ident>,
    }

    impl Parse for RecursiveTuples {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let m = input.parse::<Ident>()?;
            input.parse::<Token!(,)>()?;
            let num = input.parse::<LitInt>()?.base10_parse()?;
            input.parse::<Token!(,)>()?;
            let mut idents = vec![input.parse::<Ident>()?];
            while input.parse::<Token!(,)>().is_ok() {
                idents.push(input.parse::<Ident>()?);
            }
            Ok(RecursiveTuples { m, num, idents })
        }
    }

    let input = parse_macro_input!(input as RecursiveTuples);
    let mut tuples = Vec::with_capacity(input.num);
    for i in 0..=input.num {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));

        if input.idents.len() < 2 {
            tuples.push(quote! {
                #(#idents)*
            });
        } else {
            tuples.push(quote! {
                (#(#idents),*)
            });
        }
    }

    let m = &input.m;
    let invocations = (0..=input.num).map(|i| {
        let ident_tuples = &tuples[..i];
        quote! {
            #m!(#(#ident_tuples),*);
        }
    });

    let out = quote! {
        #(
            #invocations
        )*
    };
    out.into()
}

#[proc_macro_derive(SystemId)]
pub fn derive_system_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let inner_impl = match input.data {
        Data::Struct(_) => unimplemented!("Structs are not supported"),
        Data::Enum(e) => {
            let variants = e.variants.iter().map(|variant| {
                let id = format!("{}::{}", name, variant.ident);
                quote! {
                    #name::#variant => crate::system::SystemId(#id),
                }
            });
            quote! {
                match self {
                    #(#variants)*
                }
            }
        }
        Data::Union(_) => unimplemented!("Unions are not supported"),
    };

    let out = quote! {
        impl ze_ecs::system::IntoSystemId for #name {
            fn system_id(&self) -> SystemId {
                #inner_impl
            }
        }
    };
    out.into()
}
