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

//! Types and traits that should be provided by a wallet
//! implementation

use chrono::{DateTime, Utc};

use super::error::Error;
use crate::alphavantage::ExchangeRateResult;

/// Combined trait to allow dynamic oracle dispatch
pub trait OracleInst: OracleBackend + Send + Sync + 'static {}
impl<T> OracleInst for T where T: OracleBackend + Send + Sync + 'static {}

/// Oracles should implement this backend for their storage. All functions
/// here expect that the oracle instance has instantiated itself or stored
/// whatever credentials it needs
pub trait OracleBackend {
	/// Iterate over all local exchange rate data stored by the backend
	fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = ExchangeRateResult> + 'a>;

	/// Iterate over all local exchange rate data stored by the backend with same id
	fn iter_id<'a>(&'a self, id: &str) -> Box<dyn Iterator<Item = ExchangeRateResult> + 'a>;

	/// Get self owned output data by id
	fn get(&self, id: &str) -> Result<ExchangeRateResult, Error>;

	/// Create a new write batch to update or remove output data
	fn batch<'a>(&'a mut self) -> Result<Box<dyn ExchangePriceBatch + 'a>, Error>;
}

/// Batch trait to update the exchange price data backend atomically. Trying to use a
/// batch after commit MAY result in a panic. Due to this being a trait, the
/// commit method can't take ownership.
pub trait ExchangePriceBatch {
	/// Add or update data about an exchange rate to the backend
	fn save(&mut self, date: DateTime<Utc>, exchange_rate: ExchangeRateResult)
		-> Result<(), Error>;

	/// Gets exchange rate data by id
	fn get(&self, id: &str) -> Result<ExchangeRateResult, Error>;

	/// Iterate over all exchange rate data stored by the backend
	fn iter(&self) -> Box<dyn Iterator<Item = ExchangeRateResult>>;

	/// Iterate over all exchange rate data stored by the backend with same id
	fn iter_id<'a>(&'a self, id: &str) -> Box<dyn Iterator<Item = ExchangeRateResult> + 'a>;

	/// Delete data about an exchange rate from the backend
	fn delete(&mut self, id: &str, date: DateTime<Utc>) -> Result<(), Error>;

	/// Write the oracle data to backend file
	fn commit(&self) -> Result<(), Error>;
}
