//! Bot runtime data

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::RateLimiter;
use mongodb::{Client, Database};
use once_cell::sync::OnceCell;
use poise::serenity_prelude::ChannelId;
use roux::subreddit::responses::Submissions;

use crate::db::ChannelInfo;

/// Map of subreddit groups and subreddit names from `subs.json`.
pub static SUBS: OnceCell<HashMap<String, Vec<String>>> = OnceCell::new();

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
    // /// Add reddit posts to the cache.
    // #[inline]
    // fn add_posts(&mut self, sub: String, posts: Vec<QuickPost>) {
    //     self.posts
    //         .entry(sub)
    //         .and_modify(|v| v.extend(posts))
    //         .or_insert(posts);
    // }

    // /// Add a reddit post to the blacklist.
    // #[inline]
    // fn add_blacklist(&mut self, channel: ChannelId, post: QuickPost) {
    //     self.blacklist
    //         .entry(channel)
    //         .and_modify(|v| v.push(post))
    //         .or_insert(vec![post]);
    // }

    // /// Reset the blacklist and the blacklist time.
    // #[inline]
    // fn reset_blacklist(&mut self) {
    //     self.blacklist = Arc::new(DashMap::new());
    //     self.blacklist_time = Utc::now() + Duration::hours(3);
    // }

    // /// Update the blacklist time and reset the blacklist if the current time is greater than the
    // /// original blacklist time.
    // #[inline]
    // fn update_blacklist_time(&mut self) {
    //     if Utc::now() >= self.blacklist_time {
    //         self.reset_blacklist();
    //     }
    // }
}

/// Convert reddit posts (submissions) to `QuickPost`s.
pub fn submissions_to_quickposts(submissions: &Submissions) -> Vec<QuickPost> {
    submissions
        .data
        .children
        .iter()
        .map(|submission| {
            let data = &submission.data;
            // For a link or media post, use the content URL, otherwise use selftext
            let content = data
                .url
                .as_ref()
                .map_or_else(|| data.selftext.clone(), ToString::to_string);

            QuickPost {
                title: data.title.clone(),
                score: data.score,
                content,
                nsfw: data.over_18,
                permalink: data.permalink.clone(),
                sub: data.subreddit.clone(),
            }
        })
        .collect()
}
