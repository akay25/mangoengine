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


pub trait DBCollectionRowTraitFromRaw<
    ActualType: From<RawType>,
    RawType: Serialize + Send + Sync + for<'de> Deserialize<'de>,
>
{
    fn collection_name() -> &'static str;

    // Each impl supplies its own `OnceCell<Collection<Document>>` — typically
    // via the `#[db_collection]` attribute macro. Because the static lives at
    // the per-impl expansion site (not a generic default method body), every
    // model gets its own independent cell: zero locks, zero contention on the
    // hot path.
    fn collection_cell() -> &'static OnceCell<Collection<Document>>;

    async fn get_collection() -> Collection<RawType> {
        Self::collection_cell()
            .get_or_init(|| async { get_db().collection(Self::collection_name()) })
            .await
            .clone_with_type()
    }

    // Create one
    async fn create(new_doc: RawType) -> bool {
        let collection = Self::get_collection().await;

        match collection.insert_one(new_doc).await {
            Ok(_res) => true,
            Err(e) => {
                debug!("Error inserting data into database. Error: {:#?}", e);
                error!("Error inserting data into database");
                false
            }
        }
    }

    // Find all
    async fn find(filter: Document, optional_sort_query: Option<Document>) -> Vec<ActualType> {
        let collection = Self::get_collection().await;

        let mut query = collection.find(filter);
        if let Some(sort_fields) = optional_sort_query {
            query = query.sort(sort_fields);
        }

        match query.await {
            Ok(cursor) => {
                let raw_rows: Vec<RawType> = cursor.try_collect().await.unwrap_or_else(|e| {
                    error!("Error reading cursor from database: {:#?}", e);
                    Vec::new()
                });
                raw_rows.into_iter().map(ActualType::from).collect()
            }
            Err(e) => {
                error!("Error querying database: {:#?}", e);
                Vec::new()
            }
        }
    }

    async fn find_one(
        filter: Document,
        optional_sort_query: Option<Document>,
    ) -> Option<ActualType> {
        let collection = Self::get_collection().await;

        let mut query = collection.find_one(filter);
        if let Some(sort_fields) = optional_sort_query {
            query = query.sort(sort_fields);
        }

        match query.await {
            Ok(doc) => doc.map(ActualType::from),
            Err(e) => {
                debug!("Error fetching data from database. Error: {:#?}", e);
                error!("Error fetching data from database");
                None
            }
        }
    }

    // Update one
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
                error!("Error updating data in database. Error: {:#?}", e);
                false
            }
        }
    }

    // Delete one
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
                error!("Error deleting data from database. Error: {:#?}", e);
                false
            }
        }
    }

    // Count documents
    async fn count(query: Document) -> Option<u64> {
        let collection = Self::get_collection().await;
        match collection.count_documents(query).await {
            Ok(total_records) => Some(total_records),
            Err(e) => {
                debug!("Error counting documents. Error: {:#?}", e);
                error!("Error counting documents");
                None
            }
        }
    }

    // Aggregate function
    async fn aggregate(pipeline: Vec<Document>) -> Vec<Document> {
        let collection: Collection<RawType> = Self::get_collection().await;
        match collection.aggregate(pipeline).await {
            Ok(cursor) => cursor.try_collect().await.unwrap_or_else(|e| {
                error!("Error reading aggregation cursor: {:#?}", e);
                Vec::new()
            }),
            Err(e) => {
                error!("Error running aggregation: {:#?}", e);
                Vec::new()
            }
        }
    }

    // Object based functions
    fn get_id(&self) -> ObjectId;

    // Save function for record
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

    // Delete function for record
    async fn delete(&self) -> bool {
        let collection = Self::get_collection().await;
        match collection.delete_one(doc! {"_id": self.get_id()}).await {
            Ok(doc) => {
                if doc.deleted_count > 0 {
                    return true;
                }
                debug!("No matched record found in db");
                false
            }
            Err(e) => {
                error!("Error deleting data from database. Error: {:#?}", e);
                false
            }
        }
    }
}