#![warn(clippy::all, clippy::nursery, rust_2018_idioms)]
#![doc = include_str!("../README.md")]

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Error, Result};
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
mod serde;
mod setup;

pub use data::Data;

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

    let token = setup::token()?;
    let app_id = setup::app_id()?;

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
                    let timer = Instant::now();
                    let Ready { user, guilds, .. } = ready;

                    info!("logged in as {} on {} servers", user.tag(), guilds.len());

                    let (mongo, db) = db::client_and_db().await?;
                    let subs = setup::subs_from_file()?;
                    let channels = db::channels(&db).await?;

                    let cache_time = Utc::now();
                    let posts = setup::populate_posts(&subs).await;

                    setup::invite_url(ctx, ready).await;
                    setup::set_activity(ctx).await;
                    setup::register_commands(ctx, framework, guilds).await;

                    let blacklist_time = Utc::now();

                    info!("done in {}", humantime::Duration::from(timer.elapsed()));

                    Ok(Data {
                        bot_id: user.id.0,
                        bot_name: user.name.to_string(),
                        bot_tag: user.tag(),

                        mongo,
                        db,

                        cache_time,
                        blacklist_time,

                        channels,
                        subs,
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
        match tokio::signal::ctrl_c().await {
            Ok(_) => shard_mgr.lock().await.shutdown_all().await,
            Err(e) => error!("failed to listen for ctrl-c signal: {e}"),
        }
    });

    info!("ready");
    framework.start_autosharded().await?;

    Ok(())
}
