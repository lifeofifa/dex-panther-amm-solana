use anchor_lang::prelude::*;

mod errors;
mod instructions;
mod math;
mod state;

use instructions::admin::*;
use instructions::deposit::*;
use instructions::initialize_pool::*;
use instructions::swap::*;
use instructions::withdraw::*;

// PLACEHOLDER ID — replace after running `anchor keys list` post-build.
// See the comment in Anchor.toml; this value must match exactly in both
// places or the program will fail to deploy / be unverifiable.
declare_id!("6cmEegP2W9pBaP2CB7ZkyupZxM8NZUwU3NiqSE7ci5iR");

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
        ctx: Context<InitializePool>,
        fee_bps: u16,
        protocol_fee_share_bps: u16,
        single_sided_deposits_enabled: bool,
    ) -> Result<()> {
        instructions::initialize_pool::handler(ctx, fee_bps, protocol_fee_share_bps, single_sided_deposits_enabled)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        mode: DepositMode,
        amount_a: u64,
        amount_b: u64,
        min_lp_out: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, mode, amount_a, amount_b, min_lp_out)
    }

    pub fn swap(
        ctx: Context<Swap>,
        direction: SwapDirection,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        instructions::swap::handler(ctx, direction, amount_in, min_amount_out)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        lp_amount: u64,
        min_amount_a_out: u64,
        min_amount_b_out: u64,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx, lp_amount, min_amount_a_out, min_amount_b_out)
    }

    pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
        instructions::admin::set_paused_handler(ctx, paused)
    }

    pub fn collect_protocol_fees(ctx: Context<CollectProtocolFees>) -> Result<()> {
        instructions::admin::collect_protocol_fees_handler(ctx)
    }
}
