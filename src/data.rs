//! Poise framework data

use std::time::Duration;

use dashmap::DashMap;
use mongodb::{Client, Database};

use crate::common::QuickPost;

pub struct Data {
    pub bot_name: String,
    pub bot_tag: String,

    pub mongo: Client,
    pub db: Database,

    pub cache_ttl: Duration,
    pub blacklist_ttl: Duration,

    pub servers: DashMap<String, String>,
    pub nsfw: DashMap<String, bool>,
    pub posts: DashMap<String, QuickPost>,
    pub blacklist: DashMap<String, QuickPost>,
    pub last_post: DashMap<String, QuickPost>,
    pub subs: DashMap<String, Vec<String>>,

    pub request_count: DashMap<String, u8>,
    pub req_timer: DashMap<String, Duration>,
    pub queue_state: DashMap<String, bool>,
}
