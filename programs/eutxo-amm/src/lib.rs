use anchor_lang::prelude::*;

mod errors;
mod instructions;
mod math;
mod state;

use instructions::deposit::DepositMode;
use instructions::swap::SwapDirection;

// PLACEHOLDER ID — replace after running `anchor keys list` post-build.
// See the comment in Anchor.toml; this value must match exactly in both
// places or the program will fail to deploy / be unverifiable.
declare_id!("EuTXoAMM11111111111111111111111111111111");

#[program]
pub mod eutxo_amm {
    use super::*;

    /// Create a new pool for a (mint_a, mint_b) pair. Mints must be passed
    /// in canonical order (mint_a < mint_b by pubkey) so a pair maps to
    /// exactly one pool address.
    ///
    /// `fee_bps`: total swap fee in basis points (e.g. 30 = 0.30%).
    /// `protocol_fee_share_bps`: share of that fee routed to protocol
    ///   revenue rather than left for LPs, in bps of the fee itself
    ///   (e.g. 1000 = 10% of the fee).
    /// `single_sided_deposits_enabled`: whether this pool accepts
    ///   one-sided deposits — recommended `true` for thin/bridged-asset
    ///   pools, `false` for deep majors where paired deposits are the norm.
    pub fn initialize_pool(
        ctx: Context<instructions::initialize_pool::InitializePool>,
        fee_bps: u16,
        protocol_fee_share_bps: u16,
        single_sided_deposits_enabled: bool,
    ) -> Result<()> {
        instructions::initialize_pool::handler(
            ctx,
            fee_bps,
            protocol_fee_share_bps,
            single_sided_deposits_enabled,
        )
    }

    /// Deposit liquidity. See `deposit::handler` doc comment for the
    /// semantics of `amount_a` / `amount_b` under each `DepositMode`.
    pub fn deposit(
        ctx: Context<instructions::deposit::Deposit>,
        mode: DepositMode,
        amount_a: u64,
        amount_b: u64,
        min_lp_out: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, mode, amount_a, amount_b, min_lp_out)
    }

    /// Swap exact `amount_in` of one token for the other, failing if the
    /// trader would receive less than `min_amount_out`.
    pub fn swap(
        ctx: Context<instructions::swap::Swap>,
        direction: SwapDirection,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        instructions::swap::handler(ctx, direction, amount_in, min_amount_out)
    }

    /// Burn `lp_amount` LP tokens for a pro-rata share of both reserves.
    /// Never blocked by `pool.paused` — LPs can always exit.
    pub fn withdraw(
        ctx: Context<instructions::withdraw::Withdraw>,
        lp_amount: u64,
        min_amount_a_out: u64,
        min_amount_b_out: u64,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx, lp_amount, min_amount_a_out, min_amount_b_out)
    }

    /// Admin-only: pause or unpause swaps and deposits on a pool.
    pub fn set_paused(
        ctx: Context<instructions::admin::SetPaused>,
        paused: bool,
    ) -> Result<()> {
        instructions::admin::set_paused_handler(ctx, paused)
    }

    /// Admin-only: sweep accrued protocol fee revenue to the configured
    /// fee destination accounts. Only ever touches bookkeeping balances
    /// accrued by past swaps, never LP principal.
    pub fn collect_protocol_fees(
        ctx: Context<instructions::admin::CollectProtocolFees>,
    ) -> Result<()> {
        instructions::admin::collect_protocol_fees_handler(ctx)
    }
}
