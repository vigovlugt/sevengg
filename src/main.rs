pub mod emote;
pub mod sevengg;

use std::{collections::HashMap, env};

use emote::Emote;
use serenity::{
    model::prelude::{Message, Ready},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};
use uwuifier::uwuify_str_sse;

use crate::sevengg::get_emotes;

struct DiscordHandler {
    emote_map: HashMap<String, Emote>,
}

#[async_trait::async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        let content = msg.content.clone();

        let emote = self.emote_map.get(&content);

        if let Some(emote) = emote {
            let extension = if !emote.animated { "png" } else { "gif" };

            let _ = tokio::join!(
                msg.delete(&ctx.http),
                msg.channel_id
                    .say(&ctx.http, format!("**{}**", msg.author.name))
            );
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("https://cdn.7tv.app/emote/{}/2x.{}", emote.id, extension),
                )
                .await;
        } else {
            let uwu_message = uwuify_str_sse(&content);
            if !content.is_empty()
                && !content.starts_with("http://")
                && !content.starts_with("https://")
                && msg.attachments.is_empty()
                && msg.embeds.is_empty()
                && msg.activity.is_none()
                && msg.application.is_none()
                && msg.referenced_message.is_none()
                && msg.kind == serenity::model::channel::MessageType::Regular
                && !msg.author.bot
                && uwu_message != content
            {
                let percentage = 1.0 / 100.0;
                let random_float = rand::random::<f64>();

                let uwu_message = uwuify_str_sse(&(content + "."));

                if random_float < percentage {
                    let _ = tokio::join!(
                        msg.delete(&ctx.http),
                        msg.channel_id
                            .say(&ctx.http, format!("**{}**", msg.author.name))
                    );
                    let _ = msg.channel_id.say(&ctx.http, uwu_message).await;
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenv::dotenv().expect("Failed to load .env file");

    let emote_map = get_emotes().await?;
    println!("Emotes: {:?}", emote_map.len());

    let token = env::var("DISCORD_BOT_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let handler = DiscordHandler { emote_map };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;

    Ok(())
}
