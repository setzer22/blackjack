// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Deserialize;

/// For fields that used to be a required f32 and now are optional
pub fn de_option_or_f32<'de, D>(de: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct OptionOrF32Visitor;
    impl<'de> serde::de::Visitor<'de> for OptionOrF32Visitor {
        type Value = Option<f32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an f32 or an Option<f32>")
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v as f32))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            f32::deserialize(deserializer).map(Some)
        }
    }

    de.deserialize_any(OptionOrF32Visitor)
}
