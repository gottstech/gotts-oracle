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

use std::sync::Arc;
use std::thread;
use std::time::Duration;

extern crate gotts_oracle_alphavantage;
use gotts_oracle_alphavantage as alphavantage;

extern crate gotts_oracle_api;
use gotts_oracle_api as api;

fn main() {
    //create alphavantage client
    let shared_client = Arc::new(alphavantage::Client::new("2BY6TAJHCM9Z7HQT"));

    //start api server
    let res = api::start_rest_apis(shared_client.clone(), "127.0.0.1:8008".to_string(), None);

    if let Ok(handle) = res {
        handle.join().expect("The thread being joined has panicked");
    }
    thread::sleep(Duration::from_millis(1000));
}
