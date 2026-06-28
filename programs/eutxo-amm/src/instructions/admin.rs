use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::errors::AmmError;
use crate::state::Pool;

// ---------------------------------------------------------------------
// set_paused — emergency stop for swaps and deposits. Does NOT block
// withdraw (see withdraw.rs for why). Admin-gated.
// ---------------------------------------------------------------------

#[derive(Accounts)]
pub struct SetPaused<'info> {
    #[account(address = pool.admin @ AmmError::Unauthorized)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [Pool::POOL_SEED, pool.mint_a.as_ref(), pool.mint_b.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,
}

pub fn set_paused_handler(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
    ctx.accounts.pool.paused = paused;
    msg!("Pool paused state set to {}", paused);
    Ok(())
}

// ---------------------------------------------------------------------
// collect_protocol_fees — admin withdraws accrued protocol fee revenue.
// This only ever touches the `protocol_fees_a` / `protocol_fees_b`
// bookkeeping balances, never LP principal, so it cannot drain the pool
// beyond what swaps have actually earmarked as protocol revenue.
// ---------------------------------------------------------------------

#[derive(Accounts)]
pub struct CollectProtocolFees<'info> {
    #[account(address = pool.admin @ AmmError::Unauthorized)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [Pool::POOL_SEED, pool.mint_a.as_ref(), pool.mint_b.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,

    #[account(address = pool.mint_a)]
    pub mint_a: InterfaceAccount<'info, Mint>,
    #[account(address = pool.mint_b)]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(mut, address = pool.vault_a)]
    pub vault_a: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, address = pool.vault_b)]
    pub vault_b: InterfaceAccount<'info, TokenAccount>,

    /// Destination for collected token-A fees. Validated only on mint —
    /// ownership can be any account the admin (e.g. a DAO treasury
    /// multisig) controls.
    #[account(mut, token::mint = mint_a)]
    pub fee_destination_a: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, token::mint = mint_b)]
    pub fee_destination_b: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn collect_protocol_fees_handler(ctx: Context<CollectProtocolFees>) -> Result<()> {
    let amount_a = ctx.accounts.pool.protocol_fees_a;
    let amount_b = ctx.accounts.pool.protocol_fees_b;

    require!(amount_a > 0 || amount_b > 0, AmmError::NoFeesToCollect);

    let mint_a_key = ctx.accounts.pool.mint_a;
    let mint_b_key = ctx.accounts.pool.mint_b;
    let bump = ctx.accounts.pool.bump;
    let pool_seeds: &[&[u8]] = &[
        Pool::POOL_SEED,
        mint_a_key.as_ref(),
        mint_b_key.as_ref(),
        &[bump],
    ];

    if amount_a > 0 {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.vault_a.to_account_info(),
                    mint: ctx.accounts.mint_a.to_account_info(),
                    to: ctx.accounts.fee_destination_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[pool_seeds],
            ),
            amount_a,
            ctx.accounts.mint_a.decimals,
        )?;
    }

    if amount_b > 0 {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.vault_b.to_account_info(),
                    mint: ctx.accounts.mint_b.to_account_info(),
                    to: ctx.accounts.fee_destination_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[pool_seeds],
            ),
            amount_b,
            ctx.accounts.mint_b.decimals,
        )?;
    }

    let pool = &mut ctx.accounts.pool;
    pool.protocol_fees_a = 0;
    pool.protocol_fees_b = 0;

    msg!(
        "Protocol fees collected: amount_a={}, amount_b={}",
        amount_a,
        amount_b
    );

    Ok(())
}
