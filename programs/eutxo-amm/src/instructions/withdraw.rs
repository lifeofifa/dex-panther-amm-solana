use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::errors::AmmError;
use crate::math;
use crate::state::Pool;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub withdrawer: Signer<'info>,

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

    #[account(mut, address = pool.lp_mint)]
    pub lp_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint_a,
        token::authority = withdrawer,
    )]
    pub withdrawer_token_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint_b,
        token::authority = withdrawer,
    )]
    pub withdrawer_token_b: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = lp_mint,
        token::authority = withdrawer,
    )]
    pub withdrawer_lp_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub lp_token_program: Interface<'info, TokenInterface>,
}

/// Withdraw is intentionally NOT gated by `pool.paused`. If a pool is ever
/// paused (e.g. suspected exploit, emergency response), LPs must still be
/// able to exit with their funds — pausing blocks new swaps/deposits, not
/// the ability to leave. This is a deliberate safety property, not an
/// oversight: never build a pause mechanism that can trap user funds.
pub fn handler(
    ctx: Context<Withdraw>,
    lp_amount: u64,
    min_amount_a_out: u64,
    min_amount_b_out: u64,
) -> Result<()> {
    let reserve_a = ctx.accounts.vault_a.amount;
    let reserve_b = ctx.accounts.vault_b.amount;
    let lp_supply = ctx.accounts.lp_mint.supply;

    let (amount_a, amount_b) =
        math::get_withdraw_amounts(lp_amount, lp_supply, reserve_a, reserve_b)?;

    require!(amount_a >= min_amount_a_out, AmmError::SlippageExceeded);
    require!(amount_b >= min_amount_b_out, AmmError::SlippageExceeded);

    // Burn the withdrawer's LP tokens first. Authority is the withdrawer
    // themselves (they own the LP tokens being burned), not the pool PDA.
    token_interface::burn(
        CpiContext::new(
            ctx.accounts.lp_token_program.key(),
            Burn {
                mint: ctx.accounts.lp_mint.to_account_info(),
                from: ctx.accounts.withdrawer_lp_token.to_account_info(),
                authority: ctx.accounts.withdrawer.to_account_info(),
            },
        ),
        lp_amount,
    )?;

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
                    to: ctx.accounts.withdrawer_token_a.to_account_info(),
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
                    to: ctx.accounts.withdrawer_token_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[pool_seeds],
            ),
            amount_b,
            ctx.accounts.mint_b.decimals,
        )?;
    }

    msg!(
        "Withdraw complete: lp_burned={}, amount_a={}, amount_b={}",
        lp_amount,
        amount_a,
        amount_b
    );

    Ok(())
}
