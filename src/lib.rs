use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::{Error, Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Fields, Ident, ItemStruct, Result, Type,
};

#[proc_macro_derive(ComposeLens)]
pub fn compose_lens(item: TokenStream) -> TokenStream {
    let compose_lens = parse_macro_input!(item as ComposeLens);
    let name = &compose_lens.name;
    let field_names: Vec<_> = compose_lens
        .fields
        .iter()
        .map(|field| field.0.clone())
        .collect();
    let field_tys: Vec<_> = compose_lens
        .fields
        .iter()
        .map(|field| field.1.clone())
        .collect();
    let lenses: Vec<_> = field_tys
        .iter()
        .map(|ty| quote!(::druid::Lens<T, #ty>))
        .collect();
    let fn_params: Vec<_> = field_names
        .iter()
        .zip(lenses.iter())
        .map(|(name, lens)| quote!(#name: impl #lens))
        .collect();
    let generics: Vec<_> = (0..field_names.len())
        .into_iter()
        .map(|n| Ident::new(&format!("L{}", n), Span::call_site()))
        .collect();
    let field_names_tys: Vec<_> = generics
        .iter()
        .zip(field_names.iter())
        .map(|(ty, name)| quote!(#name: #ty))
        .collect();
    let wheres = generics
        .iter()
        .zip(lenses.iter())
        .map(|(generic, lens)| quote!(#generic: #lens));
    let lets = field_names
        .iter()
        .map(|name| quote!(let #name = self.#name.with(data, |v| v.clone());));
    TokenStream::from(quote! {
        impl #name {
            pub fn compose_lens<T>(
                #(#fn_params),*
            ) -> impl ::druid::Lens<T, #name> {
                struct LensCompose<#(#generics),*> {
                    #(#field_names_tys),*
                }

                impl<T, #(#generics),*> Lens<T, #name> for LensCompose<#(#generics),*>
                where
                    #(#wheres),*
                {
                    fn with<V, F: FnOnce(&#name) -> V>(&self, data: &T, f: F) -> V {
                        #(#lets)*
                        let _widget_data = #name { #(#field_names),* };
                        f(&_widget_data)
                    }
                    fn with_mut<V, F: FnOnce(&mut #name) -> V>(&self, data: &mut T, f: F) -> V {
                        todo!()
                    }
                }

                LensCompose { #(#field_names),* }
            }
        }
    })
}

struct ComposeLens {
    name: Ident,
    fields: Vec<(Ident, Type)>,
}

impl Parse for ComposeLens {
    fn parse(input: ParseStream) -> Result<Self> {
        let raw: ItemStruct = input.parse()?;
        let name = raw.ident;
        if raw.generics.lt_token.is_some() {
            return Err(Error::new(
                raw.generics.span(),
                "generics not supported on ComposeLens",
            ));
        }
        let fields = match raw.fields {
            Fields::Named(f) => f,
            other => {
                return Err(Error::new(
                    other.span(),
                    "only named fields are supported for now",
                ))
            }
        };
        let fields = fields
            .named
            .iter()
            .map(|field| (field.ident.as_ref().cloned().unwrap(), field.ty.clone()))
            .collect::<Vec<_>>();
        Ok(ComposeLens { name, fields })
    }
}
