use crate::{
    emote::Emote,
    generated::prisma::{guild_emotes, PrismaClient},
};

pub async fn save_emotes(
    db: &PrismaClient,
    guild_id: &str,
    emotes: &Vec<&Emote>,
) -> color_eyre::Result<()> {
    for emote in emotes {
        // Delete existing emote for guild
        let _ = db
            .guild_emotes()
            .delete_many(vec![
                guild_emotes::guild_id::equals(guild_id.to_owned()),
                guild_emotes::name::equals(emote.name.clone()),
            ])
            .exec()
            .await?;

        let _ = db
            .guild_emotes()
            .create(
                emote.name.clone(),
                guild_id.to_owned(),
                emote.id.clone(),
                vec![],
            )
            .exec()
            .await?;
    }

    Ok(())
}
