use serenity::all::Message;
use serenity::builder::EditInteractionResponse;
use serenity::client::Context;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Reaction;
use serenity::model::prelude::ReactionType;
use serenity::builder::CreateMessage;
use anyhow::Result;

use crate::article_handler::SUMMARY_MARKER;

pub enum ResponseTarget {
    ReactionDm { reaction: Reaction },
    ReactionReply { reaction: Reaction },
    EphemeralInteraction { interaction: CommandInteraction },
    VisibleInteraction { interaction: CommandInteraction },
}

pub async fn deliver_chat_summary(
    target: &ResponseTarget,
    ctx: &Context,
    summary_text: &str,
) -> Result<()> {
    match target {
        ResponseTarget::ReactionDm { reaction } => {
            let dm = CreateMessage::new().content(summary_text);
            if let Some(user_id) = reaction.user_id {
                let user = user_id.to_user(&ctx.http).await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch user: {}", e))?;
                user.direct_message(&ctx.http, dm).await
                    .map_err(|e| anyhow::anyhow!("Failed to send DM: {}", e))?;
            } else {
                return Err(anyhow::anyhow!("No user_id on reaction"));
            }
        }
        ResponseTarget::EphemeralInteraction { interaction } => {
            let response = EditInteractionResponse::new().content(summary_text);
            interaction.edit_response(&ctx.http, response).await
                .map_err(|e| anyhow::anyhow!("Failed to edit interaction response: {}", e))?;
        }
        _ => return Err(anyhow::anyhow!("Invalid response target for chat summary")),
    }
    Ok(())
}

pub async fn deliver_article_summary(
    target: &ResponseTarget,
    ctx: &Context,
    original_msg: Option<&Message>,
    summary_text: &str,
) -> Result<()> {
    match target {
        ResponseTarget::ReactionReply { reaction: _ } => {
            if let Some(msg) = original_msg {
                let reply = CreateMessage::new()
                    .content(summary_text)
                    .reference_message(msg);
                msg.channel_id.send_message(&ctx.http, reply).await
                    .map_err(|e| anyhow::anyhow!("Failed to send reply: {}", e))?;
                msg.channel_id.create_reaction(
                    &ctx.http, msg.id,
                    ReactionType::Unicode(SUMMARY_MARKER.to_string()),
                ).await
                    .map_err(|e| anyhow::anyhow!("Failed to add marker: {}", e))?;
            }
        }
        ResponseTarget::EphemeralInteraction { interaction } => {
            let response = EditInteractionResponse::new().content(summary_text);
            interaction.edit_response(&ctx.http, response).await
                .map_err(|e| anyhow::anyhow!("Failed to edit interaction response: {}", e))?;
        }
        ResponseTarget::VisibleInteraction { interaction } => {
            let response = EditInteractionResponse::new().content(summary_text);
            interaction.edit_response(&ctx.http, response).await
                .map_err(|e| anyhow::anyhow!("Failed to edit interaction response: {}", e))?;
            if let Some(msg) = original_msg {
                interaction.channel_id.create_reaction(
                    &ctx.http, msg.id,
                    ReactionType::Unicode(SUMMARY_MARKER.to_string()),
                ).await
                    .map_err(|e| anyhow::anyhow!("Failed to add marker: {}", e))?;
            }
        }
        _ => return Err(anyhow::anyhow!("Invalid response target for article summary")),
    }
    Ok(())
}

pub async fn send_error_response(
    target: &ResponseTarget,
    ctx: &Context,
    error_msg: &str,
) -> Result<()> {
    match target {
        ResponseTarget::EphemeralInteraction { interaction }
        | ResponseTarget::VisibleInteraction { interaction } => {
            let response = EditInteractionResponse::new()
                .content(format!("Something went wrong: {}", error_msg));
            interaction.edit_response(&ctx.http, response).await
                .map_err(|e| anyhow::anyhow!("Failed to send error response: {}", e))?;
        }
        _ => {}
    }
    Ok(())
}
