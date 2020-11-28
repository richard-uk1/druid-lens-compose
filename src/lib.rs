/// TODO add optional fields after the never type is stabilized with `!` fallback.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::{fmt::Write, iter};
use syn::{
    parse::{Error, Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    visit::Visit,
    Fields, GenericParam, Ident, ItemStruct, Result, Type, TypeParam,
};

#[proc_macro_derive(ComposeLens)]
pub fn compose_lens(item: TokenStream) -> TokenStream {
    let compose_lens = parse_macro_input!(item as ComposeLens);
    let inner_generics = &compose_lens.generics;
    let inner_generics_constraints: Vec<_> = inner_generics
        .iter()
        .map(|ty| quote!(#ty: Clone + ::druid::Data))
        .collect();
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
        .map(|ty| quote!(::druid::Lens<Outer, #ty>))
        .collect();
    let fn_params: Vec<_> = field_names
        .iter()
        .zip(lenses.iter())
        .map(|(name, lens)| quote!(#name: impl #lens))
        .collect();
    let generics: Vec<_> = (0..field_names.len())
        .into_iter()
        .map(|n| Ident::new(&format!("_L{}", n), Span::call_site()))
        .collect();
    let field_names_tys: Vec<_> = generics
        .iter()
        .zip(field_names.iter())
        .map(|(ty, name)| quote!(#name: #ty))
        .collect();
    let wheres = inner_generics_constraints.iter().cloned().chain(
        generics
            .iter()
            .zip(lenses.iter())
            .map(|(generic, lens)| quote!(#generic: #lens)),
    );
    let all_generics: Vec<_> = iter::once(Ident::new("Outer", Span::call_site()))
        .chain(inner_generics.iter().cloned())
        .chain(generics.iter().cloned())
        .collect();
    let lets: Vec<_> = field_names
        .iter()
        .map(|name| quote!(let #name = self.#name.with(data, |v| v.clone());))
        .collect();
    let assigns = field_names.iter().map(|name| {
        quote!(
            self.#name.with_mut(data, |v| {
                if !::druid::Data::same(&#name, v) {
                    *v = #name;
                }
            });
        )
    });
    TokenStream::from(quote! {
        impl <#(#inner_generics),*> #name<#(#inner_generics),*>
        where
            #(#inner_generics_constraints),*
        {

            pub fn compose_lens<Outer>(
                #(#fn_params),*
            ) -> impl ::druid::Lens<Outer, #name <#(#inner_generics),*>> {
                struct LensCompose<#(#generics),*> {
                    #(#field_names_tys),*
                }

                impl<#(#all_generics),*> ::druid::Lens<Outer, #name <#(#inner_generics),*>> for LensCompose<#(#generics),*>
                where
                    #(#wheres),*
                {
                    fn with<V, F: FnOnce(&#name <#(#inner_generics),*>) -> V>(&self, data: &Outer, f: F) -> V {
                        #(#lets)*
                        let _widget_data = #name { #(#field_names),* };
                        f(&_widget_data)
                    }
                    fn with_mut<V, F: FnOnce(&mut #name <#(#inner_generics),*>) -> V>(&self, data: &mut Outer, f: F) -> V {
                        #(#lets)*
                        let mut _widget_data = #name { #(#field_names),* };
                        let output = f(&mut _widget_data);
                        let #name { #(#field_names),* } = _widget_data;
                        #(#assigns)*
                        output
                    }
                }

                LensCompose { #(#field_names),* }
            }
        }
    })
}

struct ComposeLens {
    name: Ident,
    generics: Vec<Ident>,
    fields: Vec<(Ident, Type)>,
}

impl Parse for ComposeLens {
    fn parse(input: ParseStream) -> Result<Self> {
        let raw: ItemStruct = input.parse()?;
        // check the user hasn't used any reserved names for their generics.
        let mut check_generics = CheckGenerics::new();
        check_generics.visit_item_struct(&raw);
        if !check_generics.bad_generics.is_empty() {
            let mut msg = format!("found type names that might clash: ");
            let mut it = check_generics.bad_generics.into_iter();
            let first = it.next().unwrap();
            write!(msg, "\"{}\"", first).unwrap();
            for generic in it {
                write!(msg, ", \"{}\"", generic).unwrap();
            }
            return Err(Error::new(first.span(), &msg));
        }
        // check params
        if let Some(clause) = raw.generics.where_clause {
            return Err(Error::new(
                clause.span(),
                "no constraints allowed on the struct for now",
            ));
        }
        let generics = raw
            .generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Type(param) => {
                    if !param.attrs.is_empty() {
                        Err(Error::new(
                            param.attrs.first().unwrap().span(),
                            "attributes not supported",
                        ))
                    } else if param.colon_token.is_some() {
                        Err(Error::new(
                            param.bounds.span(),
                            "constraints on generic parameters not supported",
                        ))
                    } else {
                        Ok(param.ident.clone())
                    }
                }
                other => Err(Error::new(
                    other.span(),
                    "lifetime and const generic parameters not supported",
                )),
            })
            .collect::<Result<Vec<_>>>()?;
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
        let name = raw.ident;
        Ok(ComposeLens {
            name,
            generics,
            fields,
        })
    }
}

// Visitor to check that our generic names don't clash with the user's

struct CheckGenerics {
    bad_generics: Vec<Ident>,
}

impl CheckGenerics {
    fn new() -> Self {
        CheckGenerics {
            bad_generics: vec![],
        }
    }
}

impl<'ast> Visit<'ast> for CheckGenerics {
    fn visit_type_param(&mut self, i: &'ast TypeParam) {
        if bad_ident(&i.ident.to_string()) {
            self.bad_generics.push(i.ident.clone());
        }
    }
}

fn bad_ident(i: &str) -> bool {
    if i == "Outer" {
        true
    } else if i.starts_with("_L") {
        let num = &i[1..];
        num.parse::<u64>().is_ok()
    } else {
        false
    }
}
