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

use crate::rest::*;
use crate::router::{Handler, ResponseFuture};
use crate::web::*;
use hyper::{Body, Request};
use serde::{Deserialize, Serialize};

const CRATE_VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// API Version Information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Version {
	/// Current oracle API Version (api crate version)
	pub oracle_version: String,
	/// Block header version
	pub block_header_version: u16,
}

/// Version handler. Get running node API version
/// GET /v1/version
pub struct VersionHandler {}

impl VersionHandler {
	pub fn get_version(&self) -> Result<Version, Error> {
		debug!("API call: get_version");
		Ok(Version {
			oracle_version: CRATE_VERSION.to_owned(),
			block_header_version: 0u16,
		})
	}
}

impl Handler for VersionHandler {
	fn get(&self, _req: Request<Body>) -> ResponseFuture {
		result_to_response(self.get_version())
	}
}
