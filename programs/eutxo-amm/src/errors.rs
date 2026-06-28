use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Token A mint and token B mint must differ")]
    IdenticalMints,

    #[msg("Mints must be provided in canonical order (mint_a < mint_b)")]
    MintsNotCanonicalOrder,

    #[msg("Fee in basis points must be less than 10000 (100%)")]
    FeeTooHigh,

    #[msg("Protocol fee share in basis points must be less than or equal to 10000")]
    ProtocolFeeShareTooHigh,

    #[msg("Deposit amounts must be greater than zero")]
    ZeroDepositAmount,

    #[msg("Swap input amount must be greater than zero")]
    ZeroSwapAmount,

    #[msg("This pool does not allow single-sided deposits")]
    SingleSidedDepositsDisabled,

    #[msg("Resulting LP token amount is zero — deposit too small relative to pool size")]
    ZeroLpTokensMinted,

    #[msg("Slippage tolerance exceeded: output amount below minimum specified")]
    SlippageExceeded,

    #[msg("Slippage tolerance exceeded: required input above maximum specified")]
    ExcessiveInputRequired,

    #[msg("Pool reserves cannot be zero for this operation")]
    InsufficientLiquidity,

    #[msg("Arithmetic overflow")]
    MathOverflow,

    #[msg("Arithmetic underflow")]
    MathUnderflow,

    #[msg("Pool is currently paused for swaps and deposits")]
    PoolPaused,

    #[msg("LP token amount to burn must be greater than zero")]
    ZeroLpBurnAmount,

    #[msg("Insufficient LP token balance for this withdrawal")]
    InsufficientLpBalance,

    #[msg("Only the pool admin may perform this action")]
    Unauthorized,

    #[msg("No protocol fees available to collect")]
    NoFeesToCollect,

    #[msg("Provided token mint does not match this pool's configured mint")]
    MintMismatch,
}
