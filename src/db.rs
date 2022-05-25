//! Mongo stuff.

use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};
use dashmap::DashMap;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, Bson};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use poise::futures_util::{future, Stream, StreamExt};
use poise::serenity_prelude::ChannelId;
use tracing::error;

/// Discord channel data for mongo.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Channel {
    #[serde(rename = "channel", with = "crate::serde::channel_id")]
    pub channel_id: ChannelId,
    #[serde(flatten)]
    pub info: ChannelInfo,
    pub time: i64,
    #[serde(rename = "_id", skip_serializing)]
    pub id: Option<ObjectId>,
}

/// Discord channel information.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    pub nsfw: bool,
}

/// A discord channel that has banned a subreddit.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BannedSub {
    #[serde(rename = "channelID", with = "crate::serde::channel_id")]
    pub channel_id: ChannelId,
    pub subreddit: String,
    #[serde(rename = "_id", skip_serializing)]
    pub id: Option<ObjectId>,
}

/// Create a mongodb client.
#[tracing::instrument]
pub async fn client_and_db() -> Result<(Client, Database)> {
    let mongo_uri =
        env::var("MEMER_MONGO_URI").context("missing MEMER_MONGO_URI environment variable")?;
    let mut client_options = ClientOptions::parse(mongo_uri).await?;
    let db = env::var("MEMER_MONGO_DB").context("missing MEMER_MONGO_DB environment variable")?;

    client_options.default_database = Some(db);
    // TODO: client_options.tls?
    // TODO: client_options.max_idle_time?

    let client = mongodb::Client::with_options(client_options)?;
    let db = client
        .default_database()
        .context("failed to set the default database")?;

    Ok((client, db))
}

/// Get all active channels' info from the database.
#[tracing::instrument(skip_all)]
pub async fn all_channels(db: &Database) -> Result<Arc<DashMap<ChannelId, ChannelInfo>>> {
    // TODO: explicitly type collection if there are no errors deserializing
    let cursor = db.collection("channels").find(None, None).await?;
    let (lo, hi) = cursor.size_hint();
    let channels = Arc::new(DashMap::with_capacity(hi.unwrap_or(lo)));

    let handles = cursor
        .map(|res| {
            let channels = Arc::clone(&channels);

            tokio::spawn(async move {
                if let Ok(doc) = res {
                    match bson::from_bson::<Channel>(Bson::Document(doc)) {
                        Ok(channel) => {
                            channels.insert(channel.channel_id, channel.info);
                        }
                        Err(e) => error!("failed to deserialize channel from bson: {e}"),
                    }
                }
            })
        })
        .collect::<Vec<_>>()
        .await;
    future::join_all(handles).await;

    Ok(channels)
}
