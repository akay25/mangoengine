use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr};

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
