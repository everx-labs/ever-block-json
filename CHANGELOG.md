# Release Notes

All notable changes to this project will be documented in this file.

## Version 0.9.30

- Added export pack_processing_info for shard descriptions in masterchain block

## Version 0.9.28

- Fixed for clippy

## Version 0.9.24

- Additional SMFT params

## Version 0.9.23

- Add fast finality params serialization

## Version 0.9.16

- SMFT config parameters

## Version 0.9.3

- Added support of fast finality data structures

## Version 0.9.0

- Use modern crates anyhow and thiserror instead of failure

## Version 0.8.7

- Mesh data structures

## Version 0.8.5

- Removed extra crate base64

## Version 0.8.4

- Common Message support

## Version 0.8.0

- the crate was renamed from `ever_block_json` to `ever_block_json`
- supported renaming of other crates

## Version 0.7.212

- README file added

## Version 0.7.196

- BLS structures support

## Version 0.7.189

### Fixed

- Block parser: parsing was failed if block had empty shard state update.
- Block parser: optimized deserialization accounts from merkle update.  

## Version 0.7.188

- Added block parser  implements common block parsing strategy (with accounts, transactions, messages etc.).
  It is a generalized parsing algorithm based on three sources (ever-node, parser-service, evernode-se). 

## Version 0.7.179

- Parse block proof

## Version 0.7.170

- Pruned cells are not serialized to BOC, only hash is written

## Version 0.7.120

- Parse config parameter 44

## Version 0.7.118

- Supported new enum variant "ComputeSkipReason::Suspended"

## Version 0.7.109

- Supported ever-types version 2.0