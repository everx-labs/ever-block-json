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

use num::BigInt;
use serde_json::{Map, Value};
use std::str::FromStr;
use ton_types::{deserialize_tree_of_cells, error, fail, Result, UInt256};
use ton_block::{
    Deserializable,
    Account,
    BlockCreateFees,
    BlockLimits,
    CatchainConfig,
    ConfigParamEnum, ConfigParam0, ConfigParam1, ConfigParam2,
    ConfigParam7, ConfigParam8, ConfigParam9,
    ConfigParam10, ConfigParam11, ConfigParam12, ConfigParam13, ConfigParam14,
    ConfigParam15, ConfigParam16, ConfigParam17, ConfigParam18,
    ConfigParam29, ConfigParam31, ConfigParam34, ConfigParam40,
    ConfigParam18Map, ConfigParams,
    ConfigProposalSetup,
    ConsensusConfig,
    CurrencyCollection,
    ExtraCurrencyCollection,
    FundamentalSmcAddresses,
    GasLimitsPrices,
    GlobalVersion,
    Grams,
    LibDescr,
    MandatoryParams,
    McStateExtra,
    MsgForwardPrices,
    ParamLimits,
    ShardAccount, ShardIdent, ShardStateUnsplit,
    SlashingConfig,
    StoragePrices,
    ValidatorDescr, ValidatorSet,
    Workchains, WorkchainDescr, WorkchainFormat, WorkchainFormat0, WorkchainFormat1,
};

trait ParseJson {
    fn as_uint256(&self) -> Result<UInt256>;
    fn as_base64(&self) -> Result<Vec<u8>>;
    fn as_int(&self) -> Result<i32>;
    fn as_uint(&self) -> Result<u32>;
    fn as_long(&self) -> Result<i64>;
    fn as_ulong(&self) -> Result<u64>;
}

impl ParseJson for Value {
    fn as_uint256(&self) -> Result<UInt256> {
        UInt256::from_str(self.as_str().ok_or_else(|| error!("field is not str"))?)
    }
    fn as_base64(&self) -> Result<Vec<u8>> {
        Ok(base64::decode(self.as_str().ok_or_else(|| error!("field is not str"))?)?)
    }
    fn as_int(&self) -> Result<i32> {
        match self.as_i64() {
            Some(v) => Ok(v as i32),
            None => match self.as_str() {
                Some(s) => Ok(i32::from_str(s)?),
                None => Ok(i32::default())
            }
        }
    }
    fn as_uint(&self) -> Result<u32> {
        match self.as_u64() {
            Some(v) => Ok(v as u32),
            None => match self.as_str() {
                Some(s) => Ok(u32::from_str(s)?),
                None => Ok(u32::default())
            }
        }
    }
    fn as_long(&self) -> Result<i64> {
        match self.as_i64() {
            Some(v) => Ok(v),
            None => match self.as_str() {
                Some(s) => Ok(i64::from_str(s)?),
                None => Ok(i64::default())
            }
        }
    }
    fn as_ulong(&self) -> Result<u64> {
        match self.as_u64() {
            Some(v) => Ok(v),
            None => match self.as_str() {
                Some(s) => Ok(u64::from_str(s)?),
                None => Ok(u64::default())
            }
        }
    }
}

#[derive(Debug)]
struct PathMap<'m, 'a> {
    map: &'m Map<String, Value>,
    path: Vec<&'a str>
}

impl<'m, 'a> PathMap<'m, 'a> {
    fn new(map: &'m Map<String, Value>) -> Self {
        Self {
            map,
            path: vec!["root"]
        }
    }
    fn cont(prev: &Self, name: &'a str, value: &'m Value) -> Result<Self> {
        let map = value
            .as_object()
            .ok_or_else(|| error!("{}/{} must be the vector of objects", prev.path.join("/"), name))?;
        let mut path = prev.path.clone();
        path.push(name);
        Ok(Self {
            map,
            path
        })
    }
    fn get_item(&self, name: &'a str) -> Result<&'m Value> {
        let item = self.map.get(name).ok_or_else(|| error!("{} must have the field `{}`", self.path.join("/"), name))?;
        Ok(item)
    }
    fn get_obj(&self, name: &'a str) -> Result<Self> {
        let map = self.get_item(name)?
            .as_object()
            .ok_or_else(|| error!("{}/{} must be the object", self.path.join("/"), name))?;
        let mut path = self.path.clone();
        path.push(name);
        Ok(Self {
            map,
            path
        })
    }
    fn get_vec(&self, name: &'a str) -> Result<&'m Vec<Value>> {
        self.get_item(name)?
            .as_array()
            .ok_or_else(|| error!("{}/{} must be the vector", self.path.join("/"), name))
    }
    fn get_str(&self, name: &'a str) -> Result<&'m str> {
        self.get_item(name)?
            .as_str()
            .ok_or_else(|| error!("{}/{} must be the string", self.path.join("/"), name))
    }
    fn get_uint256(&self, name: &'a str) -> Result<UInt256> {
        UInt256::from_str(self.get_str(name)?)
            .map_err(|err| error!("{}/{} must be the uint256 in hex format : {}", self.path.join("/"), name, err))
    }
    fn get_base64(&self, name: &'a str) -> Result<Vec<u8>> {
        base64::decode(self.get_str(name)?)
            .map_err(|err| error!("{}/{} must be the base64 : {}", self.path.join("/"), name, err))
    }
    fn get_num(&self, name: &'a str) -> Result<i64> {
        let item = self.get_item(name)?;
        match item.as_i64() {
            Some(v) => Ok(v),
            None => match item.as_str() {
                Some(s) => {
                    i64::from_str(s)
                    .map_err(|_| error!("{}/{} must be the integer or a string with the integer {}", self.path.join("/"), name, s))
                }
                None => fail!("{}/{} must be the integer or a string with the integer {}", self.path.join("/"), name, item)
            }
        }
    }
    fn get_bigint(&self, name: &'a str) -> Result<BigInt> {
        let item = self.get_item(name)?;
        match item.as_i64() {
            Some(v) => Ok(v.into()),
            None => match item.as_str() {
                Some(s) => {
                    BigInt::from_str(s)
                        .map_err(|_| error!("{}/{} must be the integer or a string with the integer {}", self.path.join("/"), name, s))
                }
                None => fail!("{}/{} must be the integer or a string with the integer {}", self.path.join("/"), name, item)
            }
        }
    }
    #[allow(dead_code)]
    fn get_u32(&self, name: &'a str, value: &mut u32) {
        if let Ok(new_value) = self.get_num(name) {
            *value = new_value as u32;
        }
    }
    fn get_bool(&self, name: &'a str) -> Result<bool> {
        self.get_item(name)?
            .as_bool()
            .ok_or_else(|| error!("{}/{} must be boolean", self.path.join("/"), name))
    }
}

struct StateParser {
    state: ShardStateUnsplit,
    extra: McStateExtra,
    errors: Vec<failure::Error>,
}

impl StateParser {

    fn new() -> Self {
        Self {
            state: ShardStateUnsplit::with_ident(ShardIdent::masterchain()),
            extra: McStateExtra::default(),
            errors: Vec::new()
        }
    }

    fn set_config(&mut self, map: &PathMap, config: ConfigParamEnum) {
        if let Err(err) = self.extra.config.set_config(config) {
            self.errors.push(error!("Can't set config for {} : {}", map.path.join("/"), err));
        }
    }

    fn parse_param_limits(param: &PathMap) -> Result<ParamLimits> {
        ParamLimits::with_limits(
            param.get_num("underload")? as u32,
            param.get_num("soft_limit")? as u32,
            param.get_num("hard_limit")? as u32,
        )
    }

    fn parse_block_limits(param: &PathMap) -> Result<BlockLimits> {
        Ok(BlockLimits::with_limits(
            Self::parse_param_limits(&param.get_obj("bytes")?)?,
            Self::parse_param_limits(&param.get_obj("gas")?)?,
            Self::parse_param_limits(&param.get_obj("lt_delta")?)?,
        ))
    }

    fn parse_msg_forward_prices(param: &PathMap) -> Result<MsgForwardPrices> {
        Ok(MsgForwardPrices {
            lump_price:       param.get_num("lump_price")? as u64,
            bit_price:        param.get_num("bit_price")? as u64,
            cell_price:       param.get_num("cell_price")? as u64,
            ihr_price_factor: param.get_num("ihr_price_factor")? as u32,
            first_frac:       param.get_num("first_frac")? as u16,
            next_frac:        param.get_num("next_frac")? as u16,
        })
    }

    fn parse_gas_limits(&mut self, config: &PathMap, name: &str) -> Option<GasLimitsPrices> {
        let result = config.get_obj(name).and_then(|param| Ok(GasLimitsPrices {
            gas_price:         param.get_num("gas_price")? as u64,
            gas_limit:         param.get_num("gas_limit")? as u64,
            special_gas_limit: param.get_num("special_gas_limit")? as u64,
            gas_credit:        param.get_num("gas_credit")? as u64,
            block_gas_limit:   param.get_num("block_gas_limit")? as u64,
            freeze_due_limit:  param.get_num("freeze_due_limit")? as u64,
            delete_due_limit:  param.get_num("delete_due_limit")? as u64,
            flat_gas_limit:    param.get_num("flat_gas_limit")? as u64,
            flat_gas_price:    param.get_num("flat_gas_price")? as u64,
            max_gas_threshold: 0,
        }));
        match result {
            Err(err) => {
                self.errors.push(err);
                None
            }
            Ok(param) => Some(param)
        }
    }

    fn parse_param_set(&mut self, config: &PathMap, name: &str) -> Option<MandatoryParams> {
        match config.get_vec(name) {
            Ok(vec) => {
                let mut params = MandatoryParams::default();
                match vec.iter().try_for_each(|n| params.set(&n.as_uint()?, &())) {
                    Ok(_) => return Some(params),
                    Err(err) => self.errors.push(err)
                }
            }
            Err(err) => self.errors.push(err)
        }
        None
    }

    fn parse_critical_params(&mut self, p11: &PathMap, name: &str) -> ConfigProposalSetup {
        let mut normal_params = ConfigProposalSetup::default();
        if let Err(err) = p11.get_obj(name).and_then(|params| {
            normal_params.min_tot_rounds = params.get_num("min_tot_rounds")? as u8;
            normal_params.max_tot_rounds = params.get_num("max_tot_rounds")? as u8;
            normal_params.min_wins       = params.get_num("min_wins"      )? as u8;
            normal_params.max_losses     = params.get_num("max_losses"    )? as u8;
            normal_params.min_store_sec  = params.get_num("min_store_sec" )? as u32;
            normal_params.max_store_sec  = params.get_num("max_store_sec" )? as u32;
            normal_params.bit_price      = params.get_num("bit_price"     )? as u32;
            normal_params.cell_price     = params.get_num("cell_price"    )? as u32;
            Ok(())
        }) { self.errors.push(err) }
        normal_params
    }

    fn parse_p11(&mut self, config: &PathMap) {
        if let Err(err) = config.get_obj("p11").and_then(|p11| {
            let normal_params = self.parse_critical_params(&p11, "normal_params");
            let critical_params = self.parse_critical_params(&p11, "critical_params");
            let p11 = ConfigParam11::new(&normal_params, &critical_params)?;
            self.set_config(&config, ConfigParamEnum::ConfigParam11(p11));
            Ok(())
        }) { self.errors.push(err) }
    }

    fn parse_p12(&mut self, config: &PathMap) {
        if let Err(err) = config.get_vec("p12").and_then(|p12| {
            let mut workchains = Workchains::default();
            p12.iter().try_for_each(|wc_info| {
                let wc_info = PathMap::cont(&config, "p12", wc_info)?;
                let mut descr = WorkchainDescr::default();
                let workchain_id = wc_info.get_num("workchain_id")? as u32;
                descr.enabled_since = wc_info.get_num("enabled_since")? as u32;
                descr.set_min_split(wc_info.get_num("min_split")? as u8)?;
                descr.set_max_split(wc_info.get_num("max_split")? as u8)?;
                descr.flags = wc_info.get_num("flags")? as u16;
                descr.active = wc_info.get_bool("active")?;
                descr.accept_msgs = wc_info.get_bool("accept_msgs")?;
                descr.zerostate_root_hash = wc_info.get_uint256("zerostate_root_hash")?;
                descr.zerostate_file_hash = wc_info.get_uint256("zerostate_file_hash")?;
                // TODO: check here
                descr.format = match wc_info.get_bool("basic")? {
                    true => {
                        let vm_version = wc_info.get_num("vm_version")? as i32;
                        let vm_mode    = wc_info.get_num("vm_mode"   )? as u64;
                        WorkchainFormat::Basic(WorkchainFormat1::with_params(vm_version, vm_mode))
                    }
                    false => {
                        let min_addr_len      = wc_info.get_num("min_addr_len")? as u16;
                        let max_addr_len      = wc_info.get_num("max_addr_len")? as u16;
                        let addr_len_step     = wc_info.get_num("addr_len_step")? as u16;
                    let workchain_type_id = wc_info.get_num("workchain_type_id")? as u32;
                    WorkchainFormat::Extended(WorkchainFormat0::with_params(min_addr_len, max_addr_len, addr_len_step, workchain_type_id)?)
                    }
                };
                workchains.set(&workchain_id, &descr)
            })?;
            self.set_config(&config, ConfigParamEnum::ConfigParam12(ConfigParam12 {workchains}));
            Ok(())
        }) { self.errors.push(err) }
    }

    pub fn parse_config(&mut self, config: &PathMap) -> Result<()> {
        match config.get_uint256("p0") {
            Ok(config_addr) => self.set_config(&config, ConfigParamEnum::ConfigParam0(ConfigParam0 {config_addr} )),
            Err(err) => self.errors.push(err)
        }
        match config.get_uint256("p1") {
            Ok(elector_addr) => self.set_config(&config, ConfigParamEnum::ConfigParam1(ConfigParam1 {elector_addr} )),
            Err(err) => self.errors.push(err)
        }
        match config.get_uint256("p2") {
            Ok(minter_addr) => self.set_config(&config, ConfigParamEnum::ConfigParam2(ConfigParam2 {minter_addr} )),
            Err(err) => self.errors.push(err)
        }

        if let Err(err) = config.get_vec("p7").and_then(|p7| {
            let mut to_mint = ExtraCurrencyCollection::default();
            p7.iter().try_for_each(|currency| {
                let currency = PathMap::cont(&config, "p7", currency)?;
                to_mint.set(
                    &(currency.get_num("currency")? as u32),
                    &BigInt::from_str(currency.get_str("value")?)?.into()
                )
            })?;
            self.set_config(&config, ConfigParamEnum::ConfigParam7(ConfigParam7 {to_mint} ));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_obj("p8").and_then(|p8| {
            match (p8.get_num("version"), p8.get_num("capabilities")) {
                (Ok(version), Ok(capabilities)) => {
                    let global_version = GlobalVersion {version: version as u32, capabilities: capabilities as u64};
                    self.set_config(&config, ConfigParamEnum::ConfigParam8(ConfigParam8 {global_version} ));
                }
                (Err(err), Ok(_)) => self.errors.push(err),
                (Ok(_), Err(err)) => self.errors.push(err),
                (Err(err1), Err(err2)) => {
                    self.errors.push(err1);
                    self.errors.push(err2);
                }
            }
            Ok(())
        }) { self.errors.push(err) }

        if let Some(mandatory_params) = self.parse_param_set(&config, "p9") {
            self.set_config(&config, ConfigParamEnum::ConfigParam9(ConfigParam9 {mandatory_params} ));
        }

        if let Some(critical_params) = self.parse_param_set(&config, "p10") {
            self.set_config(&config, ConfigParamEnum::ConfigParam10(ConfigParam10 {critical_params} ));
        }

        self.parse_p11(&config);

        self.parse_p12(&config);

        if let Ok(p13) = config.get_obj("p13") {
            let cell = deserialize_tree_of_cells(&mut std::io::Cursor::new(p13.get_base64("boc")?))?;
            self.set_config(&config, ConfigParamEnum::ConfigParam13(ConfigParam13 {cell}));
        }

        if let Err(err) = config.get_obj("p14").and_then(|p14| {
            let masterchain_block_fee = Grams::from(p14.get_num("masterchain_block_fee")? as u64);
            let basechain_block_fee = Grams::from(p14.get_num("basechain_block_fee")? as u64);
            let block_create_fees = BlockCreateFees { masterchain_block_fee, basechain_block_fee };
            self.set_config(&config, ConfigParamEnum::ConfigParam14(ConfigParam14 {block_create_fees}));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_obj("p15").and_then(|p15| {
            let p15 = ConfigParam15 {
                validators_elected_for: p15.get_num("validators_elected_for")? as u32,
                elections_start_before: p15.get_num("elections_start_before")? as u32,
                elections_end_before:   p15.get_num("elections_end_before")? as u32,
                stake_held_for:         p15.get_num("stake_held_for")? as u32,
            };
            self.set_config(&config, ConfigParamEnum::ConfigParam15(p15));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_obj("p16").and_then(|p16| {
            let p16 = ConfigParam16 {
                min_validators:      p16.get_num("min_validators")?.into(),
                max_validators:      p16.get_num("max_validators")?.into(),
                max_main_validators: p16.get_num("max_main_validators")?.into(),
            };
            self.set_config(&config, ConfigParamEnum::ConfigParam16(p16));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_obj("p17").and_then(|p17| {
            let p17 = ConfigParam17 {
                min_stake:        p17.get_num("min_stake")?.into(),
                max_stake:        p17.get_num("max_stake")?.into(),
                min_total_stake:  p17.get_num("min_total_stake")?.into(),
                max_stake_factor: p17.get_num("max_stake_factor")? as u32,
            };
            self.set_config(&config, ConfigParamEnum::ConfigParam17(p17));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_vec("p18").and_then(|p18| {
            let mut map = ConfigParam18Map::default();
            let mut index = 0u32;
            p18.iter().try_for_each::<_, Result<_>>(|p| {
                let p = PathMap::cont(&config, "p18", p)?;
                let p = StoragePrices {
                    utime_since:      p.get_num("utime_since")? as u32,
                    bit_price_ps:     p.get_num("bit_price_ps")? as u64,
                    cell_price_ps:    p.get_num("cell_price_ps")? as u64,
                    mc_bit_price_ps:  p.get_num("mc_bit_price_ps")? as u64,
                    mc_cell_price_ps: p.get_num("mc_cell_price_ps")? as u64,
                };
                map.set(&index, &p)?;
                index += 1;
                Ok(())
            })?;
            self.set_config(&config, ConfigParamEnum::ConfigParam18(ConfigParam18 { map }));
            Ok(())
        }) { self.errors.push(err) }

        if let Some(p20) = self.parse_gas_limits(&config, "p20") {
            self.set_config(&config, ConfigParamEnum::ConfigParam20(p20));
        }

        if let Some(p21) = self.parse_gas_limits(&config, "p21") {
            self.set_config(&config, ConfigParamEnum::ConfigParam21(p21));
        }

        match config.get_obj("p22").and_then(|p22| Self::parse_block_limits(&p22)) {
            Ok(p22) => self.set_config(&config, ConfigParamEnum::ConfigParam22(p22)),
            Err(err) => self.errors.push(err)
        }
        match config.get_obj("p23").and_then(|p23| Self::parse_block_limits(&p23)) {
            Ok(p23) => self.set_config(&config, ConfigParamEnum::ConfigParam23(p23)),
            Err(err) => self.errors.push(err)
        }
        match config.get_obj("p24").and_then(|p24| Self::parse_msg_forward_prices(&p24)) {
            Ok(p24) => self.set_config(&config, ConfigParamEnum::ConfigParam24(p24)),
            Err(err) => self.errors.push(err)
        }
        match config.get_obj("p25").and_then(|p25| Self::parse_msg_forward_prices(&p25)) {
            Ok(p25) => self.set_config(&config, ConfigParamEnum::ConfigParam25(p25)),
            Err(err) => self.errors.push(err)
        }

        if let Err(err) = config.get_obj("p28").and_then(|p28| {
            let p28 = CatchainConfig {
                shuffle_mc_validators:     p28.get_bool("shuffle_mc_validators")?,
                isolate_mc_validators:     p28.get_bool("isolate_mc_validators").unwrap_or_default(),
                mc_catchain_lifetime:      p28.get_num("mc_catchain_lifetime")? as u32,
                shard_catchain_lifetime:   p28.get_num("shard_catchain_lifetime")? as u32,
                shard_validators_lifetime: p28.get_num("shard_validators_lifetime")? as u32,
                shard_validators_num:      p28.get_num("shard_validators_num")? as u32,
            };
            self.set_config(&config, ConfigParamEnum::ConfigParam28(p28));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_obj("p29").and_then(|p29| {
            let consensus_config = ConsensusConfig {
                new_catchain_ids:        p29.get_bool("new_catchain_ids")?,
                round_candidates:        p29.get_num("round_candidates")? as u32,
                next_candidate_delay_ms: p29.get_num("next_candidate_delay_ms")? as u32,
                consensus_timeout_ms:    p29.get_num("consensus_timeout_ms")? as u32,
                fast_attempts:           p29.get_num("fast_attempts")? as u32,
                attempt_duration:        p29.get_num("attempt_duration")? as u32,
                catchain_max_deps:       p29.get_num("catchain_max_deps")? as u32,
                max_block_bytes:         p29.get_num("max_block_bytes")? as u32,
                max_collated_bytes:      p29.get_num("max_collated_bytes")? as u32,
            };
            self.set_config(&config, ConfigParamEnum::ConfigParam29(ConfigParam29 {consensus_config}));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_vec("p31").and_then(|p31| {
            let mut fundamental_smc_addr = FundamentalSmcAddresses::default();
            p31.iter().try_for_each(|n| fundamental_smc_addr.set(&n.as_uint256()?, &()))?;
            self.set_config(&config, ConfigParamEnum::ConfigParam31(ConfigParam31 {fundamental_smc_addr} ));
            Ok(())
        }) { self.errors.push(err) }

        if let Err(err) = config.get_obj("p34").and_then(|p34| {
            let mut list = vec![];
            p34.get_vec("list").and_then(|p| p.iter().try_for_each::<_, Result<()>>(|p| {
                let p = PathMap::cont(&config, "p34", p)?;
                list.push(ValidatorDescr::with_params(
                    FromStr::from_str(p.get_str("public_key")?)?,
                    p.get_num("weight")? as u64,
                    None
                ));
                Ok(())
            }))?;
            let cur_validators = ValidatorSet::new(
                p34.get_num("utime_since")? as u32,
                p34.get_num("utime_until")? as u32,
                p34.get_num("main")? as u16,
                list
            )?;
            self.set_config(&config, ConfigParamEnum::ConfigParam34(ConfigParam34 {cur_validators}));
            Ok(())
        }) { self.errors.push(err) }

        let mut slashing_config = SlashingConfig::default();
        if let Ok(p40) = config.get_obj("p40") {
            p40.get_u32("slashing_period_mc_blocks_count", &mut slashing_config.slashing_period_mc_blocks_count);
            p40.get_u32("resend_mc_blocks_count", &mut slashing_config.resend_mc_blocks_count);
            p40.get_u32("min_samples_count", &mut slashing_config.min_samples_count);
            p40.get_u32("collations_score_weight", &mut slashing_config.collations_score_weight);
            p40.get_u32("signing_score_weight", &mut slashing_config.signing_score_weight);
            p40.get_u32("min_slashing_protection_score", &mut slashing_config.min_slashing_protection_score);
            p40.get_u32("z_param_numerator", &mut slashing_config.z_param_numerator);
            p40.get_u32("z_param_denominator", &mut slashing_config.z_param_denominator);
        }
        self.set_config(&config, ConfigParamEnum::ConfigParam40(ConfigParam40 {slashing_config}));
        Ok(())
    }

    fn parse_state_unchecked(mut self, map: &Map<String, Value>) -> (ShardStateUnsplit, Vec<failure::Error>) {
        let map_path = PathMap::new(map);

        self.state.set_min_ref_mc_seqno(std::u32::MAX);

        match map_path.get_num("global_id") {
            Ok(global_id) => self.state.set_global_id(global_id as i32),
            Err(err) => self.errors.push(err)
        }
        match map_path.get_num("gen_utime") {
            Ok(gen_utime) => self.state.set_gen_time(gen_utime as u32),
            Err(err) => self.errors.push(err)
        }

        match map_path.get_bigint("total_balance") {
            Ok(balance) => self.state.set_total_balance(CurrencyCollection::from_grams(Grams::from(balance))),
            Err(err) => self.errors.push(err)
        }

        if let Err(err) = map_path.get_obj("master").and_then(|master| {
            let config = master.get_obj("config")?;
            self.parse_config(&config)?;
            match master.get_uint256("config_addr") {
                Ok(addr) => self.extra.config.config_addr = addr,
                Err(err) => self.errors.push(err)
            }
            match master.get_num("validator_list_hash_short") {
                Ok(v) => self.extra.validator_info.validator_list_hash_short = v as u32,
                Err(err) => self.errors.push(err)
            }
            match master.get_num("catchain_seqno") {
                Ok(v) => self.extra.validator_info.catchain_seqno = v as u32,
                Err(err) => self.errors.push(err)
            }
            match master.get_bool("nx_cc_updated") {
                Ok(v) => self.extra.validator_info.nx_cc_updated = v,
                Err(err) => self.errors.push(err)
            }
            match master.get_bigint("global_balance") {
                Ok(balance) => self.extra.global_balance.grams = Grams::from(balance),
                Err(err) => self.errors.push(err)
            }
            self.extra.after_key_block = true;
            self.state.write_custom(Some(&self.extra))
        }) { self.errors.push(err) }

        if let Err(err) = map_path.get_vec("accounts").and_then(|accounts| {
            accounts.iter().try_for_each::<_, Result<()>>(|account| {
                let account = PathMap::cont(&map_path, "accounts", account)?;
                let id = account.get_str("id")?;
                let account_id = UInt256::from_str(id.trim_start_matches("-1:"))?;
                Account::construct_from_bytes(&account.get_base64("boc")?)
                    .and_then(|acc| ShardAccount::with_params(&acc, UInt256::default(), 0))
                    .and_then(|acc| self.state.insert_account(&account_id, &acc))
            })
        }) { self.errors.push(err) }

        if let Err(err) = map_path.get_vec("libraries").and_then(|libraries| {
            libraries.iter().try_for_each::<_, Result<()>>(|library| {
                let library = PathMap::cont(&map_path, "libraries", library)?;
                let id = library.get_uint256("hash")?;
                let lib = library.get_base64("lib")?;
                let lib = deserialize_tree_of_cells(&mut std::io::Cursor::new(lib))?;
                let mut lib = LibDescr::new(lib);
                let publishers = library.get_vec("publishers")?;
                publishers.iter().try_for_each::<_, Result<()>>(|publisher| {
                    lib.publishers_mut().set(&publisher.as_uint256()?, &())
                })?;
                self.state.libraries_mut().set(&id, &lib)
            })
        }) { self.errors.push(err) }

        (self.state, self.errors)
    }
}

pub fn parse_config(config: &Map<String, Value>) -> Result<ConfigParams> {
    let config = PathMap::new(config);
    let mut parser = StateParser::new();
    parser.parse_config(&config)?;
    Ok(parser.extra.config)
}

pub fn parse_state(map: &Map<String, Value>) -> Result<ShardStateUnsplit> {
    let (state, mut errors) = StateParser::new().parse_state_unchecked(map);
    match errors.pop() {
        Some(err) => Err(err),
        None => Ok(state)
    }
}

pub fn parse_state_unchecked(map: &Map<String, Value>) -> (ShardStateUnsplit, Vec<failure::Error>) {
    StateParser::new().parse_state_unchecked(map)
}

