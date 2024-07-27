use anchor_lang::prelude::*;

#[event]
pub struct InitializePool {
    pub mint: Pubkey,
    pub mint_amount: u64,
    pub open_time: u64,
    pub pool_creator: Pubkey,
    pub amm_config: Pubkey,
    pub off_set: u64,
}

#[event]
pub struct CollectFees {
    pub mint: Pubkey,
    pub creator_mint_fees: u64,
    pub creator_usdc_fees: u64,
    pub protocol_mint_fees: u64,
    pub protocol_usdc_fees: u64,
}

/// Emitted when swap
#[event]
pub struct SwapPriceEvent {
    pub timestamp: u64,
    pub mint: Pubkey,
    pub price: u128,
    pub liquidity_before: u64,
    pub liquidity_after: u64,
    /// cacluate result without transfer fee
    pub input_amount: u64,
    /// cacluate result without transfer fee
    pub output_amount: u64,
    pub buy: bool,
    pub user: Pubkey,
}
