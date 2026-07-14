pub mod serde {
    use serde::Deserialize;
    use std::str::FromStr;

    /// Helper: deserialize a string, then parse it with FromStr.
    pub fn from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: FromStr,
        T::Err: std::fmt::Display, // required for serde::de::Error::custom
    {
        let s = String::deserialize(deserializer)?;
        T::from_str(&s).map_err(serde::de::Error::custom)
    }
}
