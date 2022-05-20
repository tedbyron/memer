//! Utility functions

use std::env;
use std::fs;
use std::sync::Arc;

use anyhow::{bail, Context as _, Error, Result};
use dashmap::DashMap;
use poise::builtins::create_application_commands;
use poise::futures_util::future;
use poise::serenity_prelude::*;
use poise::Framework;
use roux::Subreddit;
use tokio::time::Instant;
use tracing::{error, info, warn};

use crate::data::{self, QuickPost, SubMap};
use crate::{Data, TraceErr};

/// Get and validate the bot token.
#[tracing::instrument]
pub fn token() -> Result<String> {
    let token = match env::var("MEMER_TOKEN") {
        Ok(token) => token,
        Err(_) => bail!("missing MEMER_TOKEN environment variable"),
    };

    if validate_token(&token).is_err() {
        bail!("invalid MEMER_TOKEN environment variable");
    }

    Ok(token)
}

/// Get the bot application ID.
#[tracing::instrument]
pub fn app_id() -> Result<u64> {
    match env::var("MEMER_APPLICATION_ID") {
        Ok(id) => match id.parse::<u64>() {
            Ok(parsed) => Ok(parsed),
            Err(_) => bail!("invalid MEMER_APPLICATION_ID environment variable"),
        },
        Err(_) => bail!("missing MEMER_APPLICATION_ID environment variable"),
    }
}

/// Generate an invite URL for the bot.
#[tracing::instrument(skip_all)]
pub async fn invite_url<H>(http: H, ready: &Ready)
where
    H: AsRef<Http> + Send + Sync,
{
    match ready
        .user
        .invite_url_with_oauth2_scopes(
            http,
            Permissions::ADD_REACTIONS | Permissions::SEND_MESSAGES,
            &[OAuth2Scope::Bot, OAuth2Scope::ApplicationsCommands],
        )
        .await
    {
        Ok(url) => info!(invite_url = %url),
        Err(_) => warn!("could not generate a bot invite URL"),
    }
}

/// Set the bot's activity.
#[tracing::instrument(skip_all)]
pub async fn set_activity(ctx: &Context) {
    let activity = match (
        env::var("MEMER_ACTIVITY_TYPE"),
        env::var("MEMER_ACTIVITY_NAME"),
    ) {
        (Ok(type_), Ok(name)) => {
            let type_str = type_.as_str();
            let activity_type = match type_str {
                "competing" | "listening" | "playing" | "streaming" | "watching" => type_str,
                _ => {
                    tracing::warn!("invalid MEMER_ACTIVITY_TYPE environment variable");
                    ""
                }
            };

            match activity_type {
                "competing" => Some(Activity::competing(name)),
                "listening" => Some(Activity::listening(name)),
                "playing" => Some(Activity::playing(name)),
                "streaming" => {
                    if let Ok(streaming) = env::var("MEMER_ACTIVITY_STREAMING") {
                        Some(Activity::streaming(name, streaming))
                    } else {
                        tracing::warn!("missing MEMER_ACTIVITY_STREAMING environment variable");
                        None
                    }
                }
                "watching" => Some(Activity::watching(name)),
                _ => None,
            }
        }
        _ => None,
    };

    if let Some(activity) = activity {
        ctx.set_activity(activity).await;
    }
}

/// Load subreddits from `subs.json`.
pub fn subs_from_file() -> Result<SubMap> {
    let path = env::current_dir()
        .context("failed to get cwd")?
        .join("subs.json");
    let contents = fs::read_to_string(path)
        .with_context(|| &format!("failed to read file: {}", path.display()))?;
    let subs = serde_json::from_str::<SubMap>(&contents)
        .with_context(|| format!("failed to deserialize file: {}", path.display()))?;

    Ok(subs)
}

/// Register application commands on all servers.
#[tracing::instrument(skip_all)]
pub async fn register_commands<I>(
    ctx: &'static Context,
    framework: &'static Framework<Data, Error>,
    guild_ids: I,
) where
    I: Iterator<Item = &'static GuildId>,
{
    let timer = Instant::now();

    future::join_all(guild_ids.map(|guild_id| {
        guild_id
            .set_application_commands(ctx, |commands| {
                *commands = create_application_commands(&framework.options().commands);
                commands
            })
            .inspect(|res| {
                if res.is_err() {
                    error!("failed to set applications for guild: {}", guild_id.clone())
                }
            })
    }))
    .await;

    info!("done in {}", humantime::Duration::from(timer.elapsed()));
}

/// Get the top 100 hot posts for all subreddits.
#[tracing::instrument(skip_all)]
pub async fn populate_posts(sub_map: &SubMap) -> Arc<DashMap<String, Vec<QuickPost>>> {
    info!("populating subreddit post data...");
    info!(?sub_map);
    let timer = Instant::now();

    let view = sub_map.clone().into_read_only();
    let subs = view.values().flatten().collect::<Vec<_>>();
    let posts = Arc::new(DashMap::with_capacity(subs.len()));

    future::join_all(
        subs.into_iter()
            .map(|sub| get_hot_posts(sub, Arc::clone(&posts))),
    )
    .await;

    info!("done in {}", humantime::Duration::from(timer.elapsed()));
    posts
}

/// Retrieve the top 100 hot reddit posts for the specified subreddit, and store them as
/// `QuickPost`s in a map.
#[tracing::instrument(skip_all, fields(subreddit = %sub_name))]
async fn get_hot_posts(sub_name: &str, posts: Arc<DashMap<String, Vec<QuickPost>>>) {
    let sub = Subreddit::new(sub_name);
    let hot = match sub.hot(100, None).await {
        Ok(hot) => hot,
        Err(_) => {
            error!("failed to get hot posts for {sub_name}");
            return;
        }
    };

    posts.insert(sub_name.to_string(), data::submissions_to_quickposts(&hot));
}
