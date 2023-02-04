use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Emote {
    pub id: String,
    pub name: String,
    pub host: EmoteHost,
    pub animated: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmoteHost {
    pub files: Vec<EmoteImage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmoteImage {
    pub format: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmoteSetEmote {
    pub data: Emote,
}
