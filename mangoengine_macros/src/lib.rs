use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, DeriveInput, Ident, LitStr, Token,
};

/// Attribute macro that wires a struct up as a MongoDB-backed model.
///
/// Placed above a struct definition, it:
/// 1. Re-emits the struct unchanged.
/// 2. Generates an `impl mangoengine::DBCollectionRowTrait<Struct> for Struct`
///    block containing `collection_name()`, `collection_cell()` (with a
///    per-struct `static OnceCell`), and `get_id()` (delegating to `self._id`).
///
/// The struct must have an `_id: ObjectId` field.
///
/// # Example
/// ```ignore
/// use mangoengine::db_collection;
///
/// #[db_collection("shard_lock")]
/// #[derive(Serialize, Deserialize, Debug, Clone)]
/// pub struct Lock {
///     pub _id: ObjectId,
///     pub name: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn db_collection(attr: TokenStream, item: TokenStream) -> TokenStream {
    let collection_name = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let expanded = quote! {
        #input

        impl ::mangoengine::DBCollectionRowTrait<#struct_name> for #struct_name {
            fn collection_name() -> &'static str {
                #collection_name
            }

            fn collection_cell() -> &'static ::tokio::sync::OnceCell<
                ::mongodb::Collection<::mongodb::bson::Document>,
            > {
                static CELL: ::tokio::sync::OnceCell<
                    ::mongodb::Collection<::mongodb::bson::Document>,
                > = ::tokio::sync::OnceCell::const_new();
                &CELL
            }

            fn get_id(&self) -> ::mongodb::bson::oid::ObjectId {
                self._id.clone()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Arguments for `#[db_collection_from_raw("name", RawType)]`.
struct FromRawArgs {
    collection_name: LitStr,
    raw_type: Ident,
}

impl Parse for FromRawArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let collection_name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let raw_type: Ident = input.parse()?;
        Ok(FromRawArgs {
            collection_name,
            raw_type,
        })
    }
}

/// Attribute macro that wires a struct up as a MongoDB-backed model whose
/// on-disk shape is a separate "raw" struct, converted via `From<RawType>`.
///
/// Placed above the *actual* (decoded) struct definition, it:
/// 1. Re-emits the struct unchanged.
/// 2. Generates an
///    `impl mangoengine::DBCollectionRowTraitFromRaw<Struct, RawType> for Struct`
///    block containing `collection_name()`, `collection_cell()` (with a
///    per-struct `static OnceCell`), and `get_id()` (delegating to `self._id`).
///
/// The actual struct must have an `_id: ObjectId` field, and you must provide
/// an `impl From<RawType> for Struct` separately.
///
/// # Example
/// ```ignore
/// use mangoengine::db_collection_from_raw;
///
/// #[derive(Serialize, Deserialize, Debug)]
/// pub struct RawChapter {
///     _id: ObjectId,
///     title: String,
///     views: Option<u64>,
/// }
///
/// #[db_collection_from_raw("chapters", RawChapter)]
/// #[derive(Serialize, Deserialize, Debug)]
/// pub struct Chapter {
///     pub _id: ObjectId,
///     pub title: String,
///     pub views: u64,
/// }
///
/// impl From<RawChapter> for Chapter {
///     fn from(raw: RawChapter) -> Self {
///         Chapter {
///             _id: raw._id,
///             title: raw.title,
///             views: raw.views.unwrap_or(0),
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn db_collection_from_raw(attr: TokenStream, item: TokenStream) -> TokenStream {
    let FromRawArgs {
        collection_name,
        raw_type,
    } = parse_macro_input!(attr as FromRawArgs);
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let expanded = quote! {
        #input

        impl ::mangoengine::DBCollectionRowTraitFromRaw<#struct_name, #raw_type> for #struct_name {
            fn collection_name() -> &'static str {
                #collection_name
            }

            fn collection_cell() -> &'static ::tokio::sync::OnceCell<
                ::mongodb::Collection<::mongodb::bson::Document>,
            > {
                static CELL: ::tokio::sync::OnceCell<
                    ::mongodb::Collection<::mongodb::bson::Document>,
                > = ::tokio::sync::OnceCell::const_new();
                &CELL
            }

            fn get_id(&self) -> ::mongodb::bson::oid::ObjectId {
                self._id.clone()
            }
        }
    };

    TokenStream::from(expanded)
}
