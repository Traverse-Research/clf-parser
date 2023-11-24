use std::{
    fmt::{self, Formatter},
    str::FromStr,
};

use serde::{
    de::{self, Visitor},
    Deserializer,
};

pub(crate) fn deserialize_space_separated<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    struct ArrayVisitor<T> {
        marker: std::marker::PhantomData<T>,
    }

    impl<T> ArrayVisitor<T> {
        fn new() -> Self {
            Self {
                marker: std::marker::PhantomData,
            }
        }
    }

    impl<'de, T> Visitor<'de> for ArrayVisitor<T>
    where
        T: FromStr,
        T::Err: fmt::Display,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("10 20 30 40")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            v.split_whitespace()
                .map(|s| s.parse::<T>().map_err(de::Error::custom))
                .collect()
        }
    }

    deserializer.deserialize_str(ArrayVisitor::<T>::new())
}
