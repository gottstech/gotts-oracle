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

//! JSON-RPC Stub generation for the Foreign API

use crate::foreign::Foreign;
use crate::handlers::version_api::Version;
use crate::rest::ErrorKind;
use gotts_oracle_alphavantage::exchange_rate::ExchangeRateResult;
use gotts_oracle_lib::OracleBackend;

/// Public definition used to generate Oracle jsonrpc api.
/// * When running `gotts_oracle` with defaults, the V1 json api is available at
/// `localhost:3518/v1/json`
/// * The endpoint only supports POST operations, with the json-rpc request as the body
#[easy_jsonrpc_mw::rpc]
pub trait ForeignRpc: Sync + Send {
	fn get_version(&self) -> Result<Version, ErrorKind>;
	fn get_rate(&self, from: String, to: String) -> Result<ExchangeRateResult, ErrorKind>;
	fn get_recent(
		&self,
		prefix: String,
		items: usize,
	) -> Result<Vec<ExchangeRateResult>, ErrorKind>;
	fn compact(&self, minutes: u32) -> Result<usize, ErrorKind>;
	fn get_aggregated(&self) -> Result<Vec<ExchangeRateResult>, ErrorKind>;
}

impl<T: ?Sized> ForeignRpc for Foreign<T>
where
	T: OracleBackend + Send + Sync + 'static,
{
	fn get_version(&self) -> Result<Version, ErrorKind> {
		Foreign::get_version(self).map_err(|e| e.kind().clone())
	}

	fn get_rate(&self, from: String, to: String) -> Result<ExchangeRateResult, ErrorKind> {
		Foreign::get_rate(self, from, to).map_err(|e| e.kind().clone())
	}

	fn get_recent(
		&self,
		prefix: String,
		items: usize,
	) -> Result<Vec<ExchangeRateResult>, ErrorKind> {
		Foreign::get_recent(self, prefix, items).map_err(|e| e.kind().clone())
	}

	fn compact(&self, minutes: u32) -> Result<usize, ErrorKind> {
		Foreign::compact(self, minutes).map_err(|e| e.kind().clone())
	}

	fn get_aggregated(&self) -> Result<Vec<ExchangeRateResult>, ErrorKind> {
		Foreign::get_aggregated(self).map_err(|e| e.kind().clone())
	}
}
