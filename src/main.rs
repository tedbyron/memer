#![warn(clippy::all, clippy::nursery, rust_2018_idioms)]
#![doc = include_str!("../README.md")]

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Error, Result};
use chrono::offset::Utc;
use dashmap::DashMap;
use poise::builtins::create_application_commands;
use poise::serenity_prelude::*;
use poise::{Framework, FrameworkOptions};
use tracing::{error, info, info_span, trace};
use tracing_subscriber::EnvFilter;

mod commands;
mod common;
mod data;
mod db;
mod error;
mod utils;

pub use common::Respond;
use data::Data;
pub use error::TraceErr;

pub type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    #[cfg(feature = "dotenv")]
    dotenv::dotenv()?;

    if env::var("MEMER_LOG").is_err() {
        env::set_var("MEMER_LOG", "INFO");
    }

    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::from_env("MEMER_LOG"))
        .init();
    trace!(command = %env::args().collect::<Vec<_>>().join(" "));

    // Framework values
    let token = utils::token()?;
    let app_id = utils::app_id()?;

    // User data values
    let (mongo, db) = db::client().await?;
    let subs = data::subs_from_file()?;
    let cache_time = Utc::now();
    let posts = data::all_posts(&subs).await;
    let blacklist_time = Utc::now();

    // Framework options
    let options: FrameworkOptions<Data, Error> = FrameworkOptions {
        #[rustfmt::skip]
        commands: vec![
            commands::admin::ping(),
            commands::admin::register(),
        ],
        ..FrameworkOptions::default()
    };

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
                let span = info_span!("setup");
                let span_guard = span.enter();

                utils::invite_url(ctx, ready).await;
                utils::set_activity(ctx).await;

                info!("registering application commands on all servers...");

                for guild_id in guild_ids {
                    guild_id
                        .set_application_commands(ctx, |commands| {
                            *commands = create_application_commands(&framework.options().commands);
                            commands
                        })
                        .await
                        .or_trace();
                }

                info!("finished registering application commands");
                drop(span_guard);

                Ok(Data {
                    bot_name,
                    bot_tag,

                    mongo,
                    db,

                    cache_time,
                    blacklist_time,

                    subs,
                    nsfw,
                    posts,
                    blacklist,
                    last_post,

                    request_count,
                    req_timer,
                    queue_state,
                })
            })
        })
        .options(options)
        .build()
        .await?;

    let shard_mgr = Arc::clone(&framework.shard_manager());
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .context("failed to install ctrl-c handler")
            .or_trace();
        shard_mgr.lock().await.shutdown_all().await;
    });

    info!("ready");
    framework.start_autosharded().await?;

    Ok(())
}
