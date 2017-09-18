// Base64 encoding adapter for Serde
// From https://github.com/serde-rs/json/issues/360#issuecomment-330095360

extern crate serde;
extern crate base64;

use self::serde::{Serializer, de, Deserialize, Deserializer};

pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
{
    serializer.serialize_str(&base64::encode(bytes))
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de>
{
    let s = <&str>::deserialize(deserializer)?;
    base64::decode(s).map_err(de::Error::custom)
}
