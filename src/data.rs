//! Bot runtime data.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::RateLimiter;
use mongodb::bson::doc;
use mongodb::{Client, Database};
use once_cell::sync::OnceCell;
use poise::serenity_prelude::ChannelId;
use roux::subreddit::responses::Submissions;

use crate::db::{Channel, ChannelInfo};

/// Map of subreddit groups and subreddit names from `subs.json`.
pub static SUBS: OnceCell<HashMap<String, Vec<String>>> = OnceCell::new();

/// Bot runtime data.
#[derive(Debug)]
pub struct Data {
    /// The bot's user ID.
    pub bot_id: u64,
    /// The bot's tag.
    pub bot_tag: String,
    /// The bot's name.
    pub bot_name: String,

    /// Mongo database client.
    pub mongo: Client,
    /// The default mongo database.
    pub db: Database,

    /// The last time posts were updated.
    pub cache_time: DateTime<Utc>,
    /// The last time the blacklist was reset.
    pub blacklist_time: DateTime<Utc>,

    /// Map of subreddit names and their top 100 hot posts.
    pub posts: Arc<DashMap<String, Vec<QuickPost>>>,

    /// Map of discord channel IDs the bot is active in, and the channels' names and nsfw statuses.
    pub channels: Arc<DashMap<ChannelId, ChannelInfo>>,
    /// Map of discord channel IDs and blacklisted reddit posts.
    pub blacklist: Arc<DashMap<ChannelId, Vec<QuickPost>>>,
    /// Map of discord channel IDs and their last post.
    pub last_post: Arc<DashMap<ChannelId, QuickPost>>,

    /// Request rate limiter keyed by discord channel ID.
    pub governor: Arc<RateLimiter<ChannelId, DefaultKeyedStateStore<ChannelId>, DefaultClock>>,
}

/// Specific data for a reddit post.
#[derive(Debug)]
pub struct QuickPost {
    pub title: String,
    pub score: f64,
    pub content: String,
    pub nsfw: bool,
    pub permalink: String,
    pub sub: String,
}

impl Data {
    /// Add `QuickPost`s to the cache.
    pub fn add_posts(&mut self, sub: String, posts: Vec<QuickPost>) {
        match self.posts.entry(sub) {
            Entry::Occupied(ref mut entry) => entry.get_mut().extend(posts),
            Entry::Vacant(entry) => {
                entry.insert(posts);
            }
        }
    }

    /// Add a `QuickPost` to the blacklist.
    pub fn add_blacklist(&mut self, channel: ChannelId, post: QuickPost) {
        match self.blacklist.entry(channel) {
            Entry::Occupied(ref mut entry) => entry.get_mut().push(post),
            Entry::Vacant(entry) => {
                entry.insert(vec![post]);
            }
        }
    }

    /// Reset the blacklist and the blacklist time.
    pub fn reset_blacklist(&mut self) {
        self.blacklist = Arc::new(DashMap::new());
        self.blacklist_time = Utc::now() + Duration::hours(3);
    }

    /// Update the blacklist time and reset the blacklist if the current time is greater than the
    /// original blacklist time.
    pub fn update_blacklist_time(&mut self) {
        if Utc::now() >= self.blacklist_time {
            self.reset_blacklist();
        }
    }

    /// Add channel info to the database.
    pub async fn add_db_channel(&mut self, channel: ChannelId, info: ChannelInfo) -> Result<()> {
        let channels = self.db.collection::<Channel>("channels");
        let channel_id = channel.0.to_string();
        let time = Utc::now().timestamp();

        let doc = channels
            .find_one_and_update(
                doc! {
                    "$or": {
                        "channelID": &channel_id,
                        "channelid": &channel_id,
                    }
                },
                doc! {
                    "$set": {
                        "channelID": &channel_id,
                        "name": &info.name,
                        "nsfw": &info.nsfw,
                        "time": time,
                    }
                },
                None,
            )
            .await?;

        if doc.is_none() {
            channels
                .insert_one(
                    Channel {
                        channel_id: channel,
                        info,
                        time,
                        id: None,
                    },
                    None,
                )
                .await?;
        }

        Ok(())
    }
}

/// Convert reddit posts (submissions) to `QuickPost`s.
pub fn submissions_to_quickposts(submissions: Submissions) -> Vec<QuickPost> {
    submissions
        .data
        .children
        .into_iter()
        .map(|submission| {
            let data = submission.data;
            // For a link or media post, use the content URL, otherwise use selftext
            let content = data.url.unwrap_or(data.selftext);

            QuickPost {
                title: data.title,
                score: data.score,
                content,
                nsfw: data.over_18,
                permalink: data.permalink,
                sub: data.subreddit,
            }
        })
        .collect()
}
