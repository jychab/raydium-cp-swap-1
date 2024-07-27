use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use std::ops::{BitAnd, BitOr, BitXor};
/// Seed to derive account address and signature
pub const POOL_SEED: &str = "pool";
pub const POOL_LP_MINT_SEED: &str = "pool_lp_mint";
pub const POOL_VAULT_SEED: &str = "pool_vault";

pub const Q32: u128 = (u32::MAX as u128) + 1; // 2^32

pub enum PoolStatusBitIndex {
    Deposit,
    Withdraw,
    Swap,
}

#[derive(PartialEq, Eq)]
pub enum PoolStatusBitFlag {
    Enable,
    Disable,
}

#[account(zero_copy(unsafe))]
#[repr(packed)]
#[derive(Default, Debug)]
pub struct PoolState {
    /// Which config the pool belongs
    pub amm_config: Pubkey,
    /// pool creator
    pub pool_creator: Pubkey,
    /// Token A
    pub token_mint_vault: Pubkey,
    /// Token B
    pub token_usdc_vault: Pubkey,

    /// Mint information for token A
    pub mint: Pubkey,

    /// token_0 program
    pub mint_token_program: Pubkey,

    /// observation account to store oracle data
    pub observation_key: Pubkey,

    pub auth_bump: u8,
    /// Bitwise representation of the state of the pool
    /// bit0, 1: disable deposit(vaule is 1), 0: normal
    /// bit1, 1: disable withdraw(vaule is 2), 0: normal
    /// bit2, 1: disable swap(vaule is 4), 0: normal
    pub status: u8,

    /// mint0 and mint1 decimals
    pub mint_decimals: u8,

    pub protocol_fees_token_mint: u64,
    pub protocol_fees_token_usdc: u64,
    pub creator_fees_token_mint: u64,
    pub creator_fees_token_usdc: u64,

    /// The timestamp allowed for swap in the pool.
    pub open_time: u64,
    /// recent epoch
    pub recent_epoch: u64,
    pub off_set: u64,
    /// padding for future updates
    pub padding: [u64; 4],
}

impl PoolState {
    pub const LEN: usize = 8 + std::mem::size_of::<PoolState>();

    pub fn initialize(
        &mut self,
        off_set: u64,
        auth_bump: u8,
        open_time: u64,
        pool_creator: Pubkey,
        amm_config: Pubkey,
        token_mint_vault: Pubkey,
        token_usdc_vault: Pubkey,
        mint: &InterfaceAccount<Mint>,
        observation_key: Pubkey,
    ) {
        self.off_set = off_set;
        self.amm_config = amm_config.key();
        self.pool_creator = pool_creator.key();
        self.token_mint_vault = token_mint_vault;
        self.token_usdc_vault = token_usdc_vault;
        self.mint = mint.key();
        self.mint_token_program = *mint.to_account_info().owner;
        self.observation_key = observation_key;
        self.auth_bump = auth_bump;
        self.mint_decimals = mint.decimals;
        self.protocol_fees_token_mint = 0;
        self.protocol_fees_token_usdc = 0;
        self.creator_fees_token_mint = 0;
        self.creator_fees_token_usdc = 0;
        self.open_time = open_time;
        self.recent_epoch = Clock::get().unwrap().epoch;
        self.padding = [0u64; 4];
    }

    pub fn set_status(&mut self, status: u8) {
        self.status = status
    }

    pub fn set_status_by_bit(&mut self, bit: PoolStatusBitIndex, flag: PoolStatusBitFlag) {
        let s = u8::from(1) << (bit as u8);
        if flag == PoolStatusBitFlag::Disable {
            self.status = self.status.bitor(s);
        } else {
            let m = u8::from(255).bitxor(s);
            self.status = self.status.bitand(m);
        }
    }

    /// Get status by bit, if it is `noraml` status, return true
    pub fn get_status_by_bit(&self, bit: PoolStatusBitIndex) -> bool {
        let status = u8::from(1) << (bit as u8);
        self.status.bitand(status) == 0
    }

    pub fn vault_amount_without_fee(&self, mint_vault: u64, usdc_vault: u64) -> (u64, u64) {
        (
            mint_vault
                .checked_sub(self.protocol_fees_token_mint + self.creator_fees_token_mint)
                .unwrap(),
            usdc_vault
                .checked_add(self.off_set)
                .unwrap()
                .checked_sub(self.protocol_fees_token_usdc + self.creator_fees_token_usdc)
                .unwrap(),
        )
    }

    pub fn token_price_x32(&self, vault_0: u64, vault_1: u64) -> (u128, u128, u64) {
        let (token_0_amount, token_1_amount) = self.vault_amount_without_fee(vault_0, vault_1);
        (
            token_1_amount as u128 * Q32 as u128 / token_0_amount as u128,
            token_0_amount as u128 * Q32 as u128 / token_1_amount as u128,
            token_1_amount.checked_sub(self.off_set).unwrap(),
        )
    }
}

#[cfg(test)]
pub mod pool_test {
    use super::*;

    mod pool_status_test {
        use super::*;

        #[test]
        fn get_set_status_by_bit() {
            let mut pool_state = PoolState::default();
            pool_state.set_status(4); // 0000100
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Swap),
                false
            );
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Deposit),
                true
            );
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Withdraw),
                true
            );

            // disable -> disable, nothing to change
            pool_state.set_status_by_bit(PoolStatusBitIndex::Swap, PoolStatusBitFlag::Disable);
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Swap),
                false
            );

            // disable -> enable
            pool_state.set_status_by_bit(PoolStatusBitIndex::Swap, PoolStatusBitFlag::Enable);
            assert_eq!(pool_state.get_status_by_bit(PoolStatusBitIndex::Swap), true);

            // enable -> enable, nothing to change
            pool_state.set_status_by_bit(PoolStatusBitIndex::Swap, PoolStatusBitFlag::Enable);
            assert_eq!(pool_state.get_status_by_bit(PoolStatusBitIndex::Swap), true);
            // enable -> disable
            pool_state.set_status_by_bit(PoolStatusBitIndex::Swap, PoolStatusBitFlag::Disable);
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Swap),
                false
            );

            pool_state.set_status(5); // 0000101
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Swap),
                false
            );
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Deposit),
                false
            );
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Withdraw),
                true
            );

            pool_state.set_status(7); // 0000111
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Swap),
                false
            );
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Deposit),
                false
            );
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Withdraw),
                false
            );

            pool_state.set_status(3); // 0000011
            assert_eq!(pool_state.get_status_by_bit(PoolStatusBitIndex::Swap), true);
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Deposit),
                false
            );
            assert_eq!(
                pool_state.get_status_by_bit(PoolStatusBitIndex::Withdraw),
                false
            );
        }
    }
}
