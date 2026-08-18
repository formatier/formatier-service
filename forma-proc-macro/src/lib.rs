use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Ident, LitStr, Token, Visibility, parse::Parse, parse_macro_input};

#[derive(Clone)]
struct MigratorChunk {
    version_tag: LitStr,
    _colon_token: Token![:],
    name: Ident,
    _semi_token: Token![;],
}

impl Parse for MigratorChunk {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(MigratorChunk {
            version_tag: input.parse()?,
            _colon_token: input.parse()?,
            name: input.parse()?,
            _semi_token: input.parse()?,
        })
    }
}

struct MigratorInput {
    attr: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    items: Vec<MigratorChunk>,
}

impl Parse for MigratorInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attr = input.call(Attribute::parse_outer)?;
        let vis: Visibility = if input.peek(Token![pub]) {
            input.parse()?
        } else {
            Visibility::Inherited
        };

        let name = input.parse()?;

        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }

        Ok(MigratorInput {
            attr,
            vis,
            name,
            items,
        })
    }
}

#[proc_macro]
pub fn serde_migrator(token: TokenStream) -> TokenStream {
    let input = parse_macro_input!(token as MigratorInput);

    if input.items.is_empty() {
        return TokenStream::new();
    }

    let vis = &input.vis;
    let name = &input.name;
    let attr = &input.attr;

    let len = input.items.len();

    let last_chunk = &input.items[len - 1];
    let last_enum_name = &last_chunk.name;

    let mut from_impls = Vec::new();

    let mut enum_variants = Vec::new();
    let mut match_arms = Vec::new();

    for idx in 0..len {
        let chunk = &input.items[idx];
        let enum_name = &chunk.name;
        let version_tag = &chunk.version_tag;

        let init_var = format_ident!("step_{}", idx);
        let mut current_var = init_var.clone();

        enum_variants.push(quote! {
            #[serde(rename=#version_tag)]
            #enum_name(#enum_name),
        });

        from_impls.push(quote! {
            impl From<#enum_name> for #name {
                fn from(value: #enum_name) -> Self {
                    Self::#enum_name(value)
                }
            }
        });

        if idx == len - 1 {
            match_arms.push(quote! {
                Self::#enum_name(#init_var) => (#init_var, false),
            });
        } else {
            let mut magic_from = Vec::new();

            for magic_idx in idx + 1..len {
                let next_chunk = &input.items[magic_idx];
                let next_enum_name = &next_chunk.name;
                let next_var = format_ident!("step_{}", magic_idx);

                magic_from.push(quote! {
                    let #next_var: #next_enum_name = #current_var.into();
                });

                current_var = next_var;
            }

            match_arms.push(quote! {
                Self::#enum_name(#init_var) => {
                    #(#magic_from)*
                    (#current_var, true)
                },
            });
        }
    }

    let lastest_type = format_ident!("Latest{}", name);

    let expanded = quote! {
        #(#attr)*
        #[serde(tag = "type")]
        #vis enum #name {
            #(#enum_variants)*
        }

        impl #name {
            pub fn migrate(self) -> (#last_enum_name, bool) {
                match self {
                    #(#match_arms)*
                }
            }
        }

        #(#from_impls)*

        #vis type #lastest_type = #last_enum_name;
    };

    TokenStream::from(expanded)
}
