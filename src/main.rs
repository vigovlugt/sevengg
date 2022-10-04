use std::{collections::HashMap, env};

use serde::{Deserialize, Serialize};
use serenity::{
    model::prelude::{Message, Ready},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};

const SEVENTV_URL: &str = "https://7tv.io/v3/gql";

const PAGES: u32 = 10;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Emote {
    id: String,
    name: String,
    images: Vec<EmoteImage>,
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
                    id
                    name
                    images {{
                        format
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
    let top = [top1?.as_slice(), top2?.as_slice(), top3?.as_slice(), top4?.as_slice(), top5?.as_slice()].concat();
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

        let emote = match emote {
            Some(e) => e,
            None => return,
        };

        let is_png = emote.images.iter().any(|e| e.format == "PNG");

        let extension = if is_png { "png" } else { "gif" };

        let _ = msg.delete(&ctx.http).await;
        let _ = msg
            .channel_id
            .say(&ctx.http, format!("**{}**", msg.author.name))
            .await;
        let _ = msg
            .channel_id
            .say(
                &ctx.http,
                format!("https://cdn.7tv.app/emote/{}/2x.{}", emote.id, extension),
            )
            .await;
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
