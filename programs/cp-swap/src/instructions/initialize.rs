use std::ops::Deref;

use crate::curve::CurveCalculator;
use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::*;
use crate::USDC;
use anchor_lang::{
    accounts::interface_account::InterfaceAccount, prelude::*, solana_program::clock,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_2022::spl_token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
#[event_cpi]
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Address paying to create the pool. Can be anyone
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Which config the pool belongs to.
    pub amm_config: Box<Account<'info, AmmConfig>>,

    /// CHECK: pool vault and lp mint authority
    #[account(
        seeds = [
            crate::AUTH_SEED.as_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// Initialize an account to store the pool state
    #[account(
        init,
        seeds = [
            POOL_SEED.as_bytes(),
            mint.key().as_ref(),
        ],
        bump,
        payer = creator,
        space = PoolState::LEN
    )]
    pub pool_state: AccountLoader<'info, PoolState>,

    /// Token_0 mint, the key must smaller then token_1 mint.
    #[account(
        mint::token_program = mint_token_program,
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token_1 mint, the key must grater then token_0 mint.
    #[account(
        address = USDC,
        mint::token_program = token_program,
    )]
    pub usdc: Box<InterfaceAccount<'info, Mint>>,

    /// payer token0 account
    #[account(
        mut,
        token::mint = mint,
        token::authority = creator,
    )]
    pub creator_token_mint: Box<InterfaceAccount<'info, TokenAccount>>,

    /// creator token1 account
    #[account(
        mut,
        token::mint = usdc,
        token::authority = creator,
    )]
    pub creator_token_usdc: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Token_0 vault for the pool
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            mint.key().as_ref()
        ],
        bump,
    )]
    pub token_mint_vault: UncheckedAccount<'info>,

    /// CHECK: Token_1 vault for the pool
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            usdc.key().as_ref()
        ],
        bump,
    )]
    pub token_usdc_vault: UncheckedAccount<'info>,

    /// an account to store oracle observations
    // #[account(
    //     init,
    //     seeds = [
    //         OBSERVATION_SEED.as_bytes(),
    //         pool_state.key().as_ref(),
    //     ],
    //     bump,
    //     payer = creator,
    //     space = ObservationState::LEN
    // )]
    // pub observation_state: AccountLoader<'info, ObservationState>,

    /// Program to create mint account and mint tokens
    pub token_program: Program<'info, Token>,
    /// Spl token program or token program 2022
    pub mint_token_program: Interface<'info, TokenInterface>,
    /// Program to create an ATA for receiving position NFT
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// To create a new program account
    pub system_program: Program<'info, System>,
    /// Sysvar for program account
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize(
    ctx: Context<Initialize>,
    mint_amount: u64,
    offset: u64,
    mut open_time: u64,
) -> Result<()> {
    if !(is_supported_mint(&ctx.accounts.mint).unwrap()
        && is_supported_mint(&ctx.accounts.usdc).unwrap())
    {
        return err!(ErrorCode::NotSupportMint);
    }

    if ctx.accounts.amm_config.disable_create_pool {
        return err!(ErrorCode::NotApproved);
    }
    let block_timestamp = clock::Clock::get()?.unix_timestamp as u64;
    if open_time <= block_timestamp {
        open_time = block_timestamp + 1;
    }
    // due to stack/heap limitations, we have to create redundant new accounts ourselves.
    create_token_account(
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.creator.to_account_info(),
        &ctx.accounts.token_mint_vault.to_account_info(),
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.mint_token_program.to_account_info(),
        &[&[
            POOL_VAULT_SEED.as_bytes(),
            ctx.accounts.pool_state.key().as_ref(),
            ctx.accounts.mint.key().as_ref(),
            &[ctx.bumps.token_mint_vault][..],
        ][..]],
    )?;

    create_token_account(
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.creator.to_account_info(),
        &ctx.accounts.token_usdc_vault.to_account_info(),
        &ctx.accounts.usdc.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.token_program.to_account_info(),
        &[&[
            POOL_VAULT_SEED.as_bytes(),
            ctx.accounts.pool_state.key().as_ref(),
            ctx.accounts.usdc.key().as_ref(),
            &[ctx.bumps.token_usdc_vault][..],
        ][..]],
    )?;

    // let mut observation_state = ctx.accounts.observation_state.load_init()?;
    // observation_state.pool_id = ctx.accounts.pool_state.key();

    let pool_state = &mut ctx.accounts.pool_state.load_init()?;

    transfer_from_user_to_pool_vault(
        ctx.accounts.creator.to_account_info(),
        ctx.accounts.creator_token_mint.to_account_info(),
        ctx.accounts.token_mint_vault.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.mint_token_program.to_account_info(),
        mint_amount,
        ctx.accounts.mint.decimals,
    )?;

    let token_mint_vault =
        spl_token_2022::extension::StateWithExtensions::<spl_token_2022::state::Account>::unpack(
            ctx.accounts
                .token_mint_vault
                .to_account_info()
                .try_borrow_data()?
                .deref(),
        )?
        .base;

    CurveCalculator::validate_supply(token_mint_vault.amount, offset)?;

    pool_state.initialize(
        offset,
        ctx.bumps.authority,
        open_time,
        ctx.accounts.creator.key(),
        ctx.accounts.amm_config.key(),
        ctx.accounts.token_mint_vault.key(),
        ctx.accounts.token_usdc_vault.key(),
        &ctx.accounts.mint,
        Pubkey::default(),
    );

    emit_cpi!(InitializePool {
        mint: ctx.accounts.mint.key(),
        mint_amount,
        open_time,
        pool_creator: ctx.accounts.creator.key(),
        amm_config: ctx.accounts.amm_config.key(),
        off_set: offset
    });

    Ok(())
}
