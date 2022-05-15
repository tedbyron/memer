//! Mongo stuff

use std::env;

use anyhow::{anyhow, bail, Result};
use mongodb::bson::oid::ObjectId;
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Channel {
    #[serde(rename = "channel")]
    channel_id: String,
    nsfw: bool,
    name: String,
    time: i64,
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BannedSub {
    #[serde(rename = "channel")]
    channel_id: String,
    subreddit: String,
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
}

/// Create a mongodb client.
pub async fn client() -> Result<(Client, Database)> {
    let mongo_uri = match env::var("MEMER_MONGO_URI") {
        Ok(mongo_uri) => mongo_uri,
        Err(_) => bail!("missing MEMER_MONGO_URI environment variable"),
    };
    let mut client_options = ClientOptions::parse(mongo_uri).await?;
    let db = match env::var("MEMER_MONGO_DB") {
        Ok(mongo_db) => mongo_db,
        Err(_) => bail!("missing MEMER_MONGO_DB environment variable"),
    };

    client_options.default_database = Some(db);
    // TODO: client_options.tls?
    // TODO: client_options.max_idle_time?

    let client = mongodb::Client::with_options(client_options)?;
    let db = client
        .default_database()
        .ok_or_else(|| anyhow!("failed to set default database"))?;

    Ok((client, db))
}

// /// Get all channel names from the database.
// pub async fn channel_names(db: &Database) -> Result<()> {
//     let mut cursor = db.collection("channels").find(None, None).await?;

//     while let Some(doc) = cursor.next().await {
//         let channel = bson::from_bson::<Channel>(Bson::Document(doc?))?;
//         println!("{}", channel.name);
//     }

//     Ok(())
// }
