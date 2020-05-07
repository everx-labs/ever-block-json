/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
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

use ton_block::*;
use ton_types::{
    Result,
    {Cell, SliceData},
    cells_serialization::{serialize_toc},
    dictionary::HashmapType,
    types::UInt256,
};
use num::BigInt;
use serde_json::{Map, Value};

const VERSION: u32 = 1;

#[derive(Clone, Copy)]
pub enum SerializationMode {
    Standart,
    QServer
}

impl SerializationMode {
    pub fn is_standart(&self) -> bool {
        match self {
            SerializationMode::Standart => true,
            _ => false
        }
    }

    pub fn is_q_server(&self) -> bool {
        match self {
            SerializationMode::QServer => true,
            _ => false
        }
    }
}

fn grams_to_string(value: &BigInt, mode: SerializationMode) -> String {
    match mode {
        SerializationMode::Standart => {
            let mut string = format!("{:x}", value);
            string.insert_str(0, &format!("{:02x}", string.len() - 1));
            string        
        }
        SerializationMode::QServer => {
            format!("0x{:x}", value)
        }
    }
}

fn u64_to_string(value: &u64, mode: SerializationMode) -> String {
    match mode {
        SerializationMode::Standart => {
            let mut string = format!("{:x}", value);
            string.insert_str(0, &format!("{:x}", string.len() - 1));
            string
        }
        SerializationMode::QServer => {
            format!("0x{:x}", value)
        }
    }
}

fn shard_to_string(value: u64) -> String {
    format!("{:016x}", value)
}

fn serialize_cell(
    map: &mut Map<String, Value>,
    id_str: &'static str,
    cell: Option<&Cell>,
    write_hash: bool,
) -> Result<()> {
    if let Some(cell) = cell {
        let bytes = serialize_toc(cell)?;
        serialize_field(map, id_str, base64::encode(&bytes));
        if write_hash {
            let string = id_str.to_owned() + "_hash";
            serialize_uint256(map, &string, &cell.repr_hash())
        }
    }
    Ok(())
}

fn serialize_slice(
    map: &mut Map<String, Value>,
    id_str: &'static str,
    slice: Option<&SliceData>,
    write_hash: bool,
) -> Result<()> {
    if let Some(slice) = slice {
        let cell = slice.into_cell();
        let bytes = serialize_toc(&cell)?;
        serialize_field(map, id_str, base64::encode(&bytes));
        if write_hash {
            let string = id_str.to_owned() + "_hash";
            serialize_uint256(map, &string, &cell.repr_hash())
        }
    }
    Ok(())
}

fn serialize_id(map: &mut Map<String, Value>, id_str: & str, id: Option<&UInt256>) {
    if let Some(id) = id {
        map.insert(id_str.to_string(), id.to_hex_string().into());
    }
}

fn serialize_uint256(map: &mut Map<String, Value>, name: & str, value: &UInt256) {
    map.insert(name.to_string(), value.to_hex_string().into());
}

fn serialize_field<S: Into<Value>>(map: &mut Map<String, Value>, id_str: &str, value: S) {
    map.insert(id_str.to_string(), value.into());
}

fn serialize_split_info(map: &mut Map<String, Value>, split_info: &SplitMergeInfo) {
    serialize_field(map, "cur_shard_pfx_len", split_info.cur_shard_pfx_len);
    serialize_field(map, "acc_split_depth", split_info.acc_split_depth);
    serialize_id(map, "this_addr", Some(&split_info.this_addr));
    serialize_id(map, "sibling_addr", Some(&split_info.sibling_addr));
}

fn serialize_storage_phase(map: &mut Map<String, Value>, ph: Option<&TrStoragePhase>, mode: SerializationMode) {
    if let Some(ph) = ph {
        let mut ph_map = serde_json::Map::new();
        serialize_field(&mut ph_map, "storage_fees_collected", grams_to_string(&ph.storage_fees_collected.value(), mode));
        if let Some(grams) = &ph.storage_fees_due {
            serialize_field(&mut ph_map, "storage_fees_due", grams_to_string(&grams.value(), mode));
        }
        let status_change = match ph.status_change {
            AccStatusChange::Unchanged => 0,
            AccStatusChange::Frozen => 1,
            AccStatusChange::Deleted => 2,
        };
        serialize_field(&mut ph_map, "status_change", status_change);
        if mode.is_q_server() {
            let status_change = match ph.status_change {
                AccStatusChange::Unchanged => "unchanged",
                AccStatusChange::Frozen => "frozen",
                AccStatusChange::Deleted => "deleted",
            };
            serialize_field(&mut ph_map, "status_change_name", status_change);
        }
        serialize_field(map, "storage", ph_map);
    }
}

fn serialize_compute_phase(map: &mut Map<String, Value>, ph: Option<&TrComputePhase>, mode: SerializationMode) {
    let mut ph_map = serde_json::Map::new();
    let (type_, type_name) = match ph {
        Some(TrComputePhase::Skipped(ph)) => {
            let reason = match ph.reason {
                ComputeSkipReason::NoState => 0,
                ComputeSkipReason::BadState => 1,
                ComputeSkipReason::NoGas   => 2,
            };
            ph_map.insert("skipped_reason".to_string(), reason.into());
            if mode.is_q_server() {
                let reason = match ph.reason {
                    ComputeSkipReason::NoState  => "noState",
                    ComputeSkipReason::BadState => "badState",
                    ComputeSkipReason::NoGas    => "noGas",
                };
                ph_map.insert("skipped_reason_name".to_string(), reason.into());
            }
            (0, "skipped")
        }
        Some(TrComputePhase::Vm(ph)) => {
            ph_map.insert("success".to_string(), ph.success.into());
            ph_map.insert("msg_state_used".to_string(), ph.msg_state_used.into());
            ph_map.insert("account_activated".to_string(), ph.account_activated.into());
            ph_map.insert("gas_fees".to_string(), grams_to_string(&ph.gas_fees.value(), mode).into());
            ph_map.insert("gas_used".to_string(), ph.gas_used.0.into());
            ph_map.insert("gas_limit".to_string(), ph.gas_limit.0.into());
            ph.gas_credit.as_ref().map(|value| ph_map.insert("gas_credit".to_string(), value.0.into()));
            ph_map.insert("mode".to_string(), ph.mode.into());
            ph_map.insert("exit_code".to_string(), ph.exit_code.into());
            ph.exit_arg.map(|value| ph_map.insert("exit_arg".to_string(), value.into()));
            ph_map.insert("vm_steps".to_string(), ph.vm_steps.into());
            serialize_id(&mut ph_map, "vm_init_state_hash", Some(&ph.vm_init_state_hash));
            serialize_id(&mut ph_map, "vm_final_state_hash", Some(&ph.vm_final_state_hash));
            (1, "vm")
        }
        None => return
    };
    ph_map.insert("compute_type".to_string(), type_.into());
    if mode.is_q_server() {
        ph_map.insert("compute_type_name".to_string(), type_name.into());
    }
    serialize_field(map, "compute", ph_map);
}

fn serialize_credit_phase(map: &mut Map<String, Value>, ph: Option<&TrCreditPhase>, mode: SerializationMode) -> Result<()> {
    if let Some(ph) = ph {
        let mut ph_map = serde_json::Map::new();
        if let Some(grams) = &ph.due_fees_collected {
            ph_map.insert("due_fees_collected".to_string(), grams_to_string(&grams.value(), mode).into());
        }
        serialize_cc(&mut ph_map, "credit", &ph.credit, mode)?;
        serialize_field(map, "credit", ph_map);
    }
    Ok(())
}

fn serialize_action_phase(map: &mut Map<String, Value>, ph: Option<&TrActionPhase>, mode: SerializationMode) {
    if let Some(ph) = ph {
        let mut ph_map = serde_json::Map::new();
        ph_map.insert("success".to_string(), ph.success.into());
        ph_map.insert("valid".to_string(), ph.valid.into());
        ph_map.insert("no_funds".to_string(), ph.no_funds.into());
        let status_change = match ph.status_change {
            AccStatusChange::Unchanged => 0,
            AccStatusChange::Frozen => 1,
            AccStatusChange::Deleted => 2,
        };
        serialize_field(&mut ph_map, "status_change", status_change);
        ph.total_fwd_fees.as_ref().map(|grams|
            ph_map.insert("total_fwd_fees".to_string(), grams_to_string(&grams.value(), mode).into()));
        ph.total_action_fees.as_ref().map(|grams|
            ph_map.insert("total_action_fees".to_string(), grams_to_string(&grams.value(), mode).into()));
        ph_map.insert("result_code".to_string(), ph.result_code.into());
        ph.result_arg.map(|value| ph_map.insert("result_arg".to_string(), value.into()));
        ph_map.insert("tot_actions".to_string(), ph.tot_actions.into());
        ph_map.insert("spec_actions".to_string(), ph.spec_actions.into());
        ph_map.insert("skipped_actions".to_string(), ph.skipped_actions.into());
        ph_map.insert("msgs_created".to_string(), ph.msgs_created.into());
        ph_map.insert("action_list_hash".to_string(), ph.action_list_hash.to_hex_string().into());
        ph_map.insert("tot_msg_size_cells".to_string(), ph.tot_msg_size.cells.0.into());
        ph_map.insert("tot_msg_size_bits".to_string(), ph.tot_msg_size.bits.0.into());
        serialize_field(map, "action", ph_map);
    }
}

fn serialize_bounce_phase(map: &mut Map<String, Value>, ph: Option<&TrBouncePhase>, mode: SerializationMode) {
    let mut ph_map = serde_json::Map::new();
    let (bounce_type, type_name) = match ph {
        Some(TrBouncePhase::Negfunds) => (0, "negFunds"),
        Some(TrBouncePhase::Nofunds(ph)) => {
            ph_map.insert("msg_size_cells".to_string(), ph.msg_size.cells.0.into());
            ph_map.insert("msg_size_bits".to_string(), ph.msg_size.bits.0.into());
            ph_map.insert("req_fwd_fees".to_string(), grams_to_string(&ph.req_fwd_fees.value(), mode).into());
            (1, "noFunds")
        }
        Some(TrBouncePhase::Ok(ph)) => {
            ph_map.insert("msg_size_cells".to_string(), ph.msg_size.cells.0.into());
            ph_map.insert("msg_size_bits".to_string(), ph.msg_size.bits.0.into());
            ph_map.insert("msg_fees".to_string(), grams_to_string(&ph.msg_fees.value(), mode).into());
            ph_map.insert("fwd_fees".to_string(), grams_to_string(&ph.fwd_fees.value(), mode).into());
            (2, "ok")
        }
        None => return
    };
    ph_map.insert("bounce_type".to_string(), bounce_type.into());
    if mode.is_q_server() {
        ph_map.insert("bounce_type_name".to_string(), type_name.into());
    }
    serialize_field(map, "bounce", ph_map);
}

fn serialize_cc(map: &mut Map<String, Value>, prefix: &'static str, cc: &CurrencyCollection, mode: SerializationMode) -> Result<()> {
    map.insert(format!("{}", prefix), grams_to_string(&cc.grams.value(), mode).into());
    let mut other = Vec::new();
    cc.other_as_hashmap().iterate(&mut |ref mut key, ref mut value| -> Result<bool> {
        let key = key.get_next_u32()?.to_string();
        let value = VarUInteger32::construct_from(value)?;
        let value = grams_to_string(&value.value(), mode);
        other.push(serde_json::json!({
            "currency": key,
            "value": value,
        }));
        Ok(true)
    })?;
    if !other.is_empty() {
        map.insert(format!("{}_other", prefix), other.into());
    }
    Ok(())
}

fn serialize_ecc(ecc: &ExtraCurrencyCollection, mode: SerializationMode) -> Result<Value> {
    let mut other = Vec::new();
    ecc.iterate_with_keys(&mut |key: u32, ref mut value| -> Result<bool> {
        let value = grams_to_string(&value.value(), mode);
        other.push(serde_json::json!({
            "currency": key,
            "value": value,
        }));
        Ok(true)
    })?;
    Ok(other.into())
}

fn serialize_intermidiate_address(map: &mut Map<String, Value>, id_str: &'static str, addr: &IntermediateAddress) {
    let addr = match addr {
        IntermediateAddress::Regular(addr) => {
            addr.use_src_bits().to_string()
        },
        IntermediateAddress::Simple(addr) => {
            format!("{}:{:x}", addr.workchain_id, addr.addr_pfx)
        },
        IntermediateAddress::Ext(addr) => {
            format!("{}:{:x}", addr.workchain_id, addr.addr_pfx)
        }
    };
    map.insert(id_str.to_string(), addr.into());
}

fn serialize_envelop_msg(msg: &MsgEnvelope, mode: SerializationMode) -> Value {
    let mut map = Map::new();
    serialize_id(&mut map, "msg_id", Some(&msg.message_cell().repr_hash()));
    serialize_intermidiate_address(&mut map, "cur_addr", &msg.cur_addr());
    serialize_intermidiate_address(&mut map, "next_addr", &msg.next_addr());
    map.insert("fwd_fee_remaining".to_string(), grams_to_string(msg.fwd_fee_remaining().value(), mode).into());
    map.into()
}

fn serialize_in_msg(msg: &InMsg, mode: SerializationMode) -> Result<Value> {
    let mut map = Map::new();
    let (type_, type_name) = match msg {
        InMsg::External(msg) => {
            serialize_id(&mut map, "msg_id", Some(&msg.message_cell().repr_hash()));
            serialize_id(&mut map, "transaction_id", Some(&msg.transaction_cell().repr_hash()));
            (0, "external")
        }
        InMsg::IHR(msg) => {
            serialize_id(&mut map, "msg_id", Some(&msg.message_cell().repr_hash()));
            serialize_id(&mut map, "transaction_id", Some(&msg.transaction_cell().repr_hash()));
            map.insert("ihr_fee".to_string(), grams_to_string(msg.ihr_fee().value(), mode).into());
            serialize_cell(&mut map, "proof_created", Some(msg.proof_created()), false)?;
            (1, "ihr")
        }
        InMsg::Immediatelly(msg) => {
            map.insert("in_msg".to_string(), serialize_envelop_msg(&msg.read_message()?, mode));
            serialize_id(&mut map, "transaction_id", Some(&msg.transaction_cell().repr_hash()));
            map.insert("fwd_fee".to_string(), grams_to_string(msg.fwd_fee.value(), mode).into());
            (2, "immediately")
        }
        InMsg::Final(msg) => {
            map.insert("in_msg".to_string(), serialize_envelop_msg(&msg.read_message()?, mode));
            serialize_id(&mut map, "transaction_id", Some(&msg.transaction_cell().repr_hash()));
            map.insert("fwd_fee".to_string(), grams_to_string(msg.fwd_fee.value(), mode).into());
            (3, "final")
        }
        InMsg::Transit(msg) => {
            map.insert("in_msg".to_string(), serialize_envelop_msg(&msg.read_in_message()?, mode));
            map.insert("out_msg".to_string(), serialize_envelop_msg(&msg.read_out_message()?, mode));
            map.insert("transit_fee".to_string(), grams_to_string(msg.transit_fee.value(), mode).into());
            (4, "transit")
        }
        InMsg::DiscardedFinal(msg) => {
            map.insert("in_msg".to_string(), serialize_envelop_msg(&msg.read_message()?, mode));
            map.insert("transaction_id".to_string(), u64_to_string(&msg.transaction_id(), mode).into());
            map.insert("fwd_fee".to_string(), grams_to_string(msg.fwd_fee.value(), mode).into());
            (5, "discardedFinal")
        }
        InMsg::DiscardedTransit(msg) => {
            map.insert("in_msg".to_string(), serialize_envelop_msg(&msg.read_message()?, mode));
            map.insert("transaction_id".to_string(), u64_to_string(&msg.transaction_id(), mode).into());
            map.insert("fwd_fee".to_string(), grams_to_string(msg.fwd_fee().value(), mode).into());
            serialize_cell(&mut map, "proof_delivered", Some(msg.proof_delivered()), false)?;
            (6, "discardedTransit")
        }
        InMsg::None => (-1, "none")
    };
    map.insert("msg_type".to_string(), type_.into());
    if mode.is_q_server() {
        map.insert("msg_type_name".to_string(), type_name.into());
    }
    Ok(map.into())
}

fn serialize_out_msg(msg: &OutMsg, mode: SerializationMode) -> Result<Value> {
    let mut map = Map::new();
    let (type_, type_name) = match msg {
        OutMsg::External(msg) => {
            serialize_id(&mut map, "msg_id", Some(&msg.message_cell().repr_hash()));
            serialize_id(&mut map, "transaction_id", Some(&msg.transaction_cell().repr_hash()));
            (0, "external")
        }
        OutMsg::Immediately(msg) => {
            map.insert("out_msg".to_string(), serialize_envelop_msg(&msg.read_out_message()?, mode));
            serialize_id(&mut map, "transaction_id", Some(&msg.transaction_cell().repr_hash()));
            map.insert("reimport".to_string(), serialize_in_msg(&msg.read_reimport_message()?, mode)?);
            (1, "immediately")
        }
        OutMsg::New(msg) => {
            map.insert("out_msg".to_string(), serialize_envelop_msg(&msg.read_out_message()?, mode));
            serialize_id(&mut map, "transaction_id", Some(&msg.transaction_cell().repr_hash()));
            (2, "outMsgNew")
        }
        OutMsg::Transit(msg) => {
            map.insert("out_msg".to_string(), serialize_envelop_msg(&msg.read_out_message()?, mode));
            map.insert("imported".to_string(), serialize_in_msg(&msg.read_imported()?, mode)?);
            (3, "transit")
        }
        OutMsg::DequeueImmediately(msg) => {
            map.insert("out_msg".to_string(), serialize_envelop_msg(&msg.read_out_message()?, mode));
            map.insert("reimport".to_string(), serialize_in_msg(&msg.read_reimport_message()?, mode)?);
            (4, "dequeueImmediately")
        }
        OutMsg::Dequeue(msg) => {
            map.insert("out_msg".to_string(), serialize_envelop_msg(&msg.read_out_message()?, mode));
            map.insert("import_block_lt".to_string(), u64_to_string(&msg.import_block_lt(), mode).into());
            (5, "dequeue")
        }
        OutMsg::TransitRequired(msg) => {
            map.insert("out_msg".to_string(), serialize_envelop_msg(&msg.read_out_message()?, mode));
            map.insert("imported".to_string(), serialize_in_msg(&msg.read_imported()?, mode)?);
            (6, "transitRequired")
        }
        OutMsg::DequeueShort(msg) => {
            serialize_id(&mut map, "msg_env_hash", Some(&msg.msg_env_hash));
            map.insert("next_workchain".to_string(), msg.next_workchain.into());
            map.insert("next_addr_pfx".to_string(), u64_to_string(&msg.next_addr_pfx, mode).into());
            map.insert("import_block_lt".to_string(), u64_to_string(&msg.import_block_lt, mode).into());
            (7, "dequeueShort")
        }
        OutMsg::None => (-1, "none")
    };
    map.insert("msg_type".to_string(), type_.into());
    if mode.is_q_server() {
        map.insert("msg_type_name".to_string(), type_name.into());
    }
    Ok(map.into())
}

fn serialize_shard_descr(descr: &ShardDescr, mode: SerializationMode) -> Result<Value> {
    let mut map = Map::new();
    serialize_field(&mut map, "seq_no", descr.seq_no);
    serialize_field(&mut map, "reg_mc_seqno", descr.reg_mc_seqno);
    serialize_field(&mut map, "start_lt", u64_to_string(&descr.start_lt, mode));
    serialize_field(&mut map, "end_lt", u64_to_string(&descr.end_lt, mode));
    serialize_field(&mut map, "root_hash", descr.root_hash.to_hex_string());
    serialize_field(&mut map, "file_hash", descr.file_hash.to_hex_string());
    serialize_field(&mut map, "before_split", descr.before_split);
    serialize_field(&mut map, "before_merge", descr.before_merge);
    serialize_field(&mut map, "want_split", descr.want_split);
    serialize_field(&mut map, "want_merge", descr.want_merge);
    serialize_field(&mut map, "nx_cc_updated", descr.nx_cc_updated);
    serialize_field(&mut map, "gen_utime", descr.gen_utime);
    serialize_field(&mut map, "next_catchain_seqno", descr.next_catchain_seqno);
    serialize_field(&mut map, "next_validator_shard", shard_to_string(descr.next_validator_shard));
    serialize_field(&mut map, "min_ref_mc_seqno", descr.min_ref_mc_seqno);
    serialize_field(&mut map, "flags", descr.flags);
    serialize_cc(&mut map, "fees_collected", &descr.fees_collected, mode)?;
    serialize_cc(&mut map, "funds_created", &descr.funds_created, mode)?;
    match descr.split_merge_at {
        FutureSplitMerge::Split { split_utime, interval } => {
            serialize_field(&mut map, "split_utime", split_utime);
            serialize_field(&mut map, "split_interval", interval);
        },
        FutureSplitMerge::Merge { merge_utime, interval } => {
            serialize_field(&mut map, "merge_utime", merge_utime);
            serialize_field(&mut map, "merge_interval", interval);
        }
        FutureSplitMerge::None => ()
    };
    Ok(map.into())
}

fn serialize_config_proposal_setup(cps: &ConfigProposalSetup) -> Result<Value> {
    let mut map = Map::new();
    serialize_field(&mut map, "min_tot_rounds", cps.min_tot_rounds);
    serialize_field(&mut map, "max_tot_rounds", cps.max_tot_rounds);
    serialize_field(&mut map, "min_wins", cps.min_wins);
    serialize_field(&mut map, "max_losses", cps.max_losses);
    serialize_field(&mut map, "min_store_sec", cps.min_store_sec);
    serialize_field(&mut map, "max_store_sec", cps.max_store_sec);
    serialize_field(&mut map, "bit_price", cps.bit_price);
    serialize_field(&mut map, "cell_price", cps.cell_price);
    Ok(map.into())
}

fn serialize_mandatory_params(mp: &MandatoryParams) -> Result<Value> {
    let mut vector = Vec::new();
    mp.iterate_keys(&mut |n: u32| -> Result<bool> {
        vector.push(n);
        Ok(true)
    })?;
    Ok(vector.into())
}

fn serialize_workchains(wcs: &Workchains) -> Result<Value> {
    let mut vector = Vec::new();
    wcs.iterate_with_keys(&mut |key: u32, wc: WorkchainDescr| -> Result<bool> {
        let mut map = Map::new();
        serialize_field(&mut map, "workchain_id", key);
        serialize_field(&mut map, "enabled_since", wc.enabled_since);
        serialize_field(&mut map, "actual_min_split", wc.actual_min_split());
        serialize_field(&mut map, "min_split", wc.min_split());
        serialize_field(&mut map, "max_split", wc.max_split());
        serialize_field(&mut map, "active", wc.active);
        serialize_field(&mut map, "accept_msgs", wc.accept_msgs);
        serialize_field(&mut map, "flags", wc.flags);
        serialize_uint256(&mut map, "zerostate_root_hash", &wc.zerostate_root_hash);
        serialize_uint256(&mut map, "zerostate_file_hash", &wc.zerostate_file_hash);
        serialize_field(&mut map, "version", wc.version);
        match wc.format {
            WorkchainFormat::Basic(f) => {
                serialize_field(&mut map, "basic", true);
                serialize_field(&mut map, "vm_version" , f.vm_version);
                serialize_field(&mut map, "vm_mode" , f.vm_mode);
            },
            WorkchainFormat::Extended(f) => {
                serialize_field(&mut map, "basic", false);
                serialize_field(&mut map, "min_addr_len", f.min_addr_len());
                serialize_field(&mut map, "max_addr_len", f.max_addr_len());
                serialize_field(&mut map, "addr_len_step", f.addr_len_step());
                serialize_field(&mut map, "workchain_type_id", f.workchain_type_id());
            }
        }
        vector.push(Value::from(map));
        Ok(true)
    })?;
    Ok(vector.into())
}

fn serialize_storage_prices(wcs: &ConfigParam18Map, mode: SerializationMode) -> Result<Value> {
    let mut vector = Vec::new();
    wcs.iterate(&mut |val| {
        let mut map = Map::new();
        serialize_field(&mut map, "utime_since", val.utime_since);
        serialize_field(&mut map, "bit_price_ps", u64_to_string(&val.bit_price_ps, mode));
        serialize_field(&mut map, "cell_price_ps", u64_to_string(&val.cell_price_ps, mode));
        serialize_field(&mut map, "mc_bit_price_ps", u64_to_string(&val.mc_bit_price_ps, mode));
        serialize_field(&mut map, "mc_cell_price_ps", u64_to_string(&val.mc_cell_price_ps, mode));
        vector.push(Value::from(map));
        Ok(true)
    })?;
    Ok(vector.into())
}

fn serialize_gas_limits_prices(map: &mut Map<String, Value>, glp: &GasLimitsPrices, mode: SerializationMode) {
    match glp {
        GasLimitsPrices::Std(gp) => {
            serialize_field(map, "gas_price", u64_to_string(&gp.gas_price, mode));
            serialize_field(map, "gas_limit", u64_to_string(&gp.gas_limit, mode));
            serialize_field(map, "gas_credit", u64_to_string(&gp.gas_credit, mode));
            serialize_field(map, "block_gas_limit", u64_to_string(&gp.block_gas_limit, mode));
            serialize_field(map, "freeze_due_limit", u64_to_string(&gp.freeze_due_limit, mode));
            serialize_field(map, "delete_due_limit", u64_to_string(&gp.delete_due_limit, mode));
        }
        GasLimitsPrices::Ex(gp) => {
            serialize_field(map, "gas_price", u64_to_string(&gp.gas_price, mode));
            serialize_field(map, "gas_limit", u64_to_string(&gp.gas_limit, mode));
            serialize_field(map, "special_gas_limit", u64_to_string(&gp.special_gas_limit, mode));
            serialize_field(map, "gas_credit", u64_to_string(&gp.gas_credit, mode));
            serialize_field(map, "block_gas_limit", u64_to_string(&gp.block_gas_limit, mode));
            serialize_field(map, "freeze_due_limit", u64_to_string(&gp.freeze_due_limit, mode));
            serialize_field(map, "delete_due_limit", u64_to_string(&gp.delete_due_limit, mode));
        }
        GasLimitsPrices::FlatPfx(gp) => {
            serialize_field(map, "flat_gas_limit", u64_to_string(&gp.flat_gas_limit, mode));
            serialize_field(map, "flat_gas_price", u64_to_string(&gp.flat_gas_price, mode));
            serialize_gas_limits_prices(map, &gp.other, mode);
        }
    }
}

fn serialize_params_limits(pl: &ParamLimits) -> Result<Value> {
    let mut map = Map::new();
    serialize_field(&mut map, "underload", pl.underload());
    serialize_field(&mut map, "soft_limit", pl.soft_limit());
    serialize_field(&mut map, "hard_limit", pl.hard_limit());
    Ok(map.into())
}

fn serialize_block_limits(map: &mut Map<String, Value>, bl: &BlockLimits) -> Result<()> {
    serialize_field(map, "bytes", serialize_params_limits(bl.bytes())?);
    serialize_field(map, "gas", serialize_params_limits(bl.gas())?);
    serialize_field(map, "lt_delta", serialize_params_limits(bl.lt_delta())?);
    Ok(())
}

fn serialize_msg_fwd_prices(map: &mut Map<String, Value>, fp: &MsgForwardPrices, mode: SerializationMode) -> Result<()> {
    serialize_field(map, "lump_price", u64_to_string(&fp.lump_price, mode));
    serialize_field(map, "bit_price", u64_to_string(&fp.bit_price, mode));
    serialize_field(map, "cell_price", u64_to_string(&fp.cell_price, mode));
    serialize_field(map, "ihr_price_factor", fp.ihr_price_factor);
    serialize_field(map, "first_frac", fp.first_frac);
    serialize_field(map, "next_frac", fp.next_frac);
    Ok(())
}

fn serialize_fundamental_smc_addresses(addresses: &FundamentalSmcAddresses) -> Result<Value> {
    let mut vector = Vec::<Value>::new();
    addresses.iterate_keys(&mut |k: UInt256| -> Result<bool> {
        vector.push(k.to_hex_string().into());
        Ok(true)
    })?;
    Ok(vector.into())
}

fn serialize_validators_set(map: &mut Map<String, Value>, set: &ValidatorSet, mode: SerializationMode) -> Result<()> {
    serialize_field(map, "utime_since", set.utime_since());
    serialize_field(map, "utime_until", set.utime_until());
    serialize_field(map, "total", set.total());
    serialize_field(map, "main", set.main());
    serialize_field(map, "total_weight", u64_to_string(&set.total_weight(), mode));
    let mut vector = Vec::<Value>::new();
    for v in set.list() {
        let mut map = Map::new();
        serialize_field(&mut map, "public_key", hex::encode(v.public_key.key_bytes()));
        serialize_field(&mut map, "weight", u64_to_string(&v.weight, mode));
        serialize_id(&mut map, "adnl_addr", v.adnl_addr.as_ref());
        vector.push(map.into());
    };
    serialize_field(map, "list", Value::from(vector));
    Ok(())
}

fn serialize_validator_signed_temp_keys(stk: &ValidatorKeys) -> Result<Value> {
    let mut vector = Vec::<Value>::new();
    stk.iterate(&mut |val| -> Result<bool> {
        let mut map = Map::new();
        serialize_uint256(&mut map, "adnl_addr", val.key().adnl_addr());
        serialize_field(&mut map, "temp_public_key", hex::encode(val.key().temp_public_key().key_bytes()));
        serialize_field(&mut map, "seqno", val.key().seqno());
        serialize_field(&mut map, "valid_until", val.key().valid_until());
        let (r, s) = val.signature().to_r_s_bytes();
        serialize_field(&mut map, "signature_r", hex::encode(r));
        serialize_field(&mut map, "signature_s", hex::encode(s));
        vector.push(Value::from(map));
        Ok(true)
    })?;
    Ok(vector.into())
}

fn serialize_crypto_signature(s: &CryptoSignaturePair) -> Result<Value> {
    let mut map = Map::new();
    serialize_uint256(&mut map, "node_id", &s.node_id_short);
    let (r, s) = s.sign.to_r_s_bytes();
    serialize_field(&mut map, "r", hex::encode(r));
    serialize_field(&mut map, "s", hex::encode(s));
    Ok(map.into())
}

fn serialize_known_config_param(number: u32, param: &mut SliceData, mode: SerializationMode) -> Result<Option<Value>> {
    let mut map = Map::new();

    match ConfigParamEnum::construct_from_slice_and_number(param, number)? {
        ConfigParamEnum::ConfigParam0(ref c) => {
            return Ok(Some(c.config_addr.to_hex_string().into()));
        },
        ConfigParamEnum::ConfigParam1(ref c) => {
            return Ok(Some(c.elector_addr.to_hex_string().into()));
        },
        ConfigParamEnum::ConfigParam2(ref c) => {
            return Ok(Some(c.minter_addr.to_hex_string().into()));
        },
        ConfigParamEnum::ConfigParam3(ref c) => {
            return Ok(Some(c.fee_collector_addr.to_hex_string().into()));
        },
        ConfigParamEnum::ConfigParam4(ref c) => {
            return Ok(Some(c.dns_root_addr.to_hex_string().into()));
        },
        ConfigParamEnum::ConfigParam6(ref c) => {
            serialize_field(&mut map, "mint_new_price", grams_to_string(c.mint_new_price.value(), mode));
            serialize_field(&mut map, "mint_add_price", grams_to_string(c.mint_add_price.value(), mode));
        },
        ConfigParamEnum::ConfigParam7(ref c) => {
            return Ok(Some(serialize_ecc(&c.to_mint, mode)?));
        },
        ConfigParamEnum::ConfigParam8(ref c) => {
            serialize_field(&mut map, "version", c.global_version.version);
            serialize_field(&mut map, "capabilities", u64_to_string(&c.global_version.capabilities, mode));
        },
        ConfigParamEnum::ConfigParam9(ref c) => {
            return Ok(Some(serialize_mandatory_params(&c.mandatory_params)?));
        },
        ConfigParamEnum::ConfigParam10(ref c) => {
            return Ok(Some(serialize_mandatory_params(&c.critical_params)?));
        },
        ConfigParamEnum::ConfigParam11(ref c) => {
            serialize_field(&mut map, "normal_params", 
                serialize_config_proposal_setup(&c.read_normal_params()?)?);
            serialize_field(&mut map, "critical_params", 
                serialize_config_proposal_setup(&c.read_critical_params()?)?);
        },
        ConfigParamEnum::ConfigParam12(ref c) => {
            return Ok(Some(serialize_workchains(&c.workchains)?)); 
        },
        ConfigParamEnum::ConfigParam14(ref c) => {
            serialize_field(&mut map, "masterchain_block_fee", 
                grams_to_string(c.block_create_fees.masterchain_block_fee.value(), mode));
            serialize_field(&mut map, "basechain_block_fee", 
                grams_to_string(c.block_create_fees.basechain_block_fee.value(), mode));
        },
        ConfigParamEnum::ConfigParam15(ref c) => {
            serialize_field(&mut map, "validators_elected_for", c.validators_elected_for);
            serialize_field(&mut map, "elections_start_before", c.elections_start_before);
            serialize_field(&mut map, "elections_end_before", c.elections_end_before);
            serialize_field(&mut map, "stake_held_for", c.stake_held_for);
        },
        ConfigParamEnum::ConfigParam16(ref c) => {
            serialize_field(&mut map, "max_validators", c.max_validators.0);
            serialize_field(&mut map, "max_main_validators", c.max_main_validators.0);
            serialize_field(&mut map, "min_validators", c.min_validators.0);
        },
        ConfigParamEnum::ConfigParam17(ref c) => {
            serialize_field(&mut map, "min_stake", grams_to_string(c.min_stake.value(), mode));
            serialize_field(&mut map, "max_stake", grams_to_string(c.max_stake.value(), mode));
            serialize_field(&mut map, "min_total_stake", grams_to_string(c.min_total_stake.value(), mode));
            serialize_field(&mut map, "max_stake_factor", c.max_stake_factor);
        },
        ConfigParamEnum::ConfigParam18(ref c) => {
            return Ok(Some(serialize_storage_prices(&c.map, mode)?));
        },
        ConfigParamEnum::ConfigParam20(ref c) => {
            serialize_gas_limits_prices(&mut map, c, mode);
        },
        ConfigParamEnum::ConfigParam21(ref c) => {
            serialize_gas_limits_prices(&mut map, c, mode);
        },
        ConfigParamEnum::ConfigParam22(ref c) => {
            serialize_block_limits(&mut map, c)?;
        },
        ConfigParamEnum::ConfigParam23(ref c) => {
            serialize_block_limits(&mut map, c)?;
        },
        ConfigParamEnum::ConfigParam24(ref c) => {
            serialize_msg_fwd_prices(&mut map, c, mode)?;
        },
        ConfigParamEnum::ConfigParam25(ref c) => {
            serialize_msg_fwd_prices(&mut map, c, mode)?;
        },
        ConfigParamEnum::ConfigParam28(ref c) => {
            serialize_field(&mut map, "shuffle_mc_validators", c.shuffle_mc_validators);
            serialize_field(&mut map, "mc_catchain_lifetime", c.mc_catchain_lifetime);
            serialize_field(&mut map, "shard_catchain_lifetime", c.shard_catchain_lifetime);
            serialize_field(&mut map, "shard_validators_lifetime", c.shard_validators_lifetime);
            serialize_field(&mut map, "shard_validators_num", c.shard_validators_num);
        },
        ConfigParamEnum::ConfigParam29(ref c) => {
            serialize_field(&mut map, "new_catchain_ids", c.consensus_config.new_catchain_ids);
            serialize_field(&mut map, "round_candidates", c.consensus_config.round_candidates);
            serialize_field(&mut map, "next_candidate_delay_ms", c.consensus_config.next_candidate_delay_ms);
            serialize_field(&mut map, "consensus_timeout_ms", c.consensus_config.consensus_timeout_ms);
            serialize_field(&mut map, "fast_attempts", c.consensus_config.fast_attempts);
            serialize_field(&mut map, "attempt_duration", c.consensus_config.attempt_duration);
            serialize_field(&mut map, "catchain_max_deps", c.consensus_config.catchain_max_deps);
            serialize_field(&mut map, "max_block_bytes", c.consensus_config.max_block_bytes);
            serialize_field(&mut map, "max_collated_bytes", c.consensus_config.max_collated_bytes);
        },
        ConfigParamEnum::ConfigParam31(ref c) => {
            return Ok(Some(serialize_fundamental_smc_addresses(&c.fundamental_smc_addr)?));
        },
        ConfigParamEnum::ConfigParam32(ref c) => {
            serialize_validators_set(&mut map, &c.prev_validators, mode)?;
        },
        ConfigParamEnum::ConfigParam33(ref c) => {
            serialize_validators_set(&mut map, &c.prev_temp_validators, mode)?;
        },
        ConfigParamEnum::ConfigParam34(ref c) => {
            serialize_validators_set(&mut map, &c.cur_validators, mode)?;
        },
        ConfigParamEnum::ConfigParam35(ref c) => {
            serialize_validators_set(&mut map, &c.cur_temp_validators, mode)?;
        },
        ConfigParamEnum::ConfigParam36(ref c) => {
            serialize_validators_set(&mut map, &c.next_validators, mode)?;
        },
        ConfigParamEnum::ConfigParam37(ref c) => {
            serialize_validators_set(&mut map, &c.next_temp_validators, mode)?;
        },
        ConfigParamEnum::ConfigParam39(ref c) => {
            return Ok(Some(serialize_validator_signed_temp_keys(&c.validator_keys)?));
        },
        ConfigParamEnum::ConfigParamAny(_, _) => {
            return Ok(None)
        },
    }

    Ok(Some(map.into()))
}

fn serialize_unknown_config_param(number: u32, param: &mut SliceData) -> Result<Value> {
    let mut map = Map::new();

    map.insert("number".to_string(), number.into());
    serialize_slice(&mut map, "boc", Some(&param), false)?;

    Ok(map.into())
}

pub struct BlockSerializationSet {
    pub block: Block,
    pub id: BlockId,
    pub status: BlockProcessingStatus,
    pub boc: Vec<u8>,
}

pub fn db_serialize_block(id_str: &'static str, set: &BlockSerializationSet) -> Result<Map<String, Value>> {
    db_serialize_block_ex(id_str, set, SerializationMode::Standart)
}

pub fn db_serialize_block_ex(id_str: &'static str, set: &BlockSerializationSet, mode: SerializationMode) -> Result<Map<String, Value>> {
    let mut map = Map::new();
    || -> Result<()> {
        serialize_field(&mut map, "json_version", VERSION);
        serialize_id(&mut map, id_str, Some(&set.id));
        serialize_field(&mut map, "status", set.status as u8);
        if mode.is_q_server() {
            serialize_field(&mut map, "status_name", match set.status {
                BlockProcessingStatus::Unknown => "unknown",
                BlockProcessingStatus::Proposed => "proposed",
                BlockProcessingStatus::Finalized => "finalized",
                BlockProcessingStatus::Refused => "refused",
            });
        }
        map.insert("boc".to_string(), base64::encode(&set.boc).into());
        map.insert("global_id".to_string(), set.block.global_id.into());
        let block_info = set.block.read_info()?;
        map.insert("version".to_string(), block_info.version().into());
        map.insert("after_merge".to_string(), block_info.after_merge().into());
        map.insert("before_split".to_string(), block_info.before_split().into());
        map.insert("after_split".to_string(), block_info.after_split().into());
        map.insert("want_split".to_string(), block_info.want_split().into());
        map.insert("want_merge".to_string(), block_info.want_merge().into());
        map.insert("key_block".to_string(), block_info.key_block().into());
        map.insert("vert_seqno_incr".to_string(), block_info.vert_seqno_incr().into());
        map.insert("seq_no".to_string(), block_info.seq_no().into());
        map.insert("vert_seq_no".to_string(), block_info.vert_seq_no().into());
        map.insert("gen_utime".to_string(), block_info.gen_utime().0.into());
        map.insert("start_lt".to_string(), u64_to_string(&block_info.start_lt(), mode).into());
        map.insert("end_lt".to_string(), u64_to_string(&block_info.end_lt(), mode).into());
        map.insert("gen_validator_list_hash_short".to_string(), block_info.gen_validator_list_hash_short().into());
        map.insert("gen_catchain_seqno".to_string(), block_info.gen_catchain_seqno().into());
        map.insert("min_ref_mc_seqno".to_string(), block_info.min_ref_mc_seqno().into());
        map.insert("prev_key_block_seqno".to_string(), block_info.prev_key_block_seqno().into());
        map.insert("workchain_id".to_string(), block_info.shard().workchain_id().into());
        map.insert("shard".to_string(), block_info.shard().shard_prefix_as_str_with_tag().into());

        if let Some(gs) = block_info.gen_software() {
            serialize_field(&mut map, "gen_software_version", gs.version);
            serialize_field(&mut map, "gen_software_capabilities", u64_to_string(&gs.capabilities, mode));
        }

        let prev_block_ref = block_info.read_prev_ref()?;
        map.insert("prev_seq_no".to_string(), prev_block_ref.prev1()?.seq_no.into());

        let (vert_prev1, vert_prev2) = match &block_info.read_prev_vert_ref()? {
            Some(blk) => (Some(blk.prev1()?), blk.prev2()?),
            None => (None, None)
        };
        [ ("master_ref", block_info.read_master_ref()?.map(|blk| blk.master)),
            ("prev_ref", Some(prev_block_ref.prev1()?)),
            ("prev_alt_ref", prev_block_ref.prev2()?),
            ("prev_vert_ref", vert_prev1),
            ("prev_vert_alt_ref", vert_prev2),
        ].iter().for_each(|(id_str, blk_ref)| if let Some(blk_ref) = blk_ref {
            let mut blk_ref_map = Map::new();
            blk_ref_map.insert("end_lt".to_string(), u64_to_string(&blk_ref.end_lt, mode).into());
            blk_ref_map.insert("seq_no".to_string(), blk_ref.seq_no.into());
            serialize_id(&mut blk_ref_map, "root_hash", Some(&blk_ref.root_hash));
            serialize_id(&mut blk_ref_map, "file_hash", Some(&blk_ref.file_hash));
            map.insert(id_str.to_string(), blk_ref_map.into());
        });
        let value_flow = set.block.read_value_flow()?;
        let mut value_map = Map::new();
        serialize_cc(&mut value_map, "from_prev_blk",  &value_flow.from_prev_blk, mode)?;
        serialize_cc(&mut value_map, "to_next_blk",    &value_flow.to_next_blk, mode)?;
        serialize_cc(&mut value_map, "imported",       &value_flow.imported, mode)?;
        serialize_cc(&mut value_map, "exported",       &value_flow.exported, mode)?;
        serialize_cc(&mut value_map, "fees_collected", &value_flow.fees_collected, mode)?;
        serialize_cc(&mut value_map, "fees_imported",  &value_flow.fees_imported, mode)?;
        serialize_cc(&mut value_map, "recovered",      &value_flow.recovered, mode)?;
        serialize_cc(&mut value_map, "created",        &value_flow.created, mode)?;
        serialize_cc(&mut value_map, "minted",         &value_flow.minted, mode)?;
        map.insert("value_flow".to_string(), value_map.into());

        let state_update = set.block.read_state_update()?;
        serialize_id(&mut map, "old_hash", Some(&state_update.old_hash));
        serialize_id(&mut map, "new_hash", Some(&state_update.new_hash));
        map.insert("old_depth".to_string(), state_update.old_depth.into());
        map.insert("new_depth".to_string(), state_update.new_depth.into());

        let extra = set.block.read_extra()?;
        let mut msgs = vec![];
        extra.read_in_msg_descr()?.iterate(&mut |ref msg| {
            msgs.push(serialize_in_msg(msg, mode)?);
            Ok(true)
        })?;
        map.insert("in_msg_descr".to_string(), msgs.into());

        let mut msgs = vec![];
        extra.read_out_msg_descr()?.iterate(&mut |ref msg| {
            msgs.push(serialize_out_msg(msg, mode)?);
            Ok(true)
        })?;
        map.insert("out_msg_descr".to_string(), msgs.into());
        let mut tr_count = 0;
        let mut account_blocks = Vec::new();
        extra.read_account_blocks()?.iterate(&mut |account_block| {
            let address = MsgAddressInt::with_variant(None, block_info.shard().workchain_id(), account_block.account_addr())?;
            let mut map = Map::new();
            serialize_field(&mut map, "account_addr", address.to_string());
            let mut transactions = Vec::new();
            account_block.transaction_iterate_full(&mut |key, transaction, cc| {
                let mut map = Map::new();
                serialize_field(&mut map, "lt", u64_to_string(&key, mode));
                serialize_id(&mut map, "transaction_id", Some(&transaction.repr_hash()));
                serialize_cc(&mut map, "total_fees", &cc, mode)?;
                transactions.push(map);
                Ok(true)
            })?;
            serialize_field(&mut map, "transactions", transactions);
            let state_update = account_block.read_state_update()?;
            serialize_id(&mut map, "old_hash", Some(&state_update.old_hash));
            serialize_id(&mut map, "new_hash", Some(&state_update.new_hash));
            serialize_field(&mut map, "tr_count", account_block.transaction_count()?);
            account_blocks.push(map);
            tr_count += account_block.transaction_count()?;
            Ok(true)
        })?;
        if !account_blocks.is_empty() {
            serialize_field(&mut map, "account_blocks", account_blocks);
        }
        serialize_field(&mut map, "tr_count", tr_count);

        serialize_id(&mut map, "rand_seed", Some(&extra.rand_seed));
        serialize_id(&mut map, "created_by", Some(&extra.created_by));

        if let Some(master) = extra.read_custom()? {
            let mut master_map = Map::new();
            let mut shard_hashes = Vec::new();
            let mut min_gen_utime = u32::max_value();
            let mut max_gen_utime = 0;
            master.hashes().iterate_with_keys(&mut |key: i32, InRefValue(tree)| {
                let key = key.to_string();
                tree.iterate(&mut |shard, descr| {
                    if let Ok(descr) = serialize_shard_descr(&descr, mode) {
                        shard_hashes.push(serde_json::json!({
                            "workchain_id": key,
                            "shard": shard_to_string(shard_ident_to_u64(shard.cell().data())),
                            "descr": descr,
                        }));
                    }
                    min_gen_utime = std::cmp::min(min_gen_utime, descr.gen_utime);
                    max_gen_utime = std::cmp::max(max_gen_utime, descr.gen_utime);
                    Ok(true)
                })
            })?;
            if !shard_hashes.is_empty() {
                master_map.insert("shard_hashes".to_string(), shard_hashes.into());
                serialize_field(&mut master_map, "min_shard_gen_utime", min_gen_utime);
                serialize_field(&mut master_map, "max_shard_gen_utime", max_gen_utime);
            }
            let mut fees_map = Vec::new();
            master.fees().iterate_slices(&mut |mut key, ref mut shard| {
                let workchain_id = key.get_next_i32()?;
                let shard_prefix = key.get_next_u64()?;
                let shard = ShardFeeCreated::construct_from(shard)?;
                let mut map = Map::new();
                map.insert("workchain_id".to_string(), workchain_id.into());
                map.insert("shard".to_string(), shard_to_string(shard_prefix).into());
                serialize_cc(&mut map, "fees", &shard.fees, mode)?;
                serialize_cc(&mut map, "create", &shard.create, mode)?;
                fees_map.push(map);
                Ok(true)
            })?;
            if !fees_map.is_empty() {
                master_map.insert("shard_fees".to_string(), fees_map.into());
            }
            let mut crypto_signs = vec![];
            master.prev_blk_signatures().iterate(&mut |s| {
                crypto_signs.push(serialize_crypto_signature(&s)?);
                Ok(true)
            })?;
            master_map.insert("prev_blk_signatures".to_string(), crypto_signs.into());
            if let Some(msg) = &master.read_recover_create_msg()? {
                master_map.insert("recover_create_msg".to_string(), serialize_in_msg(msg, mode)?);
            }
            if let Some(msg) = &master.read_mint_msg()? {
                master_map.insert("mint_msg".to_string(), serialize_in_msg(msg, mode)?);
            }
            if let Some(config) = master.config() {
                serialize_id(&mut master_map, "config_addr", Some(&config.config_addr));
                let mut known_cp_map = Map::new();
                let mut unknown_cp_vec = Vec::new();
                config.config_params.iterate(
                    &mut |mut num: SliceData, mut cp_ref: SliceData| -> Result<bool> {
                        println!("key {}", num);
                        let num = num.get_next_u32()?;
                        let mut cp: SliceData = cp_ref.checked_drain_reference()?.into();
                        if let Some(cp) = serialize_known_config_param(num, &mut cp.clone(), mode)? {
                            known_cp_map.insert(format!("p{}", num), cp.into());
                        } else {
                            unknown_cp_vec.push(serialize_unknown_config_param(num, &mut cp)?);
                        }
                        Ok(true)
                    })?;
                serialize_field(&mut master_map, "config", known_cp_map);
                if unknown_cp_vec.len() > 0 {
                    serialize_field(&mut master_map, "unknown_config", unknown_cp_vec);
                }
            }
            map.insert("master".to_string(), master_map.into());
        }
        Ok(())
    }()?;
    Ok(map)
}

pub struct TransactionSerializationSet {
    pub transaction: Transaction,
    pub id: TransactionId,
    pub status: TransactionProcessingStatus,
    pub block_id: Option<BlockId>,
    pub workchain_id: i32,
    pub boc: Vec<u8>,
    pub proof: Option<Vec<u8>>,
}

pub fn db_serialize_transaction(id_str: &'static str, set: &TransactionSerializationSet) -> Result<Map<String, Value>> {
    db_serialize_transaction_ex(id_str, set, SerializationMode::Standart)
}

pub fn db_serialize_transaction_ex(id_str: &'static str, set: &TransactionSerializationSet, mode: SerializationMode) -> Result<Map<String, Value>> {
    let mut map = Map::new();
    || -> Result<()> {
        serialize_field(&mut map, "json_version", VERSION);
        serialize_id(&mut map, id_str, Some(&set.id));
        serialize_id(&mut map, "block_id", set.block_id.as_ref());
        if let Some(proof) = &set.proof {
            serialize_field(&mut map, "proof", base64::encode(&proof));
        }
        serialize_field(&mut map, "boc", base64::encode(&set.boc));
        serialize_field(&mut map, "status", set.status as u8);
        if mode.is_q_server() {
            serialize_field(&mut map, "status_name", match set.status {
                TransactionProcessingStatus::Unknown => "unknown",
                TransactionProcessingStatus::Preliminary => "preliminary",
                TransactionProcessingStatus::Proposed => "proposed",
                TransactionProcessingStatus::Finalized => "finalized",
                TransactionProcessingStatus::Refused => "refused",
            });
        }
        let (tr_type, tr_type_name) = match &set.transaction.read_description()? {
            TransactionDescr::Ordinary(tr) => {
                serialize_storage_phase(&mut map, tr.storage_ph.as_ref(), mode);
                serialize_credit_phase(&mut map, tr.credit_ph.as_ref(), mode)?;
                serialize_compute_phase(&mut map, Some(&tr.compute_ph), mode);
                serialize_action_phase(&mut map, tr.action.as_ref(), mode);
                serialize_bounce_phase(&mut map, tr.bounce.as_ref(), mode);
                serialize_field(&mut map, "credit_first", tr.credit_first);
                serialize_field(&mut map, "aborted", tr.aborted);
                serialize_field(&mut map, "destroyed", tr.destroyed);
                (0b0000, "ordinary")
            }
            TransactionDescr::Storage(tr) => {
                serialize_storage_phase(&mut map, Some(&tr), mode);
                (0b0001, "storage")
            }
            TransactionDescr::TickTock(tr) => {
                serialize_storage_phase(&mut map, Some(&tr.storage), mode);
                serialize_compute_phase(&mut map, Some(&tr.compute_ph), mode);
                serialize_action_phase(&mut map, tr.action.as_ref(), mode);
                serialize_field(&mut map, "aborted", tr.aborted);
                serialize_field(&mut map, "destroyed", tr.destroyed);
                match &tr.tt {
                    TransactionTickTock::Tick => (0b0010, "tick"),
                    TransactionTickTock::Tock => (0b0011, "tock"),
                }
            }
            TransactionDescr::SplitPrepare(tr) => {
                serialize_split_info(&mut map, &tr.split_info);
                serialize_compute_phase(&mut map, Some(&tr.compute_ph), mode);
                serialize_action_phase(&mut map, tr.action.as_ref(), mode);
                serialize_field(&mut map, "aborted", tr.aborted);
                serialize_field(&mut map, "destroyed", tr.destroyed);
                (0b0100, "splitPrepare")
            }
            TransactionDescr::SplitInstall(tr) => {
                serialize_split_info(&mut map, &tr.split_info);
                serialize_id(&mut map, "prepare_transaction", tr.prepare_transaction.hash().ok().as_ref());
                serialize_field(&mut map, "installed", tr.installed);
                (0b0101, "splitInstall")
            }
            TransactionDescr::MergePrepare(tr) => {
                serialize_split_info(&mut map, &tr.split_info);
                serialize_storage_phase(&mut map, Some(&tr.storage_ph), mode);
                serialize_field(&mut map, "aborted", tr.aborted);
                (0b0110, "mergePrepare")
            }
            TransactionDescr::MergeInstall(tr) => {
                serialize_split_info(&mut map, &tr.split_info);
                serialize_id(&mut map, "prepare_transaction", tr.prepare_transaction.hash().ok().as_ref());
                serialize_credit_phase(&mut map, tr.credit_ph.as_ref(), mode)?;
                serialize_compute_phase(&mut map, Some(&tr.compute_ph), mode);
                serialize_action_phase(&mut map, tr.action.as_ref(), mode);
                serialize_field(&mut map, "aborted", tr.aborted);
                serialize_field(&mut map, "destroyed", tr.destroyed);
                (0b0111, "mergeInstall")
            }
        };
        serialize_field(&mut map, "tr_type", tr_type);
        if mode.is_q_server() {
            serialize_field(&mut map, "tr_type_name", tr_type_name);
        }
        serialize_field(&mut map, "lt", u64_to_string(&set.transaction.lt, mode));
        serialize_id(&mut map, "prev_trans_hash", Some(&set.transaction.prev_trans_hash));
        serialize_field(&mut map, "prev_trans_lt", u64_to_string(&set.transaction.prev_trans_lt, mode));
        serialize_field(&mut map, "now", set.transaction.now);
        serialize_field(&mut map, "outmsg_cnt", set.transaction.outmsg_cnt);
        serialize_account_status(&mut map, "orig_status", &set.transaction.orig_status, mode);
        serialize_account_status(&mut map, "end_status", &set.transaction.end_status, mode);
        if let Some(msg) = &set.transaction.in_msg {
            serialize_id(&mut map, "in_msg", Some(&msg.hash()));
        }
        let mut out_ids = vec![];
        set.transaction.out_msgs.iterate_slices(&mut |slice| {
            if let Ok(cell) = slice.reference(0) {
                out_ids.push(cell.repr_hash().to_hex_string());
            }
            Ok(true)
        })?;
        serialize_field(&mut map, "out_msgs", out_ids);
        let account_addr = match set.workchain_id / 128 {
            0 => MsgAddressInt::with_standart(None, set.workchain_id as i8, set.transaction.account_addr.clone())?,
            _ => MsgAddressInt::with_variant(None, set.workchain_id, set.transaction.account_addr.clone())?
        };
        serialize_field(&mut map, "account_addr", account_addr.to_string());
        serialize_field(&mut map, "workchain_id", account_addr.get_workchain_id());
        serialize_cc(&mut map, "total_fees", &set.transaction.total_fees, mode)?;
        let state_update = set.transaction.state_update.read_struct()?;
        serialize_id(&mut map, "old_hash", Some(&state_update.old_hash));
        serialize_id(&mut map, "new_hash", Some(&state_update.new_hash));
        Ok(())
    }()?;
    Ok(map)
}

fn serialize_account_status(map: &mut Map<String, Value>, name: &'static str, status: &AccountStatus, mode: SerializationMode) {
    serialize_field(map, name, match status {
        AccountStatus::AccStateUninit   => 0b00,
        AccountStatus::AccStateFrozen   => 0b10,
        AccountStatus::AccStateActive   => 0b01,
        AccountStatus::AccStateNonexist => 0b11,
    });

    if mode.is_q_server() {
        let name = format!("{}_name", name);
        serialize_field(map, &name, match status {
            AccountStatus::AccStateUninit   => "uninit",
            AccountStatus::AccStateFrozen   => "frozen",
            AccountStatus::AccStateActive   => "active",
            AccountStatus::AccStateNonexist => "nonExist",
        });
    }
}

pub struct AccountSerializationSet {
    pub account: Account,
    pub boc: Vec<u8>,
    pub proof: Option<Vec<u8>>,
}

pub fn db_serialize_account(id_str: &'static str, set: &AccountSerializationSet) -> Result<Map<String, Value>> {
    db_serialize_account_ex(id_str, set, SerializationMode::Standart)
}

pub fn db_serialize_account_ex(id_str: &'static str, set: &AccountSerializationSet, mode: SerializationMode) -> Result<Map<String, Value>> {
    let mut map = Map::new();
    serialize_field(&mut map, "json_version", VERSION);
    match set.account.stuff() {
        Some(stuff) => {
            serialize_field(&mut map, id_str, stuff.addr.to_string());
            serialize_field(&mut map, "workchain_id", stuff.addr.get_workchain_id());
            if let Some(proof) = &set.proof {
                serialize_field(&mut map, "proof", base64::encode(&proof));
            }
            serialize_field(&mut map, "boc", base64::encode(&set.boc));
            serialize_field(&mut map, "last_paid", stuff.storage_stat.last_paid);
            stuff.storage_stat.due_payment.as_ref().map(|grams|
                serialize_field(&mut map, "due_payment", grams_to_string(grams.value(), mode)));
            serialize_field(&mut map, "last_trans_lt", u64_to_string(&stuff.storage.last_trans_lt, mode));
            serialize_cc(&mut map, "balance", &stuff.storage.balance, mode)?;
            if let AccountState::AccountActive(state) = &stuff.storage.state {
                state.split_depth.as_ref().map(|split_depth| serialize_field(&mut map, "split_depth", split_depth.0));
                state.special.as_ref().map(|special| {
                    serialize_field(&mut map, "tick", special.tick);
                    serialize_field(&mut map, "tock", special.tock);
                });
                serialize_cell(&mut map, "code", state.code.as_ref(), true)?;
                serialize_cell(&mut map, "data", state.data.as_ref(), true)?;
                serialize_cell(&mut map, "library", state.library.as_ref(), true)?;
            }
        }
        None => unimplemented!("Attempt to call serde::Serialize::serialize for AccountNone")
    }
    serialize_account_status(&mut map, "acc_type", &set.account.status(), mode);
    Ok(map)
}

pub struct MessageSerializationSet {
    pub message: Message,
    pub id: MessageId,
    pub block_id: Option<UInt256>,
    pub transaction_id: Option<UInt256>,
    pub transaction_now: Option<u32>,
    pub status: MessageProcessingStatus,
    pub boc: Vec<u8>,
    pub proof: Option<Vec<u8>>,
}

pub fn db_serialize_message(id_str: &'static str, set: &MessageSerializationSet) -> Result<Map<String, Value>> {
    db_serialize_message_ex(id_str, set, SerializationMode::Standart)
}

pub fn db_serialize_message_ex(id_str: &'static str, set: &MessageSerializationSet, mode: SerializationMode) -> Result<Map<String, Value>> {
    let mut map = Map::new();
    || -> Result<()> {
        serialize_field(&mut map, "json_version", VERSION);
        serialize_id(&mut map, id_str, Some(&set.id));
        // isn't needed there - because message should be fully immutable from source block to destination one
        //serialize_id(&mut map, "block_id", set.block_id.as_ref()); 
        serialize_id(&mut map, "transaction_id", set.transaction_id.as_ref());
        if let Some(proof) = &set.proof {
            serialize_field(&mut map, "proof", base64::encode(&proof));
        }
        serialize_field(&mut map, "boc", base64::encode(&set.boc));
        serialize_field(&mut map, "status", set.status as u8);
        if mode.is_q_server() {
            serialize_field(&mut map, "status_name", match set.status {
                MessageProcessingStatus::Unknown => "unknown",
                MessageProcessingStatus::Queued => "queued",
                MessageProcessingStatus::Processing => "processing",
                MessageProcessingStatus::Preliminary => "preliminary",
                MessageProcessingStatus::Proposed => "proposed",
                MessageProcessingStatus::Finalized => "finalized",
                MessageProcessingStatus::Refused => "refused",
                MessageProcessingStatus::Transiting => "transiting",
            });
        }
        if let Some(state) = &set.message.state_init() {
            state.split_depth.as_ref().map(|split_depth| serialize_field(&mut map, "split_depth", split_depth.0));
            state.special.as_ref().map(|special| {
                serialize_field(&mut map, "tick", special.tick);
                serialize_field(&mut map, "tock", special.tock);
            });
            serialize_cell(&mut map, "code", state.code.as_ref(), true)?;
            serialize_cell(&mut map, "data", state.data.as_ref(), true)?;
            serialize_cell(&mut map, "library", state.library.as_ref(), true)?;
        }

        serialize_slice(&mut map, "body", set.message.body().as_ref(), true)?;
        match set.message.header() {
            CommonMsgInfo::IntMsgInfo(ref header) => {
                serialize_field(&mut map, "msg_type", 0);
                if mode.is_q_server() {
                    serialize_field(&mut map, "msg_type_name", "internal");
                }
                serialize_field(&mut map, "src", header.src.to_string());
                if let MsgAddressIntOrNone::Some(src_addr) = &header.src {
                    serialize_field(&mut map, "src_workchain_id", src_addr.get_workchain_id());
                }
                serialize_field(&mut map, "dst", header.dst.to_string());
                serialize_field(&mut map, "dst_workchain_id", header.dst.get_workchain_id());
                serialize_field(&mut map, "ihr_disabled", header.ihr_disabled);
                serialize_field(&mut map, "ihr_fee", grams_to_string(&header.ihr_fee.value(), mode));
                serialize_field(&mut map, "fwd_fee", grams_to_string(&header.fwd_fee.value(), mode));
                serialize_field(&mut map, "bounce", header.bounce);
                serialize_field(&mut map, "bounced", header.bounced);
                serialize_cc(&mut map, "value", &header.value, mode)?;
                serialize_field(&mut map, "created_lt", u64_to_string(&header.created_lt, mode));
                serialize_field(&mut map, "created_at", header.created_at.0);
            }
            CommonMsgInfo::ExtInMsgInfo(ref header) => {
                serialize_field(&mut map, "msg_type", 1);
                if mode.is_q_server() {
                    serialize_field(&mut map, "msg_type_name", "extIn");
                }
                serialize_field(&mut map, "src", header.src.to_string());
                serialize_field(&mut map, "dst", header.dst.to_string());
                serialize_field(&mut map, "dst_workchain_id", header.dst.get_workchain_id());
                serialize_field(&mut map, "import_fee", grams_to_string(&header.import_fee.value(), mode));
                if let Some(now) = set.transaction_now {
                    serialize_field(&mut map, "created_at", now);
                }
            }
            CommonMsgInfo::ExtOutMsgInfo(ref header) => {
                serialize_field(&mut map, "msg_type", 2);
                if mode.is_q_server() {
                    serialize_field(&mut map, "msg_type_name", "extOut");
                }
                serialize_field(&mut map, "src", header.src.to_string());
                if let MsgAddressIntOrNone::Some(src_addr) = &header.src {
                    serialize_field(&mut map, "src_workchain_id", src_addr.get_workchain_id());
                }
                serialize_field(&mut map, "dst", header.dst.to_string());
                serialize_field(&mut map, "created_lt", u64_to_string(&header.created_lt, mode));
                serialize_field(&mut map, "created_at", header.created_at.0);
            }
        }
        Ok(())
    }()?;
    Ok(map)
}

pub fn db_serialize_block_signatures(
    id_str: &'static str,
    block_id: &UInt256,
    signatures_set: &[CryptoSignaturePair]
) -> Result<Map<String, Value>> {

    let mut map = Map::new();
    let mut signs = Vec::new();
    serialize_field(&mut map, "json_version", VERSION);
    serialize_uint256(&mut map, id_str, block_id);
    for s in signatures_set.iter() {
        signs.push(serialize_crypto_signature(s)?);
    }
    serialize_field(&mut map, "signatures", signs);
    Ok(map)
}

pub fn db_serialize_block_proof(
    id_str: &'static str,
    proof: &BlockProof,
) -> Result<Map<String, Value>> {
    db_serialize_block_proof_ex(id_str, proof, SerializationMode::Standart)
}

pub fn db_serialize_block_proof_ex(
    id_str: &'static str,
    proof: &BlockProof,
    mode: SerializationMode,
) -> Result<Map<String, Value>> {

    let mut map = Map::new();

    serialize_field(&mut map, "json_version", VERSION);
    serialize_uint256(&mut map, id_str, &proof.proof_for.root_hash);

    let merkle_proof = MerkleProof::construct_from(&mut proof.root.clone().into())?;
    let block_virt_root = merkle_proof.proof.clone().virtualize(1);
    let virt_block = Block::construct_from(&mut block_virt_root.into())?;
    let block_info = virt_block.read_info()?;

    map.insert("gen_utime".to_string(), block_info.gen_utime().0.into());
    map.insert("seq_no".to_string(), block_info.seq_no().into());
    map.insert("workchain_id".to_string(), block_info.shard().workchain_id().into());
    map.insert("shard".to_string(), block_info.shard().shard_prefix_as_str_with_tag().into());
    serialize_cell(&mut map, "proof", Some(&proof.root), false)?;

    if let Some(signatures) = proof.signatures.as_ref() {
        map.insert("validator_list_hash_short".to_string(), signatures.validator_info.validator_list_hash_short.into());
        map.insert("catchain_seqno".to_string(), signatures.validator_info.catchain_seqno.into());
        map.insert("sig_weight".to_string(), u64_to_string(&signatures.pure_signatures.weight(), mode).into());

        let mut signs = Vec::new();
        signatures
           .pure_signatures
           .signatures()
           .iterate(&mut |_key, mut value| -> Result<bool> {
                signs.push(
                    serialize_crypto_signature(
                        &CryptoSignaturePair::construct_from(&mut value)?
                    )?
                );
                Ok(true)
           }
       )?;
       serialize_field(&mut map, "signatures", signs);
    } 
    Ok(map)
}

