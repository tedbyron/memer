#![warn(clippy::all, clippy::nursery, rust_2018_idioms)]
#![doc = include_str!("../README.md")]

use std::env;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Error, Result};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use governor::clock::{Clock, QuantaUpkeepClock};
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};

use poise::serenity_prelude::*;
use poise::{Framework, FrameworkOptions};
use tokio::sync::mpsc;
use tracing::{error, info, info_span, trace, Instrument};
use tracing_subscriber::EnvFilter;

mod commands;
mod data;
mod db;
mod result;
mod serde;
mod setup;

pub use data::Data;
pub use result::ResultExt;

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
    // TODO: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.intersperse
    trace!(command = %env::args().collect::<Vec<_>>().join(" "));

    let token = setup::token()?;
    let app_id = setup::app_id()?;
    let (tx, rx) = mpsc::unbounded_channel::<()>();
    let rx = Arc::new(Mutex::new(rx));
    let options: FrameworkOptions<Data, Error> = FrameworkOptions {
        commands: vec![commands::admin::ping(), commands::admin::register()],
        command_check: Some(|ctx| {
            let data = ctx.data();
            let gov = data.governor.clone();

            Box::pin(async move {
                // Check the rate limiter before every command is executed
                match gov.check_key(&ctx.channel_id()) {
                    Ok(_) => Ok(true),
                    Err(not_until) => {
                        ctx.say(format!(
                            "This channel is sending too many requests! Try again in {}",
                            humantime::format_duration(not_until.wait_time_from(data.clock.now())),
                        ))
                        .await
                        .or_trace();
                        Ok(false)
                    }
                }
            })
        }),
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
                    let bot_tag = user.tag();

                    info!("logged in as {} on {} servers", bot_tag, guilds.len());

                    setup::invite_url(ctx, ready).await;
                    setup::set_activity(ctx).await;
                    setup::register_commands(ctx, framework, guilds).await;

                    // Unwrap: any invalid value will cause an error to propagate instead of panic
                    data::SUBS.set(setup::subs_from_file()?).unwrap();
                    let (mongo, db) = db::client_and_db().await?;
                    let channels = db::all_channels(&db).await?;

                    let clock = QuantaUpkeepClock::from_interval(std::time::Duration::from_secs(1))
                        .map_err(|e| anyhow!("failed to create rate limiter clock: {e}"))?;
                    // TODO: `quanta::Error` does not implement `std::error::Error`
                    // https://github.com/metrics-rs/quanta/pull/68

                    let data = Data {
                        bot_id: user.id.0,
                        bot_name: user.name.clone(),
                        bot_tag,

                        mongo,
                        db,

                        cache_time: Utc::now() + Duration::hours(1),
                        blacklist_time: Utc::now() + Duration::hours(3),

                        posts: setup::all_hot_posts().await,

                        channels,
                        blacklist: Arc::new(DashMap::new()),
                        last_post: Arc::new(DashMap::new()),

                        governor: Arc::new(RateLimiter::new(
                            Quota::per_minute(
                                // Unwrap: 10_u32 is a valid NonZeroU32
                                10.try_into().unwrap(),
                            ),
                            DefaultKeyedStateStore::default(),
                            &clock,
                        )),
                        clock,
                    };

                    info!("done in {}", humantime::format_duration(timer.elapsed()));
                    Ok(data)
                }
                .instrument(info_span!("setup")),
            )
        })
        .build()
        .await?;

    let shard_mgr = framework.shard_manager();

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(_) => shard_mgr.lock().await.shutdown_all().await,
            Err(e) => error!("failed to listen for ctrl-c signal: {e}"),
        }
    });

    info!("ready");
    framework.start_autosharded().await?;

    drop(tx);
    let _ = rx.lock().await.recv().await;

    Ok(())
}
