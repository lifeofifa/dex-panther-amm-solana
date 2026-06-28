use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::errors::AmmError;
use crate::math;
use crate::state::Pool;

/// Which direction the swap goes. Encoded explicitly rather than inferred
/// from account ordering, so the instruction's behavior is unambiguous
/// from the IDL alone.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum SwapDirection {
    /// Trader sends token A, receives token B.
    AToB,
    /// Trader sends token B, receives token A.
    BToA,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    pub trader: Signer<'info>,

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

    #[account(
        mut,
        token::mint = mint_a,
        token::authority = trader,
    )]
    pub trader_token_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint_b,
        token::authority = trader,
    )]
    pub trader_token_b: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

/// `amount_in`: exact amount the trader is sending.
/// `min_amount_out`: slippage guard — transaction fails if the trader
/// would receive less than this. Clients should set this to
/// `quote * (1 - slippage_tolerance)`.
pub fn handler(
    ctx: Context<Swap>,
    direction: SwapDirection,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<()> {
    require!(!ctx.accounts.pool.paused, AmmError::PoolPaused);

    let fee_bps = ctx.accounts.pool.fee_bps;
    let protocol_fee_share_bps = ctx.accounts.pool.protocol_fee_share_bps;

    let (reserve_in, reserve_out) = match direction {
        SwapDirection::AToB => (ctx.accounts.vault_a.amount, ctx.accounts.vault_b.amount),
        SwapDirection::BToA => (ctx.accounts.vault_b.amount, ctx.accounts.vault_a.amount),
    };

    let quote = math::get_swap_quote(amount_in, reserve_in, reserve_out, fee_bps)?;
    require!(
        quote.amount_out >= min_amount_out,
        AmmError::SlippageExceeded
    );

    // Split the collected fee between protocol and LPs. `fee_amount` was
    // already deducted from the trader's effective input inside
    // `get_swap_quote` — here we only decide bookkeeping for how much of
    // that already-deducted fee is earmarked as protocol revenue versus
    // left in the vault accruing to LPs. The earmarked portion stays in
    // the vault (it arrived as part of amount_in's transfer below); we
    // just track it in `protocol_fees_*` so `collect_protocol_fees` knows
    // how much it may later withdraw without touching LP principal.
    let protocol_fee_cut = (quote.fee_amount as u128)
        .checked_mul(protocol_fee_share_bps as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(math::BPS_DENOMINATOR as u128)
        .ok_or(AmmError::MathOverflow)?;
    let protocol_fee_cut: u64 = protocol_fee_cut
        .try_into()
        .map_err(|_| AmmError::MathOverflow)?;

    let (from_account, to_vault, trader_receives_account, from_vault) = match direction {
        SwapDirection::AToB => (
            ctx.accounts.trader_token_a.to_account_info(),
            ctx.accounts.vault_a.to_account_info(),
            ctx.accounts.trader_token_b.to_account_info(),
            ctx.accounts.vault_b.to_account_info(),
        ),
        SwapDirection::BToA => (
            ctx.accounts.trader_token_b.to_account_info(),
            ctx.accounts.vault_b.to_account_info(),
            ctx.accounts.trader_token_a.to_account_info(),
            ctx.accounts.vault_a.to_account_info(),
        ),
    };

    let (in_mint, in_decimals, out_mint, out_decimals) = match direction {
        SwapDirection::AToB => (
            ctx.accounts.mint_a.to_account_info(),
            ctx.accounts.mint_a.decimals,
            ctx.accounts.mint_b.to_account_info(),
            ctx.accounts.mint_b.decimals,
        ),
        SwapDirection::BToA => (
            ctx.accounts.mint_b.to_account_info(),
            ctx.accounts.mint_b.decimals,
            ctx.accounts.mint_a.to_account_info(),
            ctx.accounts.mint_a.decimals,
        ),
    };

    // Trader -> pool vault. Authority is the trader's own signature.
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: from_account,
                mint: in_mint,
                to: to_vault,
                authority: ctx.accounts.trader.to_account_info(),
            },
        ),
        amount_in,
        in_decimals,
    )?;

    // Pool vault -> trader. Authority is the pool PDA, signed via seeds
    // since the vault's on-chain authority is the pool account itself.
    let mint_a_key = ctx.accounts.pool.mint_a;
    let mint_b_key = ctx.accounts.pool.mint_b;
    let bump = ctx.accounts.pool.bump;
    let pool_seeds: &[&[u8]] = &[
        Pool::POOL_SEED,
        mint_a_key.as_ref(),
        mint_b_key.as_ref(),
        &[bump],
    ];

    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: from_vault,
                mint: out_mint,
                to: trader_receives_account,
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[pool_seeds],
        ),
        quote.amount_out,
        out_decimals,
    )?;

    // Record the protocol's earmarked share of the fee for later
    // collection via `collect_protocol_fees`. This does not move any
    // tokens now — it's bookkeeping against funds already sitting in the
    // vault as part of the input transfer above.
    let pool = &mut ctx.accounts.pool;
    match direction {
        SwapDirection::AToB => {
            pool.protocol_fees_a = pool
                .protocol_fees_a
                .checked_add(protocol_fee_cut)
                .ok_or(AmmError::MathOverflow)?;
        }
        SwapDirection::BToA => {
            pool.protocol_fees_b = pool
                .protocol_fees_b
                .checked_add(protocol_fee_cut)
                .ok_or(AmmError::MathOverflow)?;
        }
    }

    msg!(
        "Swap complete: amount_in={}, amount_out={}, fee={}, protocol_cut={}",
        amount_in,
        quote.amount_out,
        quote.fee_amount,
        protocol_fee_cut
    );

    Ok(())
}
