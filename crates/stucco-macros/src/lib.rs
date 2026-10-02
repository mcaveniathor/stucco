//! Derive macros for stucco. Use them through the `stucco` crate's `derive`
//! feature, which re-exports them next to the traits they implement.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields, LitStr, Path, Token, Type, parse_macro_input};

/// Implements `stucco::collections::Columns` for a struct with named fields:
/// one data table column per field, in order. See that trait for the
/// `#[col(...)]` options.
#[proc_macro_derive(Columns, attributes(col, columns))]
pub fn derive_columns(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    columns(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// How a column reads and filters its value.
enum Kind {
    Text,
    Number,
    Date,
    Enumeration(Vec<LitStr>),
}

/// One field's `#[col(...)]` options.
#[derive(Default)]
struct Options {
    kind: Option<Kind>,
    key: Option<LitStr>,
    label: Option<LitStr>,
    value: Option<Path>,
    display: Option<Path>,
    link: Option<Path>,
    sortable: bool,
    searchable: bool,
    filter: bool,
    skip: bool,
}

fn columns(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(Error::new(
            input.generics.span(),
            "#[derive(Columns)] does not support generic structs yet; implement `Columns` by hand",
        ));
    }
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new(
                    name.span(),
                    "#[derive(Columns)] needs a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new(
                name.span(),
                "#[derive(Columns)] needs a struct with named fields",
            ));
        }
    };
    let krate = crate_path(input)?;
    let mut columns = Vec::new();
    for field in fields {
        let options = field_options(field)?;
        if options.skip {
            continue;
        }
        let ident = field.ident.as_ref().expect("named fields have names");
        let field_name = ident.to_string();
        let field_name = field_name.trim_start_matches("r#");
        let key = options
            .key
            .clone()
            .unwrap_or_else(|| LitStr::new(field_name, ident.span()));
        let label = options
            .label
            .clone()
            .unwrap_or_else(|| LitStr::new(&label_for(field_name), ident.span()));
        let kind = match options.kind {
            Some(kind) => kind,
            None => infer_kind(&field.ty)?,
        };
        // The value that sorts and filters: the field, or `value = path`.
        let text = match &options.value {
            Some(value) => quote! { #value },
            None => quote! { |row: &Self| ::std::string::ToString::to_string(&row.#ident) },
        };
        let number = match &options.value {
            Some(value) => quote! { #value },
            None => quote! { |row: &Self| row.#ident as f64 },
        };
        let mut column = match kind {
            Kind::Text => quote! { #krate::collections::Col::text(#key, #label, #text) },
            Kind::Number => quote! { #krate::collections::Col::number(#key, #label, #number) },
            Kind::Date => quote! { #krate::collections::Col::date(#key, #label, #text) },
            Kind::Enumeration(values) => quote! {
                #krate::collections::Col::enumeration(
                    #key,
                    #label,
                    ::std::vec![#((
                        ::std::string::String::from(#values),
                        ::std::string::String::from(#values),
                    )),*],
                    #text,
                )
            },
        };
        if let Some(display) = &options.display {
            column = quote! { #column.display(#display) };
        }
        if let Some(link) = &options.link {
            column = quote! { #column.href(#link) };
        }
        if options.sortable {
            column = quote! { #column.sortable() };
        }
        if options.searchable {
            column = quote! { #column.searchable() };
        }
        if options.filter {
            column = quote! { #column.filter() };
        }
        columns.push(column);
    }
    Ok(quote! {
        impl #krate::collections::Columns for #name {
            fn columns<'a>() -> ::std::vec::Vec<#krate::collections::Col<'a, Self>>
            where
                Self: 'a,
            {
                ::std::vec![#(#columns),*]
            }
        }
    })
}

/// The path to the `stucco` crate: `::stucco` unless the struct says
/// otherwise with `#[columns(crate = path)]`.
fn crate_path(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let mut krate = quote! { ::stucco };
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("columns")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                let path: Path = meta.value()?.parse::<LitStr>()?.parse()?;
                krate = quote! { #path };
                Ok(())
            } else {
                Err(meta.error("expected `crate = \"path\"`"))
            }
        })?;
    }
    Ok(krate)
}

fn field_options(field: &syn::Field) -> syn::Result<Options> {
    let mut options = Options::default();
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("col")) {
        attr.parse_nested_meta(|meta| {
            let set_kind = |options: &mut Options, kind: Kind| {
                if options.kind.is_some() {
                    return Err(meta.error("a column has one kind"));
                }
                options.kind = Some(kind);
                Ok(())
            };
            let path = &meta.path;
            if path.is_ident("text") {
                set_kind(&mut options, Kind::Text)
            } else if path.is_ident("number") {
                set_kind(&mut options, Kind::Number)
            } else if path.is_ident("date") {
                set_kind(&mut options, Kind::Date)
            } else if path.is_ident("enumeration") {
                let content;
                syn::parenthesized!(content in meta.input);
                let values = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?;
                if values.is_empty() {
                    return Err(meta.error("list the allowed values: enumeration(\"a\", \"b\")"));
                }
                set_kind(
                    &mut options,
                    Kind::Enumeration(values.into_iter().collect()),
                )
            } else if path.is_ident("key") {
                options.key = Some(meta.value()?.parse()?);
                Ok(())
            } else if path.is_ident("label") {
                options.label = Some(meta.value()?.parse()?);
                Ok(())
            } else if path.is_ident("value") {
                options.value = Some(meta.value()?.parse()?);
                Ok(())
            } else if path.is_ident("display") {
                options.display = Some(meta.value()?.parse()?);
                Ok(())
            } else if path.is_ident("link") {
                options.link = Some(meta.value()?.parse()?);
                Ok(())
            } else if path.is_ident("sortable") {
                options.sortable = true;
                Ok(())
            } else if path.is_ident("searchable") {
                options.searchable = true;
                Ok(())
            } else if path.is_ident("filter") {
                options.filter = true;
                Ok(())
            } else if path.is_ident("skip") {
                options.skip = true;
                Ok(())
            } else {
                Err(meta.error(
                    "unknown column option; expected text, number, date, enumeration(...), \
                     key, label, value, display, link, sortable, searchable, filter or skip",
                ))
            }
        })?;
    }
    Ok(options)
}

/// The kind a field's type implies: numbers for numeric primitives, text
/// for strings.
fn infer_kind(ty: &Type) -> syn::Result<Kind> {
    let ident = match ty {
        Type::Path(path) if path.qself.is_none() => path.path.segments.last().map(|s| &s.ident),
        Type::Reference(reference) => match &*reference.elem {
            Type::Path(path) => path.path.segments.last().map(|s| &s.ident),
            _ => None,
        },
        _ => None,
    };
    let name = ident.map(ToString::to_string).unwrap_or_default();
    match name.as_str() {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "isize" | "f32" | "f64" => Ok(Kind::Number),
        "String" | "str" => Ok(Kind::Text),
        _ => Err(Error::new(
            ty.span(),
            "can't tell this field's column kind from its type; add #[col(text)], \
             #[col(number)], #[col(date)], #[col(enumeration(\"a\", \"b\"))] or #[col(skip)]",
        )),
    }
}

/// `total_cents` → `Total cents`.
fn label_for(field: &str) -> String {
    let words = field.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(input: TokenStream2) -> String {
        let input: DeriveInput = syn::parse2(input).unwrap();
        columns(&input)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
    }

    #[test]
    fn unclear_fields_ask_for_a_kind() {
        let e = error(quote! { struct Row { when: Timestamp } });
        assert!(e.contains("can't tell this field's column kind"), "{e}");
        let e = error(quote! { struct Row { #[col(text, number)] name: String } });
        assert!(e.contains("one kind"), "{e}");
        let e = error(quote! { struct Row { #[col(sortabel)] name: String } });
        assert!(e.contains("unknown column option"), "{e}");
        let e = error(quote! { struct Row(String); });
        assert!(e.contains("named fields"), "{e}");
    }

    #[test]
    fn labels_come_from_field_names() {
        assert_eq!(label_for("total_cents"), "Total cents");
        assert_eq!(label_for("id"), "Id");
    }

    #[test]
    fn the_crate_path_can_be_changed() {
        let input: DeriveInput =
            syn::parse2(quote! { #[columns(crate = "my::stucco")] struct Row { name: String } })
                .unwrap();
        let out = columns(&input).unwrap().to_string();
        assert!(
            out.contains("my :: stucco :: collections :: Columns"),
            "{out}"
        );
    }
}
