use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    Collection,
};
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

pub use mongodb;

// Local imports
use crate::utils::get_db;

pub trait DBCollectionRowTrait<T: Serialize + Send + Sync + for<'de> Deserialize<'de>> {
    fn collection_name() -> &'static str;

    // Each impl supplies its own `OnceCell<Collection<Document>>` — typically
    // via the `#[db_collection]` attribute macro. Because the static lives at
    // the per-impl expansion site (not a generic default method body), every
    // model gets its own independent cell: zero locks, zero contention on the
    // hot path.
    fn collection_cell() -> &'static OnceCell<Collection<Document>>;

    async fn get_collection() -> Collection<T> {
        Self::collection_cell()
            .get_or_init(|| async { get_db().collection(Self::collection_name()) })
            .await
            .clone_with_type()
    }

    // Create one
    async fn create(new_doc: &T) -> bool {
        let collection = Self::get_collection().await;

        match collection.insert_one(new_doc).await {
            Ok(_) => true,
            Err(e) => {
                debug!("Error fetching data from database. Error: {:#?}", e);
                error!("Error fetching data from database");
                false
            }
        }
    }

    // Find all
    // TODO: Return cursor iterator instead of whole array
    async fn find(filter: Document, optional_sort_query: Option<Document>) -> Vec<T> {
        let collection = Self::get_collection().await;
        let mut query = collection.find(filter);
        if let Some(sort_fields) = optional_sort_query {
            query = query.sort(sort_fields);
        }

        let mut cursor = query.await.unwrap();
        let mut rows: Vec<T> = Vec::new();
        while let Some(doc) = cursor.try_next().await.unwrap() {
            rows.push(doc);
        }
        rows
    }

    async fn find_one(filter: Document, optional_sort_query: Option<Document>) -> Option<T> {
        let collection = Self::get_collection().await;

        let mut query = collection.find_one(filter);
        if let Some(sort_fields) = optional_sort_query {
            query = query.sort(sort_fields);
        }

        match query.await {
            Ok(doc) => doc,
            Err(e) => {
                debug!("Error fetching data from database. Error: {:#?}", e);
                error!("Error fetching data from database");
                None
            }
        }
    }

    async fn update_one(query: Document, update: Document) -> bool {
        let update_query = doc! {"$set": update};
        let collection = Self::get_collection().await;

        match collection.update_one(query, update_query).await {
            Ok(doc) => {
                if doc.matched_count > 0 {
                    return true;
                }
                debug!("No matched record found in db");
                false
            }
            Err(e) => {
                error!("Error fetching data from database. Error: {:#?}", e);
                false
            }
        }
    }

    async fn delete_one(query: Document) -> bool {
        let collection = Self::get_collection().await;

        match collection.delete_one(query).await {
            Ok(doc) => {
                if doc.deleted_count > 0 {
                    return true;
                }
                debug!("No matched record found in db");
                false
            }
            Err(e) => {
                error!("Error fetching data from database. Error: {:#?}", e);
                false
            }
        }
    }

    async fn count(query: Document) -> Option<u64> {
        let collection = Self::get_collection().await;
        match collection.count_documents(query).await {
            Ok(total_records) => Some(total_records),
            Err(e) => {
                debug!("Error fetching data from database. Error: {:#?}", e);
                error!("Error fetching data from database");
                None
            }
        }
    }

    async fn aggregate(pipeline: Vec<Document>) -> Vec<Document> {
        let collection: Collection<T> = Self::get_collection().await;
        let mut cursor = collection.aggregate(pipeline).await.unwrap();

        let mut rows: Vec<Document> = Vec::new();
        while let Some(doc) = cursor.try_next().await.unwrap() {
            rows.push(doc);
        }
        rows
    }

    // Object based functions
    fn get_id(&self) -> ObjectId;

    async fn save(&self) -> bool
    where
        Self: Serialize,
    {
        let update_doc = match mongodb::bson::to_bson(self) {
            Ok(bson) => bson.as_document().cloned().unwrap_or_default(),
            Err(e) => {
                debug!("Error converting self to BSON. Error: {:#?}", e);
                error!("Error converting self to BSON");
                return false;
            }
        };
        Self::update_one(doc! {"_id": self.get_id()}, update_doc).await
    }

    async fn delete(&self) -> bool {
        Self::delete_one(doc! {"_id": self.get_id()}).await
    }
}
