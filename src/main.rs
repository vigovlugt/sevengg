use std::{collections::HashMap, env};

use serde::{Deserialize, Serialize};
use serenity::{prelude::{GatewayIntents, EventHandler, Context}, model::prelude::{Message, Ready}, Client};

const SEVENTV_URL: &str = "https://7tv.io/v3/gql";

const PAGES: u32 = 10;

#[derive(Serialize, Deserialize, Debug)]
struct Emote {
    id: String,
    name: String,
    images: Vec<EmoteImage>
}

#[derive(Serialize, Deserialize, Debug)]
struct EmoteImage {
    format: String,
}

fn create_query(pages: u32) -> String {
    let mut query = String::new();
    query.push_str("query {");
    for page in 1..=pages {
        query.push_str(&format!(
            r#"
            page{}: emotes(query: "", page: {}, limit: 300, filter: {{category: TOP, exact_match: false, case_sensitive: false, ignore_tags: false}}) {{
                items {{
                    id
                    name
                    images {{
                        format
                    }}
                }}
            }}
            "#,
            page, page
        ));
    }
    query.push_str("}");
    query
}

async fn get_emotes() -> color_eyre::Result<HashMap<String, Emote>> {
    let client = reqwest::Client::new();

    let body =
        serde_json::json!({
            "query": create_query(PAGES),
        });

    let res = client
        .post(SEVENTV_URL)
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let mut emotes = vec![];

    for page in 1..=PAGES {
        let page_emotes = serde_json::from_value::<Vec<Emote>>(res["data"][&format!("page{}", page)]["items"].to_owned())?;
        emotes.extend(page_emotes);
    }

    let mut map = HashMap::new();

    for emote in emotes {
        if !map.contains_key(&emote.name) {
            map.insert(emote.name.clone(), emote);
        }
    }

    Ok(map)
}

struct DiscordHandler {
    emote_map: HashMap<String, Emote>,
}

#[async_trait::async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: Context, msg: Message){
        let content = msg.content.clone();

        let emote = self.emote_map.get(&content);

        let emote = match emote {
            Some(e) => e,
            None => return,
        };

        let is_png = emote.images.iter().any(|e| e.format == "PNG");

        let extension = if is_png {
            "png"
        } else {
            "gif"
        };

        let _ = msg.delete(&ctx.http).await;
        let _ = msg.channel_id.say(&ctx.http, format!("**{}**", msg.author.name)).await;
        let _ = msg.channel_id.say(&ctx.http, format!("https://cdn.7tv.app/emote/{}/2x.{}", emote.id, extension)).await;
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
    let handler = DiscordHandler{
        emote_map
    };
    let mut client =
        Client::builder(&token, intents).event_handler(handler).await?;

    client.start().await?;

    Ok(())
}
