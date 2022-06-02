// //! Common traits.

// use std::fmt::Display;

// use anyhow::Result;
// use poise::async_trait;
// use poise::serenity_prelude::*;

// #[async_trait]
// pub trait Respond {
//     async fn respond<T>(&self, ctx: &Context, content: T) -> Result<()>
//     where
//         T: ToString + Send;
//     async fn respond_err<E>(&self, ctx: &Context, error: E) -> Result<()>
//     where
//         E: Display + Send + Sync;
// }

// #[async_trait]
// impl Respond for ApplicationCommandInteraction {
//     /// Respond to an interaction with a message.
//     #[tracing::instrument(skip_all)]
//     async fn respond<T>(&self, ctx: &Context, content: T) -> Result<()>
//     where
//         T: ToString + Send,
//     {
//         self.create_interaction_response(&ctx, |res| {
//             res.kind(InteractionResponseType::ChannelMessageWithSource)
//                 .interaction_response_data(|msg| msg.content(content))
//         })
//         .await
//         .map_err(Into::into)
//     }

//     /// Respond to an interaction with an error.
//     #[tracing::instrument(skip_all)]
//     async fn respond_err<E>(&self, ctx: &Context, error: E) -> Result<()>
//     where
//         E: Display + Send + Sync,
//     {
//         self.create_interaction_response(&ctx, |res| {
//             res.kind(InteractionResponseType::ChannelMessageWithSource)
//                 .interaction_response_data(|msg| {
//                     msg.content(format!("\u{1f615} oops, there was an error: {error}"))
//                 })
//         })
//         .await
//         .map_err(Into::into)
//     }
// }
