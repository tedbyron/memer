//! Utility functions

use std::env;

use anyhow::{bail, Result};
use poise::serenity_prelude::*;
use tracing::{info, warn};

/// Get and validate the bot token and app ID.
pub fn token_app_id() -> Result<(String, u64)> {
    let token = match env::var("MEMER_TOKEN") {
        Ok(token) => token,
        Err(_) => bail!("missing MEMER_TOKEN environment variable"),
    };

    if validate_token(&token).is_err() {
        bail!("invalid MEMER_TOKEN environment variable");
    }

    let app_id = match env::var("MEMER_APPLICATION_ID") {
        Ok(id) => match id.parse::<u64>() {
            Ok(parsed) => parsed,
            Err(_) => bail!("invalid MEMER_APPLICATION_ID environment variable"),
        },
        Err(_) => bail!("missing MEMER_APPLICATION_ID environment variable"),
    };

    Ok((token, app_id))
}

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
        warn!("could not generate a bot invite URL");
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
