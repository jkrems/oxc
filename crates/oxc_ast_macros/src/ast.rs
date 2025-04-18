use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Fields, Ident, Item, ItemEnum, punctuated::Punctuated, token::Comma};

use crate::generated::get_trait_crate_and_generics;

pub fn ast(input: &Item) -> TokenStream {
    let (head, tail) = match input {
        Item::Enum(enum_) => (enum_repr(enum_), assert_generated_derives(&enum_.attrs)),
        Item::Struct(struct_) => (quote!(#[repr(C)]), assert_generated_derives(&struct_.attrs)),
        _ => unreachable!(),
    };

    quote! {
        #[derive(::oxc_ast_macros::Ast)]
        #head
        #input
        #tail
    }
}

/// If `enum_` has any non-unit variant, returns `#[repr(C, u8)]`, otherwise returns `#[repr(u8)]`.
fn enum_repr(enum_: &ItemEnum) -> TokenStream {
    if enum_.variants.iter().any(|var| !matches!(var.fields, Fields::Unit)) {
        quote!(#[repr(C, u8)])
    } else {
        quote!(#[repr(u8)])
    }
}

/// Generate assertions that traits used in `#[generate_derive]` are in scope.
///
/// e.g. for `#[generate_derive(GetSpan)]`, it generates:
///
/// ```rs
/// const _: () = {
///     {
///         trait AssertionTrait: ::oxc_span::GetSpan {}
///         impl<T: GetSpan> AssertionTrait for T {}
///     }
/// };
/// ```
///
/// If `GetSpan` is not in scope, or it is not the correct `oxc_span::GetSpan`,
/// this will raise a compilation error.
fn assert_generated_derives(attrs: &[Attribute]) -> TokenStream {
    // NOTE: At this level we don't care if a trait is derived multiple times, It is the
    // responsibility of the `ast_tools` to raise errors for those.
    let assertions = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("generate_derive"))
        .flat_map(parse_attr)
        .map(|trait_ident| {
            let trait_name = trait_ident.to_string();
            let (trait_path, generics) = get_trait_crate_and_generics(&trait_name);
            quote! {{
                // NOTE: these are wrapped in a scope to avoid the need for unique identifiers.
                trait AssertionTrait: #trait_path #generics {}
                impl<T: #trait_ident #generics> AssertionTrait for T {}
            }}
        });
    quote!( const _: () = { #(#assertions)* }; )
}

#[inline]
fn parse_attr(attr: &Attribute) -> impl Iterator<Item = Ident> + use<> {
    attr.parse_args_with(Punctuated::<Ident, Comma>::parse_terminated)
        .expect("`#[generate_derive]` only accepts traits as single segment paths. Found an invalid argument.")
        .into_iter()
}
