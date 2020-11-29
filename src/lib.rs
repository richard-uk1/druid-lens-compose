use itertools::izip;
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
    let builder_name = Ident::new(
        &format!("{}LensBuilder", compose_lens.name),
        compose_lens.name.span(),
    );
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
    let generics: Vec<_> = (0..field_names.len())
        .into_iter()
        .map(|n| Ident::new(&format!("L{}", n), Span::call_site()))
        .collect();
    let builder_fields: Vec<_> = izip!(field_names.iter(), generics.iter())
        .map(|(name, ty)| quote!(#name: Option<#ty>))
        .collect();
    let builder_fn_doc = format!(
        "Create a builder object to build a lens for `{}` out of lenses to its fields.",
        name
    );
    let builder_doc = format!(
        "An object for making `Lens`es for `{}` following the builder pattern.

Once all lenses are set, call `build` to create the lens",
        name
    );
    let builder_field_defaults: Vec<_> =
        field_names.iter().map(|name| quote!(#name: None)).collect();
    let builder_field_unwraps: Vec<_> = field_names
        .iter()
        .map(|name| quote!(#name: self.#name.unwrap()))
        .collect();
    let builder_fns: Vec<_> = izip!(field_names.iter(), generics.iter())
        .map(|(name, ty)| {
            let docs = format!("Set the lens for the `{}` field", name);
            quote! {
                #[doc = #docs]
                #[inline]
                pub fn #name(mut self, #name: #ty) -> Self {
                    self.#name = Some(#name);
                    self
                }
            }
        })
        .collect();
    let field_names_tys: Vec<_> = generics
        .iter()
        .zip(field_names.iter())
        .map(|(ty, name)| quote!(#name: #ty))
        .collect();
    let wheres: Vec<_> = inner_generics_constraints
        .iter()
        .cloned()
        .chain(
            generics
                .iter()
                .zip(lenses.iter())
                .map(|(generic, lens)| quote!(#generic: #lens)),
        )
        .collect();
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
    let lens_name = Ident::new(&format!("{}Lens", name), Span::call_site());

    TokenStream::from(quote! {
        impl <#(#inner_generics),*> #name<#(#inner_generics),*> {
            #[doc = #builder_fn_doc]
            #[inline]
            pub fn lens_builder<#(#generics),*>() -> #builder_name<#(#generics),*> {
                #builder_name::new()
            }
        }

        #[doc = #builder_doc]
        #[derive(Copy, Clone)]
        pub struct #builder_name<#(#generics),*> {
            #(#builder_fields),*
        }

        impl<#(#generics),*> #builder_name<#(#generics),*> {
            #[inline]
            pub fn new() -> Self {
                Self { #(#builder_field_defaults),* }
            }

            #(#builder_fns)*

            /// Builds the lens
            ///
            /// # Panics
            ///
            /// Panics if any of the field lenses are not set yet.
            #[inline]
            pub fn build<Outer, #(#inner_generics),*>(self) -> #lens_name<#(#generics),*>
            where
                #(#wheres),*
            {
                #lens_name {
                    #(#builder_field_unwraps),*
                }
            }
        }

        impl<#(#generics),*> Default for #builder_name<#(#generics),*> {
            #[inline]
            fn default() -> Self {
                Self::new()
            }
        }

        #[derive(Copy, Clone)]
        pub struct #lens_name<#(#generics),*> {
            #(#field_names_tys),*
        }

        impl<#(#all_generics),*> ::druid::Lens<Outer, #name <#(#inner_generics),*>>
        for #lens_name<#(#generics),*>
        where
            #(#wheres),*
        {
            #[inline]
            fn with<V, F: FnOnce(&#name <#(#inner_generics),*>) -> V>(
                &self,
                data: &Outer,
                f: F
            ) -> V {
                #(#lets)*
                let _widget_data = #name { #(#field_names),* };
                f(&_widget_data)
            }
            #[inline]
            fn with_mut<V, F: FnOnce(&mut #name <#(#inner_generics),*>) -> V>(
                &self,
                data: &mut Outer,
                f: F
            ) -> V {
                #(#lets)*
                let mut _widget_data = #name { #(#field_names),* };
                let output = f(&mut _widget_data);
                let #name { #(#field_names),* } = _widget_data;
                #(#assigns)*
                output
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
    } else if i.starts_with("L") {
        let num = &i[1..];
        num.parse::<u64>().is_ok()
    } else {
        false
    }
}
