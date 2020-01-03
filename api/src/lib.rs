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

use failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
mod web;
pub mod client;
mod handlers;
mod rest;
mod router;

pub use crate::handlers::start_rest_apis;
pub use crate::rest::*;
pub use crate::router::*;
pub use crate::web::*;
