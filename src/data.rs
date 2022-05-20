//! Bot runtime data

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use mongodb::{Client, Database};
use poise::serenity_prelude::ChannelId;
use roux::subreddit::responses::Submissions;

use crate::db::ChannelInfo;

#[derive(Debug)]
pub struct Data {
    /// The bot's `UserId`.
    pub bot_id: u64,
    /// The bot's tag.
    pub bot_tag: String,
    /// The bot's name.
    pub bot_name: String,

    /// Database client.
    pub mongo: Client,
    /// The default database.
    pub db: Database,

    pub cache_time: DateTime<Utc>,
    pub blacklist_time: DateTime<Utc>,

    /// Map of discord channel IDs the bot is active in, and the channels' names and nsfw statuses.
    pub channels: Arc<DashMap<ChannelId, ChannelInfo>>,
    /// Map of subreddit names and their top 100 hot posts.
    pub posts: Arc<DashMap<String, Vec<QuickPost>>>,
    pub blacklist: Arc<DashMap<String, Vec<QuickPost>>>,
    pub last_post: Arc<DashMap<String, QuickPost>>,
    /// Map of subreddit groups and subreddit names.
    pub subs: HashMap<String, Vec<String>>,

    pub request_count: Arc<DashMap<String, u8>>,
    pub req_timer: Arc<DashMap<String, Duration>>,
    pub queue_state: Arc<DashMap<String, bool>>,
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
    /// Add reddit posts to the cache.
    #[inline]
    pub fn add_posts(&mut self, sub: String, posts: Vec<QuickPost>) {
        let mut entry = self
            .posts
            .entry(sub)
            .or_insert(Vec::with_capacity(posts.len()));
        (*entry).extend(posts);
    }
}

/// Convert reddit posts (submissions) to `QuickPost`s.
pub fn submissions_to_quickposts(submissions: &Submissions) -> Vec<QuickPost> {
    submissions
        .data
        .children
        .iter()
        .map(|submission| {
            let data = submission.data;
            // For a link or media post, use the content URL, otherwise use selftext
            let content = data.url.unwrap_or_else(|| data.selftext.clone());

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
