//! Admin commands

use anyhow::Result;
use poise::builtins::register_application_commands;

use crate::Context;

/// Replies with "Pong!". Only usable by bot owners.
#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn ping(ctx: Context<'_>) -> Result<()> {
    ctx.say("Pong!").await?;

    Ok(())
}

/// Register application commands in a server, or globally (if a bot owner).
#[poise::command(prefix_command, hide_in_help)]
pub async fn register(ctx: Context<'_>, #[flag] all: bool) -> Result<()> {
    register_application_commands(ctx, all).await?;

    Ok(())
}
