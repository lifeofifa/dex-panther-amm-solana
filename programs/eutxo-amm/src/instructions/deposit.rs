use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked};

use crate::errors::AmmError;
use crate::math;
use crate::state::Pool;

/// Deposit mode, passed by the client to select paired vs. single-sided.
/// Encoded as an instruction argument rather than two separate instructions
/// so the account context (and therefore the IDL surface) stays identical
/// regardless of mode — simpler clients, simpler CPI callers.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum DepositMode {
    /// Deposit both tokens in the current pool ratio.
    Paired,
    /// Deposit only token A; the missing side is filled via an implicit
    /// internal swap. Only valid if the pool has
    /// `single_sided_deposits_enabled = true`.
    SingleSidedA,
    /// Deposit only token B; mirror of `SingleSidedA`.
    SingleSidedB,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

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

    /// Depositor's token A account. Required even for SingleSidedB deposits
    /// (Anchor needs a fixed account set per instruction); unused token
    /// movement is simply zero in that case.
    #[account(
        mut,
        token::mint = mint_a,
        token::authority = depositor,
    )]
    pub depositor_token_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint_b,
        token::authority = depositor,
    )]
    pub depositor_token_b: InterfaceAccount<'info, TokenAccount>,

    /// Depositor's LP token account (legacy SPL Token, see initialize_pool
    /// for rationale). Created if it doesn't exist yet.
    #[account(
        init_if_needed,
        payer = depositor,
        associated_token::mint = lp_mint,
        associated_token::authority = depositor,
        associated_token::token_program = lp_token_program,
    )]
    pub depositor_lp_token: InterfaceAccount<'info, TokenAccount>,

    /// Token program governing mint_a / mint_b — may be legacy SPL Token
    /// or Token-2022 depending on the pool's configured mints.
    pub token_program: Interface<'info, TokenInterface>,
    /// Token program governing the LP mint specifically — always legacy
    /// SPL Token (see initialize_pool).
    pub lp_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// `amount_a` / `amount_b`: for `Paired`, the amounts the depositor is
/// providing of each. In this MVP the client is expected to have already
/// computed a proportional pair (e.g. via `math::get_proportional_amount`
/// run off-chain against a fresh account fetch) — the instruction does
/// NOT auto-refund excess on one side; it mints LP based on the limiting
/// side per `get_deposit_lp_amount`, so providing a non-proportional pair
/// simply under-mints relative to the larger side rather than erroring.
/// A future version could add a strict-match mode that rejects
/// non-proportional input instead.
///
/// For `SingleSidedA` / `SingleSidedB`, only the corresponding amount is
/// read; the other is ignored.
///
/// `min_lp_out`: slippage guard — transaction fails if computed LP minted
/// would be less than this.
pub fn handler(
    ctx: Context<Deposit>,
    mode: DepositMode,
    amount_a: u64,
    amount_b: u64,
    min_lp_out: u64,
) -> Result<()> {
    let pool = &ctx.accounts.pool;
    require!(!pool.paused, AmmError::PoolPaused);

    let reserve_a = ctx.accounts.vault_a.amount;
    let reserve_b = ctx.accounts.vault_b.amount;
    let lp_supply = ctx.accounts.lp_mint.supply;
    let fee_bps = pool.fee_bps;
    let single_sided_enabled = pool.single_sided_deposits_enabled;

    let (transfer_a, transfer_b, lp_to_mint) = match mode {
        DepositMode::Paired => {
            let lp =
                math::get_deposit_lp_amount(amount_a, amount_b, reserve_a, reserve_b, lp_supply)?;
            (amount_a, amount_b, lp)
        }
        DepositMode::SingleSidedA => {
            require!(single_sided_enabled, AmmError::SingleSidedDepositsDisabled);
            let lp = math::get_single_sided_deposit_lp_amount(
                amount_a, reserve_a, reserve_b, lp_supply, fee_bps,
            )?;
            (amount_a, 0, lp)
        }
        DepositMode::SingleSidedB => {
            require!(single_sided_enabled, AmmError::SingleSidedDepositsDisabled);
            let lp = math::get_single_sided_deposit_lp_amount(
                amount_b, reserve_b, reserve_a, lp_supply, fee_bps,
            )?;
            (0, amount_b, lp)
        }
    };

    require!(lp_to_mint >= min_lp_out, AmmError::SlippageExceeded);

    // Move tokens from depositor into the pool vaults. Only the non-zero
    // side actually transfers; a zero-amount CPI is skipped, since some
    // token programs reject zero-amount transfers outright.
    if transfer_a > 0 {
        token_interface::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.depositor_token_a.to_account_info(),
                    mint: ctx.accounts.mint_a.to_account_info(),
                    to: ctx.accounts.vault_a.to_account_info(),
                    authority: ctx.accounts.depositor.to_account_info(),
                },
            ),
            transfer_a,
            ctx.accounts.mint_a.decimals,
        )?;
    }

    if transfer_b > 0 {
        token_interface::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.depositor_token_b.to_account_info(),
                    mint: ctx.accounts.mint_b.to_account_info(),
                    to: ctx.accounts.vault_b.to_account_info(),
                    authority: ctx.accounts.depositor.to_account_info(),
                },
            ),
            transfer_b,
            ctx.accounts.mint_b.decimals,
        )?;
    }

    // Mint LP tokens to the depositor, signed by the pool PDA via seeds.
    let mint_a_key = ctx.accounts.mint_a.key();
    let mint_b_key = ctx.accounts.mint_b.key();
    let bump = ctx.accounts.pool.bump;
    let pool_seeds: &[&[u8]] = &[Pool::POOL_SEED, mint_a_key.as_ref(), mint_b_key.as_ref(), &[bump]];

    token_interface::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.lp_token_program.key(),
            MintTo {
                mint: ctx.accounts.lp_mint.to_account_info(),
                to: ctx.accounts.depositor_lp_token.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[pool_seeds],
        ),
        lp_to_mint,
    )?;

    msg!(
        "Deposit complete: transfer_a={}, transfer_b={}, lp_minted={}",
        transfer_a,
        transfer_b,
        lp_to_mint
    );

    Ok(())
}
