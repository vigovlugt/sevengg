use std::{collections::HashMap, env};

use serde::{Deserialize, Serialize};
use serenity::{
    model::prelude::{Message, Ready},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};
use uwuifier::uwuify_str_sse;

const SEVENTV_URL: &str = "https://7tv.io/v3/gql";

const PAGES: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Emote {
    id: String,
    name: String,
    host: EmoteHost,
    animated: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EmoteHost {
    files: Vec<EmoteImage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EmoteImage {
    format: String,
}

fn create_query(pages: u32, category: &str, page_offset: u32) -> String {
    let mut query = String::new();
    query.push_str("query {");
    for page in (1 + page_offset)..=(pages + page_offset) {
        query.push_str(&format!(
            r#"
            page{}: emotes(query: "", page: {}, limit: 300, filter: {{category: {}, exact_match: false, case_sensitive: false, ignore_tags: false}}) {{
                items {{
                    animated
                    id
                    name
                    host {{
                        files {{
                            format
                        }}
                    }}
                }}
            }}
            "#,
            page, page, category
        ));
    }
    query.push_str("}");
    query
}

async fn get_emotes() -> color_eyre::Result<HashMap<String, Emote>> {
    let (trending, top1, top2, top3, top4, top5) = tokio::join!(
        get_category_emotes("TRENDING_DAY", 1, 0),
        get_category_emotes("TOP", PAGES, 0),
        get_category_emotes("TOP", PAGES, 1),
        get_category_emotes("TOP", PAGES, 2),
        get_category_emotes("TOP", PAGES, 3),
        get_category_emotes("TOP", PAGES, 4),
    );
    let top = [
        top1?.as_slice(),
        top2?.as_slice(),
        top3?.as_slice(),
        top4?.as_slice(),
        top5?.as_slice(),
    ]
    .concat();
    let trending = trending?;

    let mut map = HashMap::new();

    for emote in trending {
        if !map.contains_key(&emote.name) {
            map.insert(emote.name.clone(), emote);
        }
    }

    for emote in top {
        if !map.contains_key(&emote.name) {
            map.insert(emote.name.clone(), emote);
        }
    }

    Ok(map)
}

async fn get_category_emotes(
    category: &str,
    pages: u32,
    page_offset: u32,
) -> color_eyre::Result<Vec<Emote>> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "query": create_query(pages, category, page_offset),
    });

    let res = client
        .post(SEVENTV_URL)
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let mut emotes = vec![];

    for page in (1 + page_offset)..=(pages + page_offset) {
        let page_emotes = serde_json::from_value::<Vec<Emote>>(
            res["data"][&format!("page{}", page)]["items"].to_owned(),
        )?;
        emotes.extend(page_emotes);
    }

    Ok(emotes)
}

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
            {
                let percentage = 1.0 / 100.0;
                let random_float = rand::random::<f64>();

                if random_float < percentage {
                    let uwu_message = uwuify_str_sse(&content);
                    let _ = tokio::join!(
                        msg.delete(&ctx.http),
                        msg.channel_id
                            .say(&ctx.http, format!("**{}**", msg.author.name)),
                        msg.channel_id.say(&ctx.http, uwu_message)
                    );
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
