// Copyright 2019 The Gotts Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use chrono::prelude::*;
use chrono_tz::Tz;
use failure::{err_msg, Error};
use serde::de::{self, Deserialize, Deserializer};
use std::fmt::Display;
use std::str::FromStr;

pub(crate) const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub(crate) const DATE_FORMAT: &str = "%Y-%m-%d";

pub(crate) fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

pub(crate) fn parse_date(value: &str, time_zone: Tz) -> Result<DateTime<Tz>, Error> {
    if value.contains(':') {
        let datetime = NaiveDateTime::parse_from_str(value, DATETIME_FORMAT)?;
        time_zone
            .from_local_datetime(&datetime)
            .single()
            .ok_or_else(|| err_msg("unable to parse datetime"))
    } else {
        let datetime = NaiveDate::parse_from_str(value, DATE_FORMAT).map(|d| d.and_hms(0, 0, 0))?;
        time_zone
            .from_local_datetime(&datetime)
            .single()
            .ok_or_else(|| err_msg("unable to parse date"))
    }
}
