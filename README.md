# mangoengine

A lightweight, opinionated MongoDB ODM for Rust. Define a struct, annotate it with `#[db_collection("...")]`, and get async CRUD methods for free — no boilerplate, no per-call collection lookups.

Inspired by Python's [MongoEngine](http://mongoengine.org/), built on top of the official [`mongodb`](https://crates.io/crates/mongodb) driver.

## Features

- **Attribute-macro models** — `#[db_collection("name")]` wires any struct up as a MongoDB-backed model.
- **Raw ↔ decoded models** — `#[db_collection_from_raw("name", RawType)]` lets you keep a permissive on-disk shape (`Option`s, looser numeric types) and decode into a clean domain struct via `From<RawType>`.
- **Async CRUD out of the box** — `create`, `find`, `find_one`, `update_one`, `delete_one`, `count`, `aggregate`, `save`, `delete`.
- **Single global connection** — initialize once at startup with `connect(...)`, then call model methods from anywhere.

## Installation

```toml
[dependencies]
mangoengine = { git = "https://github.com/akay25/mangoengine" }
mongodb = { version = "3.1", default-features = false, features = ["rustls-tls", "compat-3-0-0"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## Quick start

```rust
use mangoengine::{connect, db_collection, ConnectOptions, DBCollectionRowTrait};
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[db_collection("users")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub _id: ObjectId,
    pub name: String,
    pub email: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect once at startup.
    connect(ConnectOptions::new("mongodb://localhost:27017", "myapp")).await?;

    // 2. Insert.
    let user = User {
        _id: ObjectId::new(),
        name: "Ada".into(),
        email: "ada@example.com".into(),
    };
    User::create(&user).await;

    // 3. Query.
    let found = User::find_one(doc! { "email": "ada@example.com" }, None).await;
    println!("{:?}", found);

    // 4. Update via instance.
    if let Some(mut u) = found {
        u.name = "Ada Lovelace".into();
        u.save().await;
    }

    // 5. Delete.
    User::delete_one(doc! { "email": "ada@example.com" }).await;

    Ok(())
}
```

## Defining a model

Any struct with an `_id: ObjectId` field can become a model. The `#[db_collection("...")]` attribute generates the `DBCollectionRowTrait` implementation.

```rust
#[db_collection("shard_locks")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lock {
    pub _id: ObjectId,
    pub name: String,
    pub acquired_at: i64,
}
```

## Raw + decoded models

Real-world Mongo collections often contain optional fields, looser numeric types, or legacy shapes you don't want leaking into the rest of your code. `#[db_collection_from_raw("name", RawType)]` lets you read into a permissive **raw** struct and expose a clean **decoded** struct to callers, with a single `From<RawType>` impl bridging the two.

```rust
use mangoengine::{db_collection_from_raw, DBCollectionRowTraitFromRaw};
use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

// On-disk shape — matches whatever the DB actually contains.
#[derive(Serialize, Deserialize, Debug)]
pub struct RawChapter {
    _id: ObjectId,
    title: String,
    thumbnail: Option<String>,
    views: Option<u64>,
    release_date: DateTime,
}

// Clean shape used by the rest of the app.
#[db_collection_from_raw("chapters", RawChapter)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Chapter {
    pub _id: ObjectId,
    pub title: String,
    pub thumbnail: String,
    pub views: u64,
    pub release_date: DateTime,
}

impl From<RawChapter> for Chapter {
    fn from(raw: RawChapter) -> Self {
        Chapter {
            _id: raw._id,
            title: raw.title,
            thumbnail: raw.thumbnail.unwrap_or_default(),
            views: raw.views.unwrap_or(0),
            release_date: raw.release_date,
        }
    }
}

// Reads return `Chapter`; writes take `RawChapter`.
let chapters: Vec<Chapter> = Chapter::find(doc! { "views": { "$gt": 0 } }, None).await;
```

The same CRUD surface as `DBCollectionRowTrait` is available on `DBCollectionRowTraitFromRaw`, with one difference: `find` / `find_one` return the decoded `ActualType`, while `create` accepts a `RawType`.

## API

Methods provided by [`DBCollectionRowTrait`](src/db_collection_row_trait.rs):

| Method | Description |
| --- | --- |
| `create(&doc)` | Insert one document. |
| `find(filter, sort)` | Return all matching documents as a `Vec<T>`. |
| `find_one(filter, sort)` | Return the first match, if any. |
| `update_one(filter, update)` | `$set` the given fields on the first match. |
| `delete_one(filter)` | Delete the first match. |
| `count(filter)` | Count matching documents. |
| `aggregate(pipeline)` | Run an aggregation pipeline, returning raw `Document`s. |
| `save(&self)` | Persist the current struct (matched by `_id`). |
| `delete(&self)` | Delete the current struct (matched by `_id`). |

### `ConnectOptions`

```rust
ConnectOptions {
    uri: String,                      // e.g. "mongodb://localhost:27017"
    db_name: String,                  // database to use for all models
    max_pool_size: Option<u32>,       // forwarded to ClientOptions
    max_idle_time: Option<Duration>,  // forwarded to ClientOptions
}
```

Use `ConnectOptions::new(uri, db_name)` for the common case.

## Design notes

- `connect()` stores the `Database` in a global `OnceLock`. All model methods reach it through `get_db()`. Call `connect` exactly once — subsequent calls are silently ignored.
- Each `#[db_collection]` expansion declares its own `static OnceCell<Collection<Document>>`, so collection handles are initialized lazily, per model, with no shared lock contention on the hot path.
- `find` currently buffers the full result set into a `Vec<T>`. A streaming cursor API is planned (see the TODO in [src/db_collection_row_trait.rs](src/db_collection_row_trait.rs)).

## Status

Early-stage and intentionally minimal. Expect API changes.

## License

MIT
