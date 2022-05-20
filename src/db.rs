//! Mongo stuff

use std::env;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use dashmap::DashMap;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, Bson};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use poise::futures_util::{future, Stream, StreamExt};
use poise::serenity_prelude::ChannelId;
use tracing::error;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Channel {
    #[serde(rename = "channel", with = "crate::channel_id")]
    pub channel_id: ChannelId,
    #[serde(flatten)]
    pub info: ChannelInfo,
    pub time: i64,
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    pub nsfw: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BannedSub {
    #[serde(rename = "channel")]
    pub channel_id: String,
    pub subreddit: String,
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
}

/// Create a mongodb client.
#[tracing::instrument]
pub async fn client_and_db() -> Result<(Client, Database)> {
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

/// Get all channel names from the database.
#[tracing::instrument(skip_all)]
pub async fn channels(db: &Database) -> Result<Arc<DashMap<ChannelId, ChannelInfo>>> {
    let mut cursor = db.collection("channels").find(None, None).await?;
    let (lo, hi) = cursor.size_hint();
    let size = hi.unwrap_or(lo);
    let channels = Arc::new(DashMap::with_capacity(size));
    let mut handles = Vec::with_capacity(size);

    while let Some(doc) = cursor.next().await {
        let channels = Arc::clone(&channels);

        handles.push(tokio::spawn(async move {
            if let Ok(doc) = doc {
                match bson::from_bson::<Channel>(Bson::Document(doc)) {
                    Ok(channel) => {
                        channels.insert(channel.channel_id, channel.info);
                    }
                    Err(e) => error!("failed to deserialize channel from bson: {e}"),
                }
            }
        }))
    }

    future::join_all(handles).await;

    Ok(channels)
}
