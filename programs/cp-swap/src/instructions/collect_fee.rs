use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::transfer_from_pool_vault_to_user;
use crate::PROTOCOL_AUTHORITY;
use crate::USDC;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Token;
use anchor_spl::token_interface::Mint;
use anchor_spl::token_interface::Token2022;
use anchor_spl::token_interface::TokenAccount;
#[event_cpi]
#[derive(Accounts)]
pub struct CollectFee<'info> {
    /// Only admin or fund_owner can collect fee now
    #[account(mut,
        constraint = (payer.key() == amm_config.protocol_fee_collector || payer.key() == pool_state.load()?.pool_creator) @ErrorCode::InvalidOwner)]
    pub payer: Signer<'info>,

    /// CHECK:
    #[account(address = pool_state.load()?.pool_creator)]
    pub pool_creator: AccountInfo<'info>,

    /// CHECK:
    #[account(address = PROTOCOL_AUTHORITY)]
    pub protocol_owner: AccountInfo<'info>,

    /// CHECK: pool vault and lp mint authority
    #[account(
        seeds = [
            crate::AUTH_SEED.as_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// Pool state stores accumulated protocol fee amount
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,

    /// Amm config account stores fund_owner
    #[account(address = pool_state.load()?.amm_config)]
    pub amm_config: Account<'info, AmmConfig>,

    /// The address that holds pool tokens for token_0
    #[account(
        mut,
        token::authority = pool_state,
        token::mint = vault_mint,
        token::token_program = token_program_2022
    )]
    pub token_mint_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_1
    #[account(
        mut,
        token::authority = pool_state,
        token::mint = vault_usdc_mint,
        token::token_program = token_program
    )]
    pub token_usdc_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The mint of token_0 vault
    #[account(
        address = pool_state.load()?.mint
    )]
    pub vault_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The mint of token_1 vault
    #[account(
        address = USDC
    )]
    pub vault_usdc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The address that receives the collected token_0 fund fees
    #[account(
        mut,
        token::authority = pool_creator,
        token::mint = vault_mint,
        token::token_program = token_program_2022
    )]
    pub recipient_token_mint_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::authority = protocol_owner,
        associated_token::mint = vault_mint,
        associated_token::token_program = token_program_2022
    )]
    pub protocol_token_mint_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The address that receives the collected token_1 fund fees
    #[account(
        mut,
        token::authority = pool_creator,
        token::mint = vault_usdc_mint,
        token::token_program = token_program
    )]
    pub recipient_token_usdc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::authority = protocol_owner,
        associated_token::mint = vault_usdc_mint,
        associated_token::token_program = token_program
    )]
    pub protocol_token_usdc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The SPL program to perform token transfers
    pub token_program: Program<'info, Token>,

    /// The SPL program 2022 to perform token transfers
    pub token_program_2022: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn collect_fee(ctx: Context<CollectFee>) -> Result<()> {
    let creator_amount_0: u64;
    let creator_amount_1: u64;
    let protocol_amount_0: u64;
    let protocol_amount_1: u64;
    let auth_bump;
    let mint;
    {
        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        creator_amount_0 = pool_state.creator_fees_token_mint;
        creator_amount_1 = pool_state.creator_fees_token_usdc;
        protocol_amount_0 = pool_state.protocol_fees_token_mint;
        protocol_amount_1 = pool_state.protocol_fees_token_usdc;
        pool_state.creator_fees_token_mint = 0;
        pool_state.creator_fees_token_usdc = 0;
        pool_state.protocol_fees_token_mint = 0;
        pool_state.protocol_fees_token_usdc = 0;
        auth_bump = pool_state.auth_bump;
        mint = pool_state.mint;
    }

    transfer_from_pool_vault_to_user(
        ctx.accounts.pool_state.to_account_info(),
        ctx.accounts.token_mint_vault.to_account_info(),
        ctx.accounts.recipient_token_mint_account.to_account_info(),
        ctx.accounts.vault_mint.to_account_info(),
        if ctx.accounts.vault_mint.to_account_info().owner == ctx.accounts.token_program.key {
            ctx.accounts.token_program.to_account_info()
        } else {
            ctx.accounts.token_program_2022.to_account_info()
        },
        creator_amount_0,
        ctx.accounts.vault_mint.decimals,
        &[&[crate::AUTH_SEED.as_bytes(), &[auth_bump]]],
    )?;

    transfer_from_pool_vault_to_user(
        ctx.accounts.pool_state.to_account_info(),
        ctx.accounts.token_mint_vault.to_account_info(),
        ctx.accounts.protocol_token_mint_account.to_account_info(),
        ctx.accounts.vault_mint.to_account_info(),
        if ctx.accounts.vault_mint.to_account_info().owner == ctx.accounts.token_program.key {
            ctx.accounts.token_program.to_account_info()
        } else {
            ctx.accounts.token_program_2022.to_account_info()
        },
        protocol_amount_0,
        ctx.accounts.vault_mint.decimals,
        &[&[crate::AUTH_SEED.as_bytes(), &[auth_bump]]],
    )?;

    transfer_from_pool_vault_to_user(
        ctx.accounts.pool_state.to_account_info(),
        ctx.accounts.token_usdc_vault.to_account_info(),
        ctx.accounts.recipient_token_usdc_account.to_account_info(),
        ctx.accounts.vault_usdc_mint.to_account_info(),
        if ctx.accounts.vault_usdc_mint.to_account_info().owner == ctx.accounts.token_program.key {
            ctx.accounts.token_program.to_account_info()
        } else {
            ctx.accounts.token_program_2022.to_account_info()
        },
        creator_amount_1,
        ctx.accounts.vault_usdc_mint.decimals,
        &[&[crate::AUTH_SEED.as_bytes(), &[auth_bump]]],
    )?;

    transfer_from_pool_vault_to_user(
        ctx.accounts.pool_state.to_account_info(),
        ctx.accounts.token_usdc_vault.to_account_info(),
        ctx.accounts.protocol_token_usdc_account.to_account_info(),
        ctx.accounts.vault_usdc_mint.to_account_info(),
        if ctx.accounts.vault_usdc_mint.to_account_info().owner == ctx.accounts.token_program.key {
            ctx.accounts.token_program.to_account_info()
        } else {
            ctx.accounts.token_program_2022.to_account_info()
        },
        protocol_amount_1,
        ctx.accounts.vault_usdc_mint.decimals,
        &[&[crate::AUTH_SEED.as_bytes(), &[auth_bump]]],
    )?;

    emit_cpi!(CollectFees {
        mint: mint,
        creator_mint_fees: creator_amount_0,
        creator_usdc_fees: creator_amount_1,
        protocol_mint_fees: protocol_amount_0,
        protocol_usdc_fees: protocol_amount_1
    });

    Ok(())
}
