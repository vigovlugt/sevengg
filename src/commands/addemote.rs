use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use futures::{future::BoxFuture, FutureExt};
use reqwest::Client;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        command::CommandOptionType,
        interaction::{
            application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
            InteractionResponseType,
        },
    },
    prelude::Context,
};

use crate::{
    db::save_emotes,
    emote::Emote,
    generated::prisma::PrismaClient,
    sevengg::{get_emotes_by_channels, get_emotes_by_id, get_emotes_by_name},
};

pub fn creator(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
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
    // Disabled because of load:
    // .create_option(|option| {
    //     option
    //         .name("channel_id")
    //         .description("The id of the channel to import")
    //         .kind(CommandOptionType::String)
    // })
}

pub fn handler<'a>(
    client: &'a Client,
    emote_map: &'a Arc<RwLock<HashMap<String, Emote>>>,
    db: &'a PrismaClient,
    ctx: &'a Context,
    command: &'a ApplicationCommandInteraction,
) -> BoxFuture<'a, ()> {
    async move {
        if command.data.options.len() != 1 {
            let _ = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content("Please provide either an emote id or name or channel id")
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
                    .map(|s| s.trim().to_owned())
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let emotes = match name {
            "id" => get_emotes_by_id(client, &values).await,
            "name" => get_emotes_by_name(client, &values).await,
            "channel_id" => get_emotes_by_channels(client, &values).await,
            _ => {
                return;
            }
        };

        match emotes {
            Ok(emotes) => {
                for (_name, emote) in emotes.iter() {
                    emote_map
                        .write()
                        .unwrap()
                        .insert(emote.name.clone(), emote.clone());
                }

                let _ = save_emotes(
                    &db,
                    &command.guild_id.unwrap().to_string(),
                    &emotes.values().into_iter().collect(),
                )
                .await;
                println!("Emotes: {:?}", emote_map.read().unwrap().len());

                let msg_content = {
                    let mut str = "Added emotes:\n".to_owned();
                    for (_name, emote) in emotes.iter() {
                        str.push_str(&format!(" - {}\n", emote.name));
                    }
                    str
                };

                let _ = command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| message.content(msg_content))
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
    .boxed()
}
