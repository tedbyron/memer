#![warn(clippy::all, clippy::nursery, rust_2018_idioms)]
#![doc = include_str!("../README.md")]

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Error, Result};
use chrono::offset::Utc;
use poise::serenity_prelude::*;
use poise::{Framework, FrameworkOptions};
use tokio::time::Instant;
use tracing::{error, info, info_span, trace, Instrument};
use tracing_subscriber::EnvFilter;

mod commands;
mod common;
mod data;
mod db;
mod error;
mod setup;

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
    let token = setup::token()?;
    let app_id = setup::app_id()?;

    // User data values
    let (mongo, db) = db::client().await?;
    let subs = setup::subs_from_file()?;

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
            Box::pin(
                async move {
                    info!("starting...");
                    let setup_timer = Instant::now();

                    let Ready { user, guilds, .. } = ready;
                    let guild_ids = guilds.iter().map(|guild| &guild.id);

                    info!(guilds = ?guild_ids.clone().map(|id| id.0).collect::<Vec<_>>());
                    info!("logged in as {}", user.tag());

                    let bot_name = user.name.to_string();
                    let bot_tag = user.tag();

                    let cache_time = Utc::now();
                    let posts = setup::populate_posts(&subs).await;
                    let blacklist_time = Utc::now();

                    setup::invite_url(ctx, ready).await;
                    setup::set_activity(ctx).await;
                    setup::register_commands(ctx, framework, guild_ids).await;

                    info!(
                        "done in {}",
                        humantime::Duration::from(setup_timer.elapsed())
                    );

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
                }
                .instrument(info_span!("setup")),
            )
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
