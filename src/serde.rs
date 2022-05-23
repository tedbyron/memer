//! Serde support

/// Serde support for serenity's `ChannelId` type
pub mod channel_id {
    use poise::serenity_prelude::ChannelId;
    use serde::de::{self, Visitor};
    use serde::{Deserializer, Serializer};

    /// Serialize a `ChannelId`'s inner value (u64) into a string.
    #[allow(clippy::trivially_copy_pass_by_ref)] // Ref required by serde
    pub fn serialize<S: Serializer>(
        channel_id: &ChannelId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&channel_id.0.to_string())
    }

    /// Deserialize a string into a `ChannelId`.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ChannelId, D::Error> {
        deserializer.deserialize_str(ChannelIdVisitor)
    }

    struct ChannelIdVisitor;

    impl<'de> Visitor<'de> for ChannelIdVisitor {
        type Value = ChannelId;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v.parse::<u64>() {
                Ok(id) => Ok(ChannelId::from(id)),
                Err(_) => Err(E::custom(format!(
                    "channel ID cannot be parsed as a u64: {v}"
                ))),
            }
        }
    }
}
