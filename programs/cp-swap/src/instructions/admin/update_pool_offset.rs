use crate::{states::*, PROTOCOL_AUTHORITY};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdatePoolOffset<'info> {
    #[account(
        address = PROTOCOL_AUTHORITY
    )]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,
}

pub fn update_pool_offset(ctx: Context<UpdatePoolOffset>, offset: u64) -> Result<()> {
    let mut pool_state = ctx.accounts.pool_state.load_mut()?;
    pool_state.off_set = offset;
    Ok(())
}
