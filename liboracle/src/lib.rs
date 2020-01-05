// Copyright 2018 The Grin Developers
// Modifications Copyright 2019 The Gotts Developers
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

//! Concrete implementations of Gotts Oracle

#[macro_use]
extern crate log;
use failure;
extern crate failure_derive;

use gotts_oracle_alphavantage as alphavantage;

pub mod error;
pub mod lmdb;
pub mod oracle_ser;
pub mod oracle_store;
pub mod types;

pub use self::error::Error;
pub use self::lmdb::{oracle_db_exists, LMDBBackend};
pub use self::types::{OracleBackend, OracleInst};
