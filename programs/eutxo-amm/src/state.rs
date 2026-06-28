use anchor_lang::prelude::*;

/// Core pool state, stored in a PDA.
/// This account is both the data store and, via PDA signing, the
/// authority over the two reserve token accounts and the LP mint.
#[account]
pub struct Pool {
    /// Bump seed for this pool's PDA, cached to avoid re-deriving on every ix.
    pub bump: u8,

    /// Mint of token A (by convention, store the lexicographically smaller
    /// mint pubkey as token A — enforced at init time — so a given pair
    /// always resolves to exactly one pool address, never two).
    pub mint_a: Pubkey,
    /// Mint of token B.
    pub mint_b: Pubkey,

    /// Pool-owned reserve token account holding token A.
    pub vault_a: Pubkey,
    /// Pool-owned reserve token account holding token B.
    pub vault_b: Pubkey,

    /// Mint for this pool's LP (liquidity provider) share token.
    /// The pool PDA is the mint authority.
    pub lp_mint: Pubkey,

    /// Swap fee in basis points (1 bps = 0.01%). E.g. 30 = 0.30%.
    pub fee_bps: u16,

    /// Protocol fee cut of the swap fee, in basis points of the fee itself
    /// (not of the trade). E.g. 1000 = 10% of the 0.30% fee, i.e. 0.03%
    /// of trade volume, accrues to the protocol; the rest stays in the pool
    /// for LPs.
    pub protocol_fee_share_bps: u16,

    /// Accrued protocol fees, denominated in token A, awaiting withdrawal.
    pub protocol_fees_a: u64,
    /// Accrued protocol fees, denominated in token B, awaiting withdrawal.
    pub protocol_fees_b: u64,

    /// Authority allowed to withdraw protocol fees and pause the pool.
    /// Set to a multisig/DAO address in production, never a single hot key.
    pub admin: Pubkey,

    /// Emergency pause switch. When true, swap and deposit are blocked;
    /// withdraw always remains available so LPs are never locked out
    /// of their own funds.
    pub paused: bool,

    /// Whether this pool allows single-sided deposits (deposit only one
    /// of the two assets). Disabled by default; enabled per-pool by the
    /// admin for thin/bridged-asset pools where requiring a matched pair
    /// would be a bad UX bar for first liquidity.
    pub single_sided_deposits_enabled: bool,

    /// Reserved space for future fields so this account can grow without
    /// a breaking migration. Shrink this as fields are added.
    pub _reserved: [u8; 64],
}

impl Pool {
    /// Anchor account discriminator (8) + all fields above.
    pub const LEN: usize = 8 // discriminator
        + 1   // bump
        + 32  // mint_a
        + 32  // mint_b
        + 32  // vault_a
        + 32  // vault_b
        + 32  // lp_mint
        + 2   // fee_bps
        + 2   // protocol_fee_share_bps
        + 8   // protocol_fees_a
        + 8   // protocol_fees_b
        + 32  // admin
        + 1   // paused
        + 1   // single_sided_deposits_enabled
        + 64; // _reserved

    pub const POOL_SEED: &'static [u8] = b"pool";
    pub const VAULT_A_SEED: &'static [u8] = b"vault_a";
    pub const VAULT_B_SEED: &'static [u8] = b"vault_b";
    pub const LP_MINT_SEED: &'static [u8] = b"lp_mint";
}
