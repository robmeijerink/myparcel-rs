use serde::de::{Deserializer, Error};
use serde::{Deserialize, Serialize, Serializer};

pub(crate) fn int_bool<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(D::Error::custom(format!("expected 0 or 1, got {other}"))),
    }
}

pub(crate) fn ser_opt_int_bool<S: Serializer>(
    value: &Option<bool>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value.map(u8::from).serialize(serializer)
}

pub(crate) fn opt_string_or_int<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Text(String),
        Number(i64),
    }
    Ok(
        Option::<Either>::deserialize(deserializer)?.map(|value| match value {
            Either::Text(text) => text,
            Either::Number(number) => number.to_string(),
        }),
    )
}

pub(crate) fn opt_amount<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Number(u64),
        Text(String),
    }
    match Option::<Either>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Either::Number(number)) => Ok(Some(number)),
        Some(Either::Text(text)) => text
            .parse()
            .map(Some)
            .map_err(|_| D::Error::custom(format!("`{text}` is not an amount"))),
    }
}
