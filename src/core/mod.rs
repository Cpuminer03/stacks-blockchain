// Copyright (C) 2013-2020 Blocstack PBC, a public benefit corporation
// Copyright (C) 2020 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// This module contains the "main loop" that drives everything
use burnchains::Error as burnchain_error;
use burnchains::{Burnchain, BurnchainHeaderHash};
use chainstate::burn::{BlockHeaderHash, ConsensusHash};
use chainstate::coordinator::comm::CoordinatorCommunication;
use util::log;

pub mod mempool;
pub use self::mempool::MemPoolDB;

// fork set identifier -- to be mixed with the consensus hash (encodes the version)
pub const SYSTEM_FORK_SET_VERSION: [u8; 4] = [23u8, 0u8, 0u8, 0u8];

// p2p network version
pub const PEER_VERSION: u32 = 0x17000000; // 23.0.0.0

// network identifiers
pub const NETWORK_ID_MAINNET: u32 = 0x17000000;
pub const NETWORK_ID_TESTNET: u32 = 0xff000000;

// default port
pub const NETWORK_P2P_PORT: u16 = 6265;

pub const MINING_COMMITMENT_WINDOW: u8 = 6;

// first burnchain block hash
// TODO: update once we know the true first burnchain block
pub const FIRST_BURNCHAIN_CONSENSUS_HASH: ConsensusHash = ConsensusHash([0u8; 20]);
pub const FIRST_BURNCHAIN_BLOCK_HASH: BurnchainHeaderHash = BurnchainHeaderHash([0u8; 32]);
pub const FIRST_BURNCHAIN_BLOCK_HEIGHT: u32 = 0;
pub const FIRST_BURNCHAIN_BLOCK_TIMESTAMP: u64 = 0;

pub const FIRST_STACKS_BLOCK_HASH: BlockHeaderHash = BlockHeaderHash([0u8; 32]);
pub const EMPTY_MICROBLOCK_PARENT_HASH: BlockHeaderHash = BlockHeaderHash([0u8; 32]);

pub const BOOT_BLOCK_HASH: BlockHeaderHash = BlockHeaderHash([0xff; 32]);
pub const BURNCHAIN_BOOT_CONSENSUS_HASH: ConsensusHash = ConsensusHash([0xff; 20]);

pub const CHAINSTATE_VERSION: &'static str = "23.0.0.0";

pub const MICROSTACKS_PER_STACKS: u32 = 1_000_000;

pub const POX_SUNSET_START: u64 = (FIRST_BURNCHAIN_BLOCK_HEIGHT as u64) + 100_000;
pub const POX_SUNSET_END: u64 = POX_SUNSET_START + 400_000;

pub const POX_PREPARE_WINDOW_LENGTH: u32 = 240;
pub const POX_REWARD_CYCLE_LENGTH: u32 = 2000;
/// The maximum amount that PoX rewards can be scaled by.
///  That is, if participation is very low, rewards are:
///      POX_MAXIMAL_SCALING x (rewards with 100% participation)
///  Set a 4x, this implies the lower bound of participation for scaling
///   is 25%
pub const POX_MAXIMAL_SCALING: u128 = 4;
/// This is the amount that PoX threshold adjustments are stepped by.
pub const POX_THRESHOLD_STEPS_USTX: u128 = 10_000 * (MICROSTACKS_PER_STACKS as u128);

pub const POX_MAX_NUM_CYCLES: u8 = 12;

/// Synchronize burn transactions from the Bitcoin blockchain
pub fn sync_burnchain_bitcoin(
    working_dir: &String,
    network_name: &String,
) -> Result<u64, burnchain_error> {
    use burnchains::bitcoin::indexer::BitcoinIndexer;
    let channels = CoordinatorCommunication::instantiate();

    let mut burnchain =
        Burnchain::new(working_dir, &"bitcoin".to_string(), network_name).map_err(|e| {
            error!(
                "Failed to instantiate burn chain driver for {}: {:?}",
                network_name, e
            );
            e
        })?;

    let new_height_res = burnchain.sync::<BitcoinIndexer>(&channels.1, None, None);
    let new_height = new_height_res.map_err(|e| {
        error!(
            "Failed to synchronize Bitcoin chain state for {} in {}",
            network_name, working_dir
        );
        e
    })?;

    Ok(new_height)
}
