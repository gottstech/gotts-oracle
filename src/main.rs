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

use colored::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[macro_use]
extern crate log;
use gotts_oracle_alphavantage as alphavantage;
use gotts_oracle_api as api;
use gotts_oracle_config as config;
use gotts_oracle_util::init_logger;

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn info_strings() -> (String, String) {
	(
		format!(
			"This is Gotts-Oracle version {}{}, built for {} by {}.",
			built_info::PKG_VERSION,
			built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
			built_info::TARGET,
			built_info::RUSTC_VERSION,
		)
		.to_string(),
		format!(
			"Built with profile \"{}\", features \"{}\".",
			built_info::PROFILE,
			built_info::FEATURES_STR,
		)
		.to_string(),
	)
}

fn log_build_info() {
	let (basic_info, detailed_info) = info_strings();
	warn!("{}", basic_info);
	debug!("{}", detailed_info);
}

fn main() {
	let node_config = Some(config::initial_setup_server().unwrap_or_else(|e| {
		panic!("Error loading server configuration: {}", e);
	}));

	if let Some(mut config) = node_config.clone() {
		let l = config.members.as_mut().unwrap().logging.clone().unwrap();
		init_logger(Some(l));

		if let Some(file_path) = &config.config_file_path {
			warn!(
				"Using configuration file at {}",
				file_path.to_str().unwrap()
			);
		} else {
			warn!("Node configuration file not found, using default");
		}
	}

	log_build_info();

	//the api key integrated here is just for demo, with very limited access,
	// please claim your own api key and set it as an environment variable before running.
	// the free api key can be requested here: https://www.alphavantage.co/support/#api-key
	//
	let default_api_key = "2BY6TAJHCM9Z7HQT";
	let alpha_vantage_api_key =
		std::env::var("ALPHAVANTAGE_API_KEY").unwrap_or_else(|_| default_api_key.to_string());
	if alpha_vantage_api_key == default_api_key {
		println!(
			"\n{} the default api key hardcoded is just for demo with very limited access.\
			 \nplease claim your own api key from https://www.alphavantage.co/support/#api-key\
			 \nand then set it as an environment variable 'ALPHAVANTAGE_API_KEY' before running.",
			"warning!".to_string().bright_red(),
		);
	}

	//create alphavantage client
	let shared_client = Arc::new(alphavantage::Client::new(alpha_vantage_api_key.as_str()));

	//start api server
	let oracle_bind_address =
		std::env::var("ORACLE_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8008".to_string());
	println!(
		"\ngotts oracle is serving on {}",
		oracle_bind_address.bright_green()
	);
	let res = api::start_rest_apis(shared_client.clone(), oracle_bind_address, None);

	if let Ok(handle) = res {
		handle.join().expect("The thread being joined has panicked");
	}
	thread::sleep(Duration::from_millis(1000));
}
