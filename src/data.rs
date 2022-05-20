//! Poise framework data

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use mongodb::{Client, Database};
use roux::responses::BasicThing;
use roux::subreddit::responses::Submissions;

#[derive(Debug)]
pub struct Data {
    pub bot_name: String,
    pub bot_tag: String,

    pub mongo: Client,
    pub db: Database,

    pub cache_time: DateTime<Utc>,
    pub blacklist_time: DateTime<Utc>,

    pub servers: DashMap<String, String>,
    pub nsfw: DashMap<String, bool>,
    pub posts: Arc<DashMap<String, Vec<QuickPost>>>,
    pub blacklist: Arc<DashMap<String, Vec<QuickPost>>>,
    pub last_post: Arc<DashMap<String, QuickPost>>,
    pub subs: SubMap,

    pub request_count: DashMap<String, u8>,
    pub req_timer: DashMap<String, Duration>,
    pub queue_state: DashMap<String, bool>,
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

/// Map of subreddit genres and subreddit names.
pub type SubMap = DashMap<String, Vec<String>>;

impl Data {
    /// Add reddit posts to the cache.
    pub fn add_posts(&mut self, sub: &str, posts: &mut Vec<QuickPost>) {
        let mut entry = self
            .posts
            .entry(sub.to_string())
            .or_insert(Vec::with_capacity(posts.len()));
        (*entry).append(&mut posts);
    }

    /// Clear the cached reddit posts, keeping the allocated memory.
    pub fn clear_posts(&mut self) {
        self.posts.clear();
    }
}

/// Convert reddit posts (submissions) to `QuickPost`s.
pub fn submissions_to_quickposts(submissions: &Submissions) -> Vec<QuickPost> {
    submissions
        .data
        .children
        .iter()
        .map(|submission| {
            let BasicThing { data, .. } = submission;

            // For a link or media post, use the content URL, otherwise use selftext
            let content = match data.url {
                Some(url) => url,
                None => data.selftext.clone(),
            };

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
