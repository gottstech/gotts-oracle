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

use crate::rest::*;
use crate::router::{Handler, ResponseFuture};
use crate::web::*;
use chrono::{DateTime, Duration, Utc};
use hyper::{Body, Request};
use std::sync::Arc;
use std::sync::Weak;

use alphavantage::exchange_rate::{ExchangeRate, ExchangeRateResult};
use gotts_oracle_alphavantage as alphavantage;
use gotts_oracle_lib::OracleBackend;
use gotts_oracle_util::Mutex;

/// Gets API index
/// GET /v1/
///
pub struct IndexHandler {
	pub list: Vec<String>,
}

impl IndexHandler {}

impl Handler for IndexHandler {
	fn get(&self, _req: Request<Body>) -> ResponseFuture {
		json_response_pretty(&self.list)
	}
}

/// Gets Exchange Rate
/// GET /v1/exchange?from=USD&to=CNY
///
pub struct ExchangeHandler<T: ?Sized>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Oracle instance
	pub oracle: Arc<Mutex<T>>,
	/// Vendor API client
	pub client: Weak<alphavantage::Client>,
}

impl<T: ?Sized> ExchangeHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	pub fn new(oracle: Arc<Mutex<T>>, client: Weak<alphavantage::Client>) -> ExchangeHandler<T> {
		ExchangeHandler { oracle, client }
	}

	fn get_rate(&self, req: Request<Body>) -> Result<ExchangeRateResult, Error> {
		let query = must_get_query!(req);
		let params = QueryParams::from(query);
		let from = parse_param_no_err!(params, "from", "USD".to_owned());
		let to = parse_param_no_err!(params, "to", "CNY".to_owned());
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
}

impl<T: ?Sized> Handler for ExchangeHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	fn get(&self, req: Request<Body>) -> ResponseFuture {
		result_to_response(self.get_rate(req))
	}
}

/// Gets recent exchange rates
/// GET /v1/recent?prefix=USD&items=16
/// GET /v1/recent?prefix=USD2CNY&items=16
///
pub struct RecentHandler<T: ?Sized>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Oracle instance
	pub oracle: Arc<Mutex<T>>,
}

impl<T: ?Sized> RecentHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	pub fn new(oracle: Arc<Mutex<T>>) -> RecentHandler<T> {
		RecentHandler { oracle }
	}

	fn get_recent(&self, req: Request<Body>) -> Result<Vec<ExchangeRateResult>, Error> {
		let query = must_get_query!(req);
		let params = QueryParams::from(query);
		let items = parse_param_no_err!(params, "items", "8".to_owned());
		let items: usize = items.parse().unwrap_or(8);
		let from = parse_param!(params, "prefix", "".to_string());

		// read the recent rate data from local database
		let oracle = self.oracle.lock();
		let mut rates: Vec<ExchangeRateResult> = if from.is_empty() {
			oracle.iter().collect()
		} else {
			oracle.iter_id(&from).collect()
		};
		rates.sort_by_key(|rate| std::cmp::Reverse(rate.date.clone()));
		rates.truncate(items);

		Ok(rates)
	}
}

impl<T: ?Sized> Handler for RecentHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	fn get(&self, req: Request<Body>) -> ResponseFuture {
		result_to_response(self.get_recent(req))
	}
}

/// Compact the exchange rate data, clean the data beyond x minutes.
/// GET /v1/compact?mins=10
///
pub struct CompactHandler<T: ?Sized>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Oracle instance
	pub oracle: Arc<Mutex<T>>,
}

impl<T: ?Sized> CompactHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	pub fn new(oracle: Arc<Mutex<T>>) -> CompactHandler<T> {
		CompactHandler { oracle }
	}

	fn compact(&self, req: Request<Body>) -> Result<usize, Error> {
		let query = must_get_query!(req);
		let params = QueryParams::from(query);
		let minutes = parse_param_no_err!(params, "mins", "10".to_owned());
		let minutes: u32 = minutes.parse().unwrap_or(10);

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
}

impl<T: ?Sized> Handler for CompactHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	fn get(&self, req: Request<Body>) -> ResponseFuture {
		result_to_response(self.compact(req))
	}
}

/// Gets aggregated exchange rates
/// GET /v1/aggregated
///
pub struct AggregateHandler<T: ?Sized>
where
	T: OracleBackend + Send + Sync + 'static,
{
	/// Oracle instance
	pub oracle: Arc<Mutex<T>>,
}

impl<T: ?Sized> AggregateHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	pub fn new(oracle: Arc<Mutex<T>>) -> AggregateHandler<T> {
		AggregateHandler { oracle }
	}

	fn get_aggregated(&self, _req: Request<Body>) -> Result<Vec<ExchangeRateResult>, Error> {
		let oracle = self.oracle.lock();
		let mut rates: Vec<ExchangeRateResult> = oracle.iter().collect();
		rates.sort_by_key(|rate| std::cmp::Reverse(rate.date.clone()));

		let currencies = vec!["USD", "EUR", "CNY", "JPY", "GBP", "CAD"];
		let mut aggregated: Vec<ExchangeRateResult> = Vec::new();
		for from in currencies.clone() {
			for to in currencies.clone() {
				if from != to {
					let index = rates
						.iter()
						.position(|r| r.from == from && r.to == to)
						.ok_or(ErrorKind::NotFound)?;
					aggregated.push(rates[index].clone());
				}
			}
		}

		let index = rates
			.iter()
			.position(|r| r.from == "BTC" && r.to == "USD")
			.ok_or(ErrorKind::NotFound)?;
		aggregated.push(rates[index].clone());
		let index = rates
			.iter()
			.position(|r| r.from == "ETH" && r.to == "USD")
			.ok_or(ErrorKind::NotFound)?;
		aggregated.push(rates[index].clone());

		Ok(aggregated)
	}
}

impl<T: ?Sized> Handler for AggregateHandler<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	fn get(&self, req: Request<Body>) -> ResponseFuture {
		result_to_response(self.get_aggregated(req))
	}
}
