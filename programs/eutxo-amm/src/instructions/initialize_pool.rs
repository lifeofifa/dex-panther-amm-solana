use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::errors::AmmError;
use crate::state::Pool;

/// Creates a new pool for the (mint_a, mint_b) pair.
///
/// Mints must be passed in canonical order (mint_a's pubkey bytes less
/// than mint_b's) so that a given pair can only ever have one pool address
/// — this prevents liquidity fragmentation across two pools for the same
/// pair due to argument ordering, and lets clients/Jupiter derive the pool
/// address deterministically without an indexer.
///
/// Token program is generic over both legacy SPL Token and Token-2022 via
/// `TokenInterface` / `token_interface::Mint`, so this same instruction
/// works whether either mint is a legacy SPL mint or a Token-2022 mint
/// (e.g. one carrying transfer-hook or metadata-pointer extensions, which
/// is the expected case for bridged Ergo/Cardano-asset representations).
#[derive(Accounts)]
#[instruction(fee_bps: u16, protocol_fee_share_bps: u16, single_sided_deposits_enabled: bool)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The address that will administer this pool (collect protocol fees,
    /// pause/unpause). In production this MUST be a multisig or DAO
    /// address, never a single hot wallet — pass that address here
    /// explicitly rather than defaulting to `payer`.
    pub admin: SystemAccount<'info>,

    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [Pool::POOL_SEED, mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
    )]
    pub pool: Account<'info, Pool>,

    /// Pool-owned vault for token A. The pool PDA is the authority.
    #[account(
        init,
        payer = payer,
        seeds = [Pool::VAULT_A_SEED, pool.key().as_ref()],
        bump,
        token::mint = mint_a,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub vault_a: InterfaceAccount<'info, TokenAccount>,

    /// Pool-owned vault for token B. The pool PDA is the authority.
    #[account(
        init,
        payer = payer,
        seeds = [Pool::VAULT_B_SEED, pool.key().as_ref()],
        bump,
        token::mint = mint_b,
        token::authority = pool,
        token::token_program = token_program,
    )]
    pub vault_b: InterfaceAccount<'info, TokenAccount>,

    /// LP share mint. Always a plain SPL Token mint (not Token-2022) for
    /// simplicity and maximum downstream compatibility (wallets, explorers,
    /// and Jupiter's LP-token handling all assume legacy SPL for LP
    /// tokens) — this only governs the LP receipt token, not the underlying
    /// pooled assets, which can still be Token-2022 mints.
    #[account(
        init,
        payer = payer,
        seeds = [Pool::LP_MINT_SEED, pool.key().as_ref()],
        bump,
        mint::decimals = 9,
        mint::authority = pool,
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitializePool>,
    fee_bps: u16,
    protocol_fee_share_bps: u16,
    single_sided_deposits_enabled: bool,
) -> Result<()> {
    require!(
        ctx.accounts.mint_a.key() != ctx.accounts.mint_b.key(),
        AmmError::IdenticalMints
    );
    require!(
        ctx.accounts.mint_a.key() < ctx.accounts.mint_b.key(),
        AmmError::MintsNotCanonicalOrder
    );
    require!(fee_bps < 10_000, AmmError::FeeTooHigh);
    require!(
        protocol_fee_share_bps <= 10_000,
        AmmError::ProtocolFeeShareTooHigh
    );

    let pool = &mut ctx.accounts.pool;
    pool.bump = ctx.bumps.pool;
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();
    pool.vault_a = ctx.accounts.vault_a.key();
    pool.vault_b = ctx.accounts.vault_b.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.fee_bps = fee_bps;
    pool.protocol_fee_share_bps = protocol_fee_share_bps;
    pool.protocol_fees_a = 0;
    pool.protocol_fees_b = 0;
    pool.admin = ctx.accounts.admin.key();
    pool.paused = false;
    pool.single_sided_deposits_enabled = single_sided_deposits_enabled;
    pool._reserved = [0u8; 64];

    msg!(
        "Pool initialized: mint_a={}, mint_b={}, fee_bps={}",
        pool.mint_a,
        pool.mint_b,
        pool.fee_bps
    );

    Ok(())
}
