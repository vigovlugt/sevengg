use std::collections::HashMap;

use reqwest::Client;
use serde_json::Value;

use crate::{emote::Emote, generated::prisma::PrismaClient};

const SEVENTV_URL: &str = "https://7tv.io/v3/gql";

const PAGES: u32 = 1;

pub fn create_category_query(pages: u32, category: &str, page_offset: u32) -> String {
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

async fn seventv_request(client: &reqwest::Client, query: &str) -> color_eyre::Result<Value> {
    let body = serde_json::json!({
        "query": query,
    });

    let res = client.post(SEVENTV_URL).json(&body).send().await?;
    println!("{}", res.status());
    if res.status().is_server_error() || res.status().is_client_error() {
        return Err(color_eyre::eyre::eyre!(
            "Error while sending request to 7TV: {} - {}",
            res.status(),
            res.text().await?
        ));
    }

    let json = res.json::<serde_json::Value>().await?;

    if json["errors"].is_array() {
        return Err(color_eyre::eyre::eyre!(
            "Error while sending request to 7TV: {}",
            json["errors"]
        ));
    }

    Ok(json)
}

pub async fn get_emotes(
    client: &reqwest::Client,
    db: &PrismaClient,
) -> color_eyre::Result<HashMap<String, Emote>> {
    let (top1, top2, top3, top4, top5) = tokio::join!(
        get_category_emotes(client, "TOP", PAGES, 0),
        get_category_emotes(client, "TOP", PAGES, 1),
        get_category_emotes(client, "TOP", PAGES, 2),
        get_category_emotes(client, "TOP", PAGES, 3),
        get_category_emotes(client, "TOP", PAGES, 4),
    );
    let top = [
        top1?.as_slice(),
        top2?.as_slice(),
        top3?.as_slice(),
        top4?.as_slice(),
        top5?.as_slice(),
    ]
    .concat();

    let db_emotes = db.emote().find_many(vec![]).exec().await?;

    let ids: Vec<String> = db_emotes.iter().map(|e| e.id.clone()).collect();
    let db_emotes = get_emotes_by_id(client, &ids).await?;

    let mut map = HashMap::new();

    for emote in top {
        if !map.contains_key(&emote.name) {
            map.insert(emote.name.clone(), emote);
        }
    }

    for (_, emote) in db_emotes {
        map.insert(emote.name.clone(), emote);
    }

    Ok(map)
}

pub async fn get_category_emotes(
    client: &reqwest::Client,
    category: &str,
    pages: u32,
    page_offset: u32,
) -> color_eyre::Result<Vec<Emote>> {
    let query = create_category_query(pages, category, page_offset);
    let res = seventv_request(client, &query).await?;

    let mut emotes = vec![];
    for page in (1 + page_offset)..=(pages + page_offset) {
        let page_emotes = serde_json::from_value::<Vec<Emote>>(
            res["data"][&format!("page{}", page)]["items"].to_owned(),
        )?;
        emotes.extend(page_emotes);
    }

    Ok(emotes)
}

pub async fn get_emotes_by_name(
    client: &reqwest::Client,
    names: &Vec<String>,
) -> color_eyre::Result<HashMap<String, Emote>> {
    let query = {
        let mut query = String::new();
        query.push_str("query {");
        for name in names.iter() {
            query.push_str(&format!(
                r#"
                emote_{}: emotes(query: "{}", page: 1, limit: 300, filter: {{category: TOP, exact_match: true, case_sensitive: true, ignore_tags: false}}) {{
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
                name, name
            ));
        }
        query.push_str("}");
        query
    };

    let res = seventv_request(&client, &query).await?;

    let mut emotes = HashMap::new();
    for name in names {
        let emote_list = serde_json::from_value::<Vec<Emote>>(
            res["data"][&format!("emote_{}", name)]["items"].to_owned(),
        )?;
        let emote = emote_list
            .iter()
            .find(|e| &e.name == name)
            .ok_or(color_eyre::eyre::eyre!("Emote not found"))?;
        emotes.insert(emote.name.clone(), emote.clone());
    }

    Ok(emotes)
}

pub async fn get_emotes_by_id(
    client: &Client,
    ids: &Vec<String>,
) -> color_eyre::Result<HashMap<String, Emote>> {
    let query = {
        let mut query = String::new();
        query.push_str("query {");
        for id in ids.iter() {
            query.push_str(&format!(
                r#"
                emote_{}: emote(id: "{}") {{
                    animated
                    id
                    name
                    host {{
                        files {{
                            format
                        }}
                    }}
                }}
                "#,
                id, id
            ));
        }
        query.push_str("}");
        query
    };

    let res = seventv_request(&client, &query).await?;

    let mut emotes = HashMap::new();
    for id in ids {
        println!("{:#?}", res);
        let emote =
            serde_json::from_value::<Emote>(res["data"][&format!("emote_{}", id)].to_owned())?;
        emotes.insert(emote.id.clone(), emote);
    }

    Ok(emotes)
}
