#![warn(clippy::all, clippy::nursery, rust_2018_idioms)]
#![doc = include_str!("../README.md")]

use std::env;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Error, Result};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use governor::{Quota, RateLimiter};
use poise::serenity_prelude::*;
use poise::{Framework, FrameworkOptions};
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
        .options(options)
        .user_data_setup(|ctx, ready, framework| {
            Box::pin(
                async move {
                    info!("starting...");
                    let timer = Instant::now();
                    let Ready { user, guilds, .. } = ready;

                    info!("logged in as {} on {} servers", user.tag(), guilds.len());

                    setup::invite_url(ctx, ready).await;
                    setup::set_activity(ctx).await;
                    setup::register_commands(ctx, framework, guilds).await;

                    // Unwrap: any invalid value will cause an error to propagate
                    data::SUBS.set(setup::subs_from_file()?).unwrap();
                    let (mongo, db) = db::client_and_db().await?;
                    let channels = db::all_channels(&db).await?;

                    let posts = setup::all_hot_posts().await;
                    let cache_time = Utc::now() + Duration::hours(1);

                    let blacklist = Arc::new(DashMap::new());
                    let blacklist_time = Utc::now() + Duration::hours(3);

                    let last_post = Arc::new(DashMap::new());
                    let governor = Arc::new(RateLimiter::keyed(Quota::per_minute(
                        // Unwrap: 10_u32 is a valid NonZeroU32
                        10.try_into().unwrap(),
                    )));

                    let data = Data {
                        bot_id: user.id.0,
                        bot_name: user.name.to_string(),
                        bot_tag: user.tag(),

                        mongo,
                        db,

                        cache_time,
                        blacklist_time,

                        posts,

                        channels,
                        blacklist,
                        last_post,

                        governor,
                    };

                    info!("done in {}", humantime::format_duration(timer.elapsed()));
                    Ok(data)
                }
                .instrument(info_span!("setup")),
            )
        })
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
