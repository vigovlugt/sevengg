pub mod emote;
pub mod generated;
pub mod sevengg;

use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
};

use emote::Emote;
use generated::prisma::PrismaClient;
use serenity::{
    model::{
        application::interaction::InteractionResponseType,
        prelude::{
            command::CommandOptionType,
            interaction::{application_command::CommandDataOptionValue, Interaction},
            GuildId, Message, Ready,
        },
    },
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};
use uwuifier::uwuify_str_sse;

use crate::sevengg::{get_emotes, get_emotes_by_id, get_emotes_by_name};

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

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        let _ = guild_id
            .create_application_command(&ctx.http, |command| {
                command
                    .name("addemote")
                    .description("Add an emote to the bot by name or id")
                    .create_option(|option| {
                        option
                            .name("id")
                            .description("The id of the emote")
                            .kind(CommandOptionType::String)
                    })
                    .create_option(|option| {
                        option
                            .name("name")
                            .description("The name of the emote")
                            .kind(CommandOptionType::String)
                    })
            })
            .await
            .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "addemote" => {
                    if command.data.options.len() != 1 {
                        let _ = command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| {
                                        message.content("Please provide either an emote id or name")
                                    })
                            })
                            .await;
                        return;
                    }

                    let option = &command.data.options[0];
                    let name = option.name.as_str();

                    let values = if let Some(value) = &option.resolved {
                        if let CommandDataOptionValue::String(string) = value {
                            string
                                .to_string()
                                .split(",")
                                .map(|s| s.to_owned())
                                .collect()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };

                    let emotes = match name {
                        "id" => get_emotes_by_id(&self.client, &values).await,
                        "name" => get_emotes_by_name(&self.client, &values).await,
                        _ => {
                            return;
                        }
                    };

                    match emotes {
                        Ok(emotes) => {
                            for (name, emote) in emotes {
                                self.emote_map
                                    .write()
                                    .unwrap()
                                    .insert(emote.name.clone(), emote);
                                println!(
                                    "Emote: {} {:?}",
                                    name,
                                    self.emote_map.read().unwrap().get(&name)
                                );
                            }
                            println!("Emotes: {:?}", self.emote_map.read().unwrap().len());

                            let _ = command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            message.content("Added emote(s)!")
                                        })
                                })
                                .await;
                        }
                        Err(e) => {
                            println!("Error while getting emotes: {:?}", e);
                            let _ = command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            message.content("Failed to get emotes")
                                        })
                                })
                                .await;
                        }
                    }
                }
                _ => println!("Unknown command: {:?}", command.data.name),
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
