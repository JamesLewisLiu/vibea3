use std::sync::Arc;
use std::time::Duration;

use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::doc,
    options::{IndexOptions, ReturnDocument as MongoReturnDocument},
};
use vibea3::{
    DbFuture, Document, DocumentDatabase, FindOneAndUpdateOptions, FindOneOptions, FindOptions,
    IndexDefinition, ReturnDocument, Update, UpdateOptions,
};

#[derive(Clone)]
pub struct MongoStore {
    database: Database,
}

impl MongoStore {
    pub async fn connect(uri: &str, name: &str) -> Result<Arc<Self>, String> {
        let client = Client::with_uri_str(uri).await.map_err(error)?;
        let database = client.database(name);
        database
            .run_command(doc! { "ping": 1 })
            .await
            .map_err(error)?;
        Ok(Arc::new(Self { database }))
    }

    fn collection(&self, name: &str) -> Collection<Document> {
        self.database.collection(name)
    }
}

impl DocumentDatabase for MongoStore {
    fn create_indexes(&self, collection: String, indexes: Vec<IndexDefinition>) -> DbFuture<()> {
        let store = self.clone();
        Box::pin(async move {
            if indexes.is_empty() {
                return Ok(());
            }
            let indexes = indexes.into_iter().map(|index| {
                let options = IndexOptions::builder()
                    .name(index.name)
                    .unique(index.unique)
                    .expire_after(index.expire_after_seconds.map(Duration::from_secs))
                    .build();
                IndexModel::builder()
                    .keys(index.keys)
                    .options(options)
                    .build()
            });
            store
                .collection(&collection)
                .create_indexes(indexes)
                .await
                .map_err(error)?;
            Ok(())
        })
    }

    fn drop_index(&self, collection: String, name: String) -> DbFuture<()> {
        let store = self.clone();
        Box::pin(async move {
            let collection = store.collection(&collection);
            let names = collection.list_index_names().await.map_err(error)?;
            if names.iter().any(|existing| existing == &name) {
                collection.drop_index(name).await.map_err(error)?;
            }
            Ok(())
        })
    }

    fn find_one(
        &self,
        collection: String,
        filter: Document,
        options: FindOneOptions,
    ) -> DbFuture<Option<Document>> {
        let store = self.clone();
        Box::pin(async move {
            let collection = store.collection(&collection);
            let mut action = collection.find_one(filter);
            if let Some(sort) = options.sort {
                action = action.sort(sort);
            }
            action.await.map_err(error)
        })
    }

    fn find_many(
        &self,
        collection: String,
        filter: Document,
        options: FindOptions,
    ) -> DbFuture<Vec<Document>> {
        let store = self.clone();
        Box::pin(async move {
            let collection = store.collection(&collection);
            let mut action = collection.find(filter);
            if let Some(sort) = options.sort {
                action = action.sort(sort);
            }
            if let Some(limit) = options.limit {
                action = action.limit(limit as i64);
            }
            let mut cursor = action.await.map_err(error)?;
            let mut documents = Vec::new();
            while cursor.advance().await.map_err(error)? {
                documents.push(cursor.deserialize_current().map_err(error)?);
            }
            Ok(documents)
        })
    }

    fn update_one(
        &self,
        collection: String,
        filter: Document,
        update: Update,
        options: UpdateOptions,
    ) -> DbFuture<()> {
        let store = self.clone();
        Box::pin(async move {
            let collection = store.collection(&collection);
            match update {
                Update::Document(update) => {
                    collection
                        .update_one(filter, update)
                        .upsert(options.upsert)
                        .await
                }
                Update::Pipeline(update) => {
                    collection
                        .update_one(filter, update)
                        .upsert(options.upsert)
                        .await
                }
            }
            .map_err(error)?;
            Ok(())
        })
    }

    fn find_one_and_update(
        &self,
        collection: String,
        filter: Document,
        update: Update,
        options: FindOneAndUpdateOptions,
    ) -> DbFuture<Option<Document>> {
        let store = self.clone();
        Box::pin(async move {
            let collection = store.collection(&collection);
            let return_document = match options.return_document {
                ReturnDocument::Before => MongoReturnDocument::Before,
                ReturnDocument::After => MongoReturnDocument::After,
            };
            match update {
                Update::Document(update) => {
                    collection
                        .find_one_and_update(filter, update)
                        .upsert(options.upsert)
                        .return_document(return_document)
                        .await
                }
                Update::Pipeline(update) => {
                    collection
                        .find_one_and_update(filter, update)
                        .upsert(options.upsert)
                        .return_document(return_document)
                        .await
                }
            }
            .map_err(error)
        })
    }

    fn insert_one(&self, collection: String, document: Document) -> DbFuture<()> {
        let store = self.clone();
        Box::pin(async move {
            store
                .collection(&collection)
                .insert_one(document)
                .await
                .map_err(error)?;
            Ok(())
        })
    }

    fn insert_many(&self, collection: String, documents: Vec<Document>) -> DbFuture<()> {
        let store = self.clone();
        Box::pin(async move {
            if documents.is_empty() {
                return Ok(());
            }
            store
                .collection(&collection)
                .insert_many(documents)
                .await
                .map_err(error)?;
            Ok(())
        })
    }

    fn delete_one(&self, collection: String, filter: Document) -> DbFuture<()> {
        let store = self.clone();
        Box::pin(async move {
            store
                .collection(&collection)
                .delete_one(filter)
                .await
                .map_err(error)?;
            Ok(())
        })
    }

    fn delete_many(&self, collection: String, filter: Document) -> DbFuture<()> {
        let store = self.clone();
        Box::pin(async move {
            store
                .collection(&collection)
                .delete_many(filter)
                .await
                .map_err(error)?;
            Ok(())
        })
    }
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
