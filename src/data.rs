//! Poise framework data

use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use mongodb::{Client, Database};
use poise::futures_util::future;
use roux::responses::BasicThing;
use roux::subreddit::responses::Submissions;
use roux::subreddit::Subreddit;
use tracing::error;

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
    pub subs: Subs,

    pub request_count: DashMap<String, u8>,
    pub req_timer: DashMap<String, Duration>,
    pub queue_state: DashMap<String, bool>,
}

#[derive(Debug)]
pub struct QuickPost {
    pub title: String,
    pub score: f64,
    pub content: String,
    pub nsfw: bool,
    pub permalink: String,
    pub sub: String,
}

pub type Subs = DashMap<String, Vec<String>>;

impl Data {
    pub fn add_posts(&mut self, sub: &str, posts: &mut Vec<QuickPost>) {
        let mut entry = self
            .posts
            .entry(sub.to_string())
            .or_insert(Vec::with_capacity(posts.len()));
        (*entry).append(&mut posts);
    }

    pub fn clear_posts(&mut self) {
        self.posts = Arc::new(DashMap::with_capacity(self.subs.len()));
    }
}

/// Load subreddits from `subs.json`.
pub fn subs_from_file() -> Result<Subs> {
    let path = env::current_dir()
        .context("failed to get cwd")?
        .join("subs.json");
    let contents = fs::read_to_string(path)
        .with_context(|| &format!("failed to read file: {}", path.display()))?;
    let subs = serde_json::from_str::<Subs>(&contents)
        .with_context(|| format!("failed to deserialize file: {}", path.display()))?;

    Ok(subs)
}

/// Get the top 25 hot reddit posts for all subs.
pub async fn hot_posts(subs: &Subs) -> Arc<DashMap<String, Vec<QuickPost>>> {
    let subs = subs.clone().into_read_only();
    let subs = subs.values().flatten().collect::<Vec<_>>();
    let len = subs.len();
    let posts = Arc::new(DashMap::with_capacity(len));

    future::join_all(subs.into_iter().map(|sub| get_hot(sub, Arc::clone(&posts)))).await;

    posts
}

/// Get the top 25 hot reddit posts from a sub.
async fn get_hot<S>(sub_name: S, posts: Arc<DashMap<String, Vec<QuickPost>>>)
where
    S: AsRef<str>,
{
    let sub_name = sub_name.as_ref();
    let sub = Subreddit::new(sub_name);
    let hot = match sub.hot(25, None).await {
        Ok(hot) => hot,
        Err(_) => {
            error!("failed to get hot posts for {sub_name}");
            return;
        }
    };

    posts.insert(sub_name.to_string(), submissions_to_quickposts(&hot));
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
