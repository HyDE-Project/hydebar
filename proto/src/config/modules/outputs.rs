//! Which outputs the bar renders on.

use serde::{Deserialize, Deserializer, de::Error as _};

/// Output targeting configuration for module rendering.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum Outputs {
    /// Render on all outputs.
    #[default]
    All,
    /// Render on the currently focused output.
    Active,
    /// Render on the explicitly configured output list.
    #[serde(deserialize_with = "non_empty")]
    Targets(Vec<String>)
}

fn non_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>
{
    let values = <Vec<T>>::deserialize(deserializer)?;

    if values.is_empty() {
        Err(D::Error::custom("need non-empty"))
    } else {
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use serde::de::value::{Error as DeError, SeqDeserializer};

    use super::*;

    #[test]
    fn non_empty_rejects_empty_vectors() {
        let error: DeError = non_empty::<_, String>(SeqDeserializer::<_, DeError>::new(
            Vec::<String>::new().into_iter()
        ))
        .expect_err("empty list should fail");
        assert!(error.to_string().contains("non-empty"));
    }
}
