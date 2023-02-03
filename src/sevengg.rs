use std::collections::HashMap;

use crate::emote::Emote;

const SEVENTV_URL: &str = "https://7tv.io/v3/gql";

const PAGES: u32 = 1;

pub fn create_query(pages: u32, category: &str, page_offset: u32) -> String {
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

pub async fn get_emotes() -> color_eyre::Result<HashMap<String, Emote>> {
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

pub async fn get_category_emotes(
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
