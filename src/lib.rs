/*
 * Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.  You may obtain a copy of the
 * License at:
 *
 * https://www.ton.dev/licenses
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and limitations
 * under the License.
 */

mod serialize;
pub use self::serialize::*;
mod deserialize;
pub use self::deserialize::*;

#[cfg(test)]
fn check_with_ethalon_file(json: &str, name: &str) {
    let ethalon = std::fs::read_to_string(format!("real_ton_data/{}-ethalon.json", name))
        .unwrap();
    check_with_ethalon(json, &ethalon, name);
}

#[cfg(test)]
fn check_with_ethalon(json: &str, ethalon: &str, name: &str) {
    let ethalon = ethalon.replace("\r", "");
    let ethalon = if let Some(new_ethalon) = ethalon.strip_suffix("\n") {
        new_ethalon.to_string()
    } else {
        ethalon
    };
    if json != ethalon {
        //assert_eq!(json, ethalon.replace("\r", ""));
        std::fs::write(format!("real_ton_data/{}.json", name), &json).unwrap();
        panic!("json != ethalon")
    }
}
