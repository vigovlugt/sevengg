pub mod commands;
pub mod db;
pub mod emote;
pub mod generated;
pub mod sevengg;

use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
};

use commands::addemote;
use emote::Emote;
use generated::prisma::PrismaClient;
use serenity::{
    model::prelude::{interaction::Interaction, GuildId, Message, Ready},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};
use uwuifier::uwuify_str_sse;

use crate::sevengg::get_emotes;

struct DiscordHandler {
    emote_map: Arc<RwLock<HashMap<String, Emote>>>,
    client: reqwest::Client,
    db: PrismaClient,
}

#[async_trait::async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        let content = msg.content.clone();

        let emote = self.emote_map.read().unwrap().get(&content).cloned();

        if let Some(emote) = emote {
            let extension = if !emote.animated { "png" } else { "gif" };

            async fn send_msg(ctx: &Context, msg: &Message, emote: &Emote, extension: &str) {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let _ = msg
                    .channel_id
                    .say(
                        &ctx.http,
                        format!("https://cdn.7tv.app/emote/{}/2x.{}", emote.id, extension),
                    )
                    .await;
            }

            let _ = tokio::join!(
                msg.delete(&ctx.http),
                msg.channel_id
                    .say(&ctx.http, format!("**{}**", msg.author.name)),
                send_msg(&ctx, &msg, &emote, extension)
            );
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

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        let _ = guild_id
            .create_application_command(&ctx.http, addemote::creator)
            .await
            .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "addemote" => {
                    addemote::handler(&self.client, &self.emote_map, &self.db, &ctx, &command)
                        .await;
                }
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenv::dotenv().expect("Failed to load .env file");

    let client = reqwest::Client::new();
    let db = PrismaClient::_builder().build().await.unwrap();
    let emote_map = get_emotes(&client, &db).await?;
    println!("Emotes: {:?}", emote_map.len());

    let token = env::var("DISCORD_BOT_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let handler = DiscordHandler {
        emote_map: Arc::new(RwLock::new(emote_map)),
        client,
        db,
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;

    Ok(())
}
