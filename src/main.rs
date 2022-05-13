#![warn(clippy::all, clippy::nursery, rust_2018_idioms)]
#![doc = include_str!("../README.md")]

use std::sync::Arc;
use std::{env, process};

use anyhow::{Context as _, Error, Result};
use poise::builtins::create_application_commands;
use poise::serenity_prelude::*;
use poise::{Framework, FrameworkOptions};
use tracing::{debug, error, info, trace};
use tracing_subscriber::EnvFilter;

mod commands;
mod common;
mod error;
mod utils;

pub use common::Respond;
pub use error::TraceErr;

pub struct Data {
    pub bot_name: String,
    pub bot_tag: String,
}

pub type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    process::exit(match run().await {
        Ok(_) => 0,
        Err(error) => {
            error!(%error);
            1
        }
    });
}

async fn run() -> Result<()> {
    #[cfg(feature = "dotenv")]
    dotenv::dotenv()?;

    if env::var("MEMER_LOG").is_err() {
        env::set_var("MEMER_LOG", "INFO");
    }

    // Setup tracing subscriber
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::from_env("MEMER_LOG"))
        .init();
    trace!(command = %env::args().collect::<Vec<_>>().join(" "));

    // Get and validate bot token and app ID
    let (token, app_id) = utils::token_app_id()?;

    // Command options
    let options: FrameworkOptions<Data, Error> = FrameworkOptions {
        #[rustfmt::skip]
        commands: vec![
            commands::admin::ping(),
            commands::admin::register(),
        ],
        ..FrameworkOptions::default()
    };

    // Build framework
    let framework = Framework::build()
        .token(token)
        .client_settings(move |client| client.application_id(app_id))
        .intents(GatewayIntents::GUILD_MESSAGES)
        .user_data_setup(|ctx, ready, framework| {
            let Ready { user, guilds, .. } = ready;

            let guild_ids = guilds.iter().map(|g| g.id);

            info!(guilds = ?guild_ids.clone().map(|id| id.0).collect::<Vec<_>>());
            info!("logged in as {}", user.tag());

            let bot_name = user.name.to_string();
            let bot_tag = user.tag();

            Box::pin(async move {
                utils::invite_url(ctx, ready).await;
                utils::set_activity(ctx).await;

                debug!("setting application commands on all servers...");
                for guild_id in guild_ids {
                    guild_id
                        .set_application_commands(ctx, |commands| {
                            *commands = create_application_commands(&framework.options().commands);
                            commands
                        })
                        .await
                        .or_trace();
                }

                Ok(Data { bot_name, bot_tag })
            })
        })
        .options(options)
        .build()
        .await?;

    let shard_mgr = Arc::clone(&framework.shard_manager());

    // Ctrl+c handler to shutdown all shards
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .context("failed to install ctrl-c handler")
            .or_trace();
        shard_mgr.lock().await.shutdown_all().await;
    });

    framework.start_autosharded().await?;

    Ok(())
}
