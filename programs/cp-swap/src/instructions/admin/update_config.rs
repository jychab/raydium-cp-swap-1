use crate::curve::fees::FEE_RATE_DENOMINATOR_VALUE;
use crate::error::ErrorCode;
use crate::{states::*, PROTOCOL_AUTHORITY};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateAmmConfig<'info> {
    /// The amm config owner or admin
    #[account(address = PROTOCOL_AUTHORITY @ ErrorCode::InvalidOwner)]
    pub owner: Signer<'info>,

    /// Amm config account to be changed
    #[account(mut)]
    pub amm_config: Account<'info, AmmConfig>,
}

pub fn update_amm_config(ctx: Context<UpdateAmmConfig>, param: u8, value: u64) -> Result<()> {
    let amm_config = &mut ctx.accounts.amm_config;
    let match_param = Some(param);
    match match_param {
        Some(0) => update_trade_fee_rate(amm_config, value),
        Some(1) => update_protocol_fee_rate(amm_config, value),
        Some(2) => {
            let new_procotol_owner = *ctx.remaining_accounts.iter().next().unwrap().key;
            set_new_protocol_owner(amm_config, new_procotol_owner)?;
        }
        Some(3) => amm_config.disable_create_pool = if value == 0 { false } else { true },
        _ => return err!(ErrorCode::InvalidInput),
    }

    Ok(())
}

fn update_protocol_fee_rate(amm_config: &mut Account<AmmConfig>, protocol_fee_rate: u64) {
    assert!(protocol_fee_rate <= FEE_RATE_DENOMINATOR_VALUE);
    assert!(protocol_fee_rate <= FEE_RATE_DENOMINATOR_VALUE);
    amm_config.protocol_fee_rate = protocol_fee_rate;
}

fn update_trade_fee_rate(amm_config: &mut Account<AmmConfig>, trade_fee_rate: u64) {
    assert!(trade_fee_rate < FEE_RATE_DENOMINATOR_VALUE);
    amm_config.trade_fee_rate = trade_fee_rate;
}

fn set_new_protocol_owner(amm_config: &mut Account<AmmConfig>, new_owner: Pubkey) -> Result<()> {
    require_keys_neq!(new_owner, Pubkey::default());
    #[cfg(feature = "enable-log")]
    msg!(
        "amm_config, old_protocol_owner:{}, new_owner:{}",
        amm_config.protocol_owner.to_string(),
        new_owner.key().to_string()
    );
    amm_config.protocol_fee_collector = new_owner;
    Ok(())
}
