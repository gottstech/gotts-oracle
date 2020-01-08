// Copyright 2019 The Grin Developers
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

//! Foreign API External Definition

use crate::handlers::version_api::{Version, VersionHandler};
use crate::rest::*;
use gotts_oracle_alphavantage::exchange_rate::{ExchangeRate, ExchangeRateResult};
use gotts_oracle_lib::OracleBackend;
use gotts_oracle_util::Mutex;

use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use std::sync::Weak;

/// Main interface into all node API functions.
/// Node APIs are split into two seperate blocks of functionality
/// called the ['Owner'](struct.Owner.html) and ['Foreign'](struct.Foreign.html) APIs
///
/// Methods in this API are intended to be 'single use'.
///

pub struct Foreign<T: ?Sized>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Oracle instance
	pub oracle: Arc<Mutex<T>>,
	/// Vendor API client
	pub client: Weak<gotts_oracle_alphavantage::Client>,
}

impl<T: ?Sized> Foreign<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Create a new API instance with the chain, transaction pool, peers and `sync_state`. All subsequent
	/// API calls will operate on this instance of node API.
	///
	/// # Arguments
	/// * `chain` - A non-owning reference of the chain.
	///
	/// # Returns
	/// * An instance of the Node holding references to the current chain, transaction pool, peers and sync_state.
	///

	pub fn new(oracle: Arc<Mutex<T>>, client: Weak<gotts_oracle_alphavantage::Client>) -> Self {
		Foreign { oracle, client }
	}

	/// Returns the oracle version and block header version (used by gotts node).
	///
	/// # Returns
	/// * Result Containing:
	/// * A [`Version`](handlers/version_api/struct.Version.html)
	/// * or [`Error`](struct.Error.html) if an error is encountered.
	///

	pub fn get_version(&self) -> Result<Version, Error> {
		let version_handler = VersionHandler {};
		version_handler.get_version()
	}

	/// Returns the Exchange Rate.
	///
	/// # Arguments
	/// * `from` - exchange rate from
	/// * `to` - exchange rate to.
	///
	/// # Returns
	/// * Result Containing:
	/// * A [`ExchangeRateResult`](types/struct.ExchangeRateResult.html)
	/// * or [`Error`](struct.Error.html) if an error is encountered.
	///
	pub fn get_rate(&self, from: String, to: String) -> Result<ExchangeRateResult, Error> {
		let arc_client = w(&self.client)?;

		let exchange_rate = crossbeam::scope(|scope| {
			let handle = scope.spawn(move |_| -> Result<ExchangeRate, Error> {
				let exchange_result = arc_client.get_exchange_rate(&from, &to);
				let result = match exchange_result {
					Ok(result) => Ok(result),
					Err(_e) => Err(ErrorKind::RequestError(
						"query alphavantage failed!".to_owned(),
					))?,
				};

				result
			});

			handle.join().unwrap()
		});

		let result: ExchangeRate = exchange_rate.unwrap().unwrap();
		let exchange_rate_result = ExchangeRateResult {
			from: result.from.code.to_string(),
			to: result.to.code.to_string(),
			rate: result.rate,
			date: result.date,
		};

		// save the query data into local database for aggregation
		{
			let mut oracle = self.oracle.lock();
			let mut batch = oracle.batch()?;
			batch.save(result.date, exchange_rate_result.clone())?;
			batch.commit()?;
		}

		Ok(exchange_rate_result)
	}

	/// Returns the recent Exchange Rate.
	///
	/// # Arguments
	/// * `prefix` - exchange rate prefix, for example "USD", "USD2CNY", etc.
	/// * `items` - how many items need to be returned.
	///
	/// # Returns
	/// * Result Containing:
	/// * A [`Vec<ExchangeRateResult>`](types/struct.ExchangeRateResult.html)
	/// * or [`Error`](struct.Error.html) if an error is encountered.
	///
	pub fn get_recent(
		&self,
		prefix: String,
		items: usize,
	) -> Result<Vec<ExchangeRateResult>, Error> {
		let oracle = self.oracle.lock();
		let mut rates: Vec<ExchangeRateResult> = if prefix.is_empty() {
			oracle.iter().collect()
		} else {
			oracle.iter_id(&prefix).collect()
		};
		rates.sort_by_key(|rate| std::cmp::Reverse(rate.date.clone()));
		rates.truncate(items);

		Ok(rates)
	}

	/// Compact the exchange rate data, clean the data beyond x minutes.
	///
	/// # Arguments
	/// * `minutes` - compact data which beyond specified minutes.
	///
	/// # Returns
	/// * Result Containing:
	/// * An usize of how many items has been cleaned.
	/// * or [`Error`](struct.Error.html) if an error is encountered.
	///
	pub fn compact(&self, minutes: u32) -> Result<usize, Error> {
		let cutoff_time: DateTime<Utc> = Utc::now() - Duration::minutes(minutes as i64);

		// read all rate data from local database
		let mut oracle = self.oracle.lock();
		let mut batch = oracle.batch()?;
		let mut total_cleaned = 0;
		for rate in batch.iter() {
			if rate.date < cutoff_time {
				total_cleaned += 1;
				let mut id = rate.from.clone();
				id.push('2');
				id.push_str(&rate.to);
				batch.delete(&id, rate.date)?;
			}
		}
		batch.commit()?;

		Ok(total_cleaned)
	}

	/// Get aggregated exchange rates.
	///
	/// # Arguments
	///
	/// # Returns
	/// * Result Containing:
	/// * A [`Vec<ExchangeRateResult>`](types/struct.ExchangeRateResult.html)
	/// * or [`Error`](struct.Error.html) if an error is encountered.
	///
	pub fn get_aggregated(&self) -> Result<Vec<ExchangeRateResult>, Error> {
		let oracle = self.oracle.lock();
		let mut rates: Vec<ExchangeRateResult> = oracle.iter().collect();
		rates.sort_by_key(|rate| std::cmp::Reverse(rate.date.clone()));

		let currencies_a = vec!["EUR", "GBP", "BTC", "ETH"];
		let currencies_b = vec!["CNY", "JPY", "CAD"];
		let mut aggregated: Vec<ExchangeRateResult> =
			Vec::with_capacity(currencies_a.len() + currencies_b.len());
		for from in currencies_a {
			let index = rates
				.iter()
				.position(|r| r.from == from && r.to == "USD")
				.ok_or(ErrorKind::NotFound)?;
			aggregated.push(rates[index].clone());
		}
		for to in currencies_b {
			let index = rates
				.iter()
				.position(|r| r.from == "USD" && r.to == to)
				.ok_or(ErrorKind::NotFound)?;
			aggregated.push(rates[index].clone());
		}

		Ok(aggregated)
	}
}
