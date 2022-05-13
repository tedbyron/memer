//! Utility functions

use std::env;

use poise::serenity_prelude::*;
use tracing::{info, warn};

/// Generate an invite URL for the bot.
#[tracing::instrument(skip_all)]
pub async fn invite_url<H>(http: H, ready: &Ready)
where
    H: AsRef<Http> + Send + Sync,
{
    if let Ok(url) = ready
        .user
        .invite_url_with_oauth2_scopes(
            http,
            Permissions::ADD_REACTIONS | Permissions::SEND_MESSAGES,
            &[OAuth2Scope::Bot, OAuth2Scope::ApplicationsCommands],
        )
        .await
    {
        info!("{url}");
    } else {
        warn!("Could not generate a bot invite URL");
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
                    tracing::warn!("Invalid MEMER_ACTIVITY_TYPE environment variable");
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
                        tracing::warn!("Missing MEMER_ACTIVITY_STREAMING environment variable");
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
