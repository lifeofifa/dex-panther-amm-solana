//! Pure math for the constant-product AMM curve: x * y = k.
//!
//! Deliberately has zero dependency on Anchor's `Context`/account types so
//! it can be unit tested as plain Rust and reasoned about independently of
//! account-validation logic. Every operation uses checked arithmetic and
//! u128 intermediates to avoid overflow when multiplying two u64 reserves.

use crate::errors::AmmError;
use anchor_lang::prelude::*;

pub const BPS_DENOMINATOR: u64 = 10_000;

/// Result of a swap quote: how much of the output token the trader
/// receives, and how much of the fee (in input-token units) is taken
/// before the swap math runs.
pub struct SwapQuote {
    pub amount_out: u64,
    pub fee_amount: u64,
}

/// Computes output amount for a constant-product swap given input amount
/// and current reserves, after deducting `fee_bps` from the input.
///
/// Formula (post-fee):
///   amount_in_after_fee = amount_in * (BPS_DENOMINATOR - fee_bps) / BPS_DENOMINATOR
///   amount_out = (reserve_out * amount_in_after_fee) / (reserve_in + amount_in_after_fee)
///
/// This is the standard Uniswap-v2-style formula: fee is deducted from the
/// input before applying x*y=k, so the fee permanently grows the pool's
/// reserves (and thus LP value) rather than being skimmed from output.
pub fn get_swap_quote(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> Result<SwapQuote> {
    require!(amount_in > 0, AmmError::ZeroSwapAmount);
    require!(
        reserve_in > 0 && reserve_out > 0,
        AmmError::InsufficientLiquidity
    );

    let fee_bps_u64 = fee_bps as u64;
    require!(fee_bps_u64 < BPS_DENOMINATOR, AmmError::FeeTooHigh);

    // Fee taken from the input, in input-token units.
    let fee_amount = mul_div_u64(amount_in, fee_bps_u64, BPS_DENOMINATOR)?;
    let amount_in_after_fee = amount_in
        .checked_sub(fee_amount)
        .ok_or(AmmError::MathUnderflow)?;

    // amount_out = reserve_out * amount_in_after_fee / (reserve_in + amount_in_after_fee)
    let new_reserve_in = (reserve_in as u128)
        .checked_add(amount_in_after_fee as u128)
        .ok_or(AmmError::MathOverflow)?;

    let numerator = (reserve_out as u128)
        .checked_mul(amount_in_after_fee as u128)
        .ok_or(AmmError::MathOverflow)?;

    let amount_out_u128 = numerator
        .checked_div(new_reserve_in)
        .ok_or(AmmError::MathOverflow)?;

    let amount_out: u64 = amount_out_u128
        .try_into()
        .map_err(|_| AmmError::MathOverflow)?;

    // Constant-product invariant must not decrease (it should strictly
    // increase, by the fee amount). This is a defensive check, not the
    // primary correctness argument — it catches arithmetic mistakes above.
    let k_before = (reserve_in as u128)
        .checked_mul(reserve_out as u128)
        .ok_or(AmmError::MathOverflow)?;
    let new_reserve_out = (reserve_out as u128)
        .checked_sub(amount_out as u128)
        .ok_or(AmmError::MathUnderflow)?;
    let k_after = new_reserve_in
        .checked_mul(new_reserve_out)
        .ok_or(AmmError::MathOverflow)?;
    require!(k_after >= k_before, AmmError::MathOverflow);

    Ok(SwapQuote {
        amount_out,
        fee_amount,
    })
}

/// Computes LP tokens to mint for a balanced (paired) deposit.
///
/// First deposit into an empty pool: LP minted = sqrt(amount_a * amount_b).
/// This is the standard Uniswap-v2 bootstrap so that initial LP supply is
/// independent of which two arbitrary token amounts were chosen to seed it.
///
/// Subsequent deposits: LP minted is proportional to the smaller of the two
/// ratios (amount_a / reserve_a, amount_b / reserve_b), which is what a
/// correctly-paired deposit should satisfy exactly; taking the min protects
/// existing LPs if the caller supplies a slightly imbalanced ratio.
pub fn get_deposit_lp_amount(
    amount_a: u64,
    amount_b: u64,
    reserve_a: u64,
    reserve_b: u64,
    lp_supply: u64,
) -> Result<u64> {
    require!(amount_a > 0 && amount_b > 0, AmmError::ZeroDepositAmount);

    if lp_supply == 0 {
        // Bootstrap case: pool is empty.
        let product = (amount_a as u128)
            .checked_mul(amount_b as u128)
            .ok_or(AmmError::MathOverflow)?;
        let lp_amount = integer_sqrt(product);
        let lp_amount_u64: u64 = lp_amount.try_into().map_err(|_| AmmError::MathOverflow)?;
        require!(lp_amount_u64 > 0, AmmError::ZeroLpTokensMinted);
        return Ok(lp_amount_u64);
    }

    require!(
        reserve_a > 0 && reserve_b > 0,
        AmmError::InsufficientLiquidity
    );

    // lp_from_a = lp_supply * amount_a / reserve_a
    let lp_from_a = mul_div_u64(lp_supply, amount_a, reserve_a)?;
    // lp_from_b = lp_supply * amount_b / reserve_b
    let lp_from_b = mul_div_u64(lp_supply, amount_b, reserve_b)?;

    let lp_amount = lp_from_a.min(lp_from_b);
    require!(lp_amount > 0, AmmError::ZeroLpTokensMinted);
    Ok(lp_amount)
}

/// Given a desired deposit amount of one side, computes the matching amount
/// of the other side needed to keep the deposit proportional to current
/// reserves. Used so the client/UI (or a same-tx CPI caller) can quote the
/// paired amount before submitting — the instruction itself still validates
/// against actual reserves at execution time, since reserves can move
/// between quote and execution.
pub fn get_proportional_amount(
    amount_known: u64,
    reserve_known: u64,
    reserve_other: u64,
) -> Result<u64> {
    require!(reserve_known > 0, AmmError::InsufficientLiquidity);
    mul_div_u64(amount_known, reserve_other, reserve_known)
}

/// Computes the (amount_a, amount_b) a withdrawing LP receives for burning
/// `lp_amount` out of `lp_supply` total, pro-rata against current reserves.
pub fn get_withdraw_amounts(
    lp_amount: u64,
    lp_supply: u64,
    reserve_a: u64,
    reserve_b: u64,
) -> Result<(u64, u64)> {
    require!(lp_amount > 0, AmmError::ZeroLpBurnAmount);
    require!(lp_supply > 0, AmmError::InsufficientLiquidity);
    require!(lp_amount <= lp_supply, AmmError::InsufficientLpBalance);

    let amount_a = mul_div_u64(reserve_a, lp_amount, lp_supply)?;
    let amount_b = mul_div_u64(reserve_b, lp_amount, lp_supply)?;
    Ok((amount_a, amount_b))
}

/// For single-sided deposits: computes LP tokens minted when depositing
/// only token A (no paired token B). This is modeled as an implicit swap
/// of half the deposit into token B followed by a balanced deposit, which
/// is the standard way to reason about single-sided liquidity without
/// introducing a separate pricing curve. The fee is still charged on the
/// implicit-swap portion so single-sided depositors don't get a free ride
/// relative to paired depositors, and so they don't dilute existing LPs.
///
/// This is a simplified model suitable for an MVP; production-grade
/// single-sided deposit math (as used by CLMM protocols) is more involved.
/// Treat this function as the clearly-isolated place to upgrade later.
pub fn get_single_sided_deposit_lp_amount(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    lp_supply: u64,
    fee_bps: u16,
) -> Result<u64> {
    require!(amount_in > 0, AmmError::ZeroDepositAmount);
    require!(
        reserve_in > 0 && reserve_out > 0,
        AmmError::InsufficientLiquidity
    );

    // Swap half the input to the other side at current pool price.
    let half_in = amount_in.checked_div(2).ok_or(AmmError::MathUnderflow)?;
    let remaining_in = amount_in
        .checked_sub(half_in)
        .ok_or(AmmError::MathUnderflow)?;

    let quote = get_swap_quote(half_in, reserve_in, reserve_out, fee_bps)?;

    // New reserves after the implicit swap.
    let new_reserve_in = reserve_in
        .checked_add(half_in)
        .ok_or(AmmError::MathOverflow)?;
    let new_reserve_out = reserve_out
        .checked_sub(quote.amount_out)
        .ok_or(AmmError::MathUnderflow)?;

    // Now treat `remaining_in` (token A) and `quote.amount_out` (token B)
    // as a balanced deposit against the post-swap reserves.
    get_deposit_lp_amount(
        remaining_in,
        quote.amount_out,
        new_reserve_in,
        new_reserve_out,
        lp_supply,
    )
}

// ---- internal helpers ----

/// Computes `(a * b) / c` using u128 intermediates, returning a u64.
/// Used wherever two u64 values are multiplied before dividing, since the
/// product can exceed u64::MAX even when the final result fits.
fn mul_div_u64(a: u64, b: u64, c: u64) -> Result<u64> {
    require!(c > 0, AmmError::InsufficientLiquidity);
    let result = (a as u128)
        .checked_mul(b as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(c as u128)
        .ok_or(AmmError::MathOverflow)?;
    result.try_into().map_err(|_| AmmError::MathOverflow.into())
}

/// Integer square root via Newton's method, operating on u128 to match the
/// product of two u64 reserves. Returns floor(sqrt(value)).
fn integer_sqrt(value: u128) -> u128 {
    if value == 0 {
        return 0;
    }
    let mut x = value;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_quote_basic_no_fee() {
        let quote = get_swap_quote(1_000, 10_000, 10_000, 0).unwrap();
        // x*y=k: (10000+1000) * (10000 - out) = 10000*10000
        // out = 10000 - 100000000/11000 = 10000 - 9090 = 909 (approx)
        assert!(quote.amount_out > 0);
        assert!(quote.amount_out < 1_000);
        assert_eq!(quote.fee_amount, 0);
    }

    #[test]
    fn swap_quote_with_fee_reduces_output() {
        let no_fee = get_swap_quote(1_000, 10_000, 10_000, 0).unwrap();
        let with_fee = get_swap_quote(1_000, 10_000, 10_000, 30).unwrap();
        assert!(with_fee.amount_out < no_fee.amount_out);
        assert_eq!(with_fee.fee_amount, 3); // 1000 * 30/10000 = 3
    }

    #[test]
    fn swap_quote_rejects_zero_amount() {
        assert!(get_swap_quote(0, 10_000, 10_000, 30).is_err());
    }

    #[test]
    fn swap_quote_rejects_empty_reserves() {
        assert!(get_swap_quote(1_000, 0, 10_000, 30).is_err());
        assert!(get_swap_quote(1_000, 10_000, 0, 30).is_err());
    }

    #[test]
    fn swap_quote_rejects_fee_at_or_above_100_percent() {
        assert!(get_swap_quote(1_000, 10_000, 10_000, 10_000).is_err());
    }

    #[test]
    fn deposit_bootstrap_uses_sqrt_product() {
        let lp = get_deposit_lp_amount(10_000, 40_000, 0, 0, 0).unwrap();
        // sqrt(10000 * 40000) = sqrt(400_000_000) = 20000
        assert_eq!(lp, 20_000);
    }

    #[test]
    fn deposit_proportional_after_bootstrap() {
        // Pool already has 10000:40000 with lp_supply 20000.
        // Depositing another 10000:40000 should double LP supply.
        let lp = get_deposit_lp_amount(10_000, 40_000, 10_000, 40_000, 20_000).unwrap();
        assert_eq!(lp, 20_000);
    }

    #[test]
    fn deposit_imbalanced_takes_minimum_ratio() {
        // Reserves 10000:40000 (ratio 1:4), lp_supply 20000.
        // Depositing 5000 A (would need 20000 B) but only providing 10000 B
        // (would need 2500 A) -> limited by the smaller ratio (B-derived).
        let lp = get_deposit_lp_amount(5_000, 10_000, 10_000, 40_000, 20_000).unwrap();
        // lp_from_a = 20000 * 5000 / 10000 = 10000
        // lp_from_b = 20000 * 10000 / 40000 = 5000
        // min = 5000
        assert_eq!(lp, 5_000);
    }

    #[test]
    fn withdraw_pro_rata() {
        let (a, b) = get_withdraw_amounts(10_000, 20_000, 10_000, 40_000).unwrap();
        // withdrawing half the LP supply returns half of each reserve
        assert_eq!(a, 5_000);
        assert_eq!(b, 20_000);
    }

    #[test]
    fn withdraw_rejects_more_than_supply() {
        assert!(get_withdraw_amounts(30_000, 20_000, 10_000, 40_000).is_err());
    }

    #[test]
    fn proportional_amount_matches_ratio() {
        // reserve_known=10000, reserve_other=40000 -> ratio 1:4
        let amount = get_proportional_amount(2_500, 10_000, 40_000).unwrap();
        assert_eq!(amount, 10_000);
    }

    #[test]
    fn single_sided_deposit_mints_less_than_double_sided_equivalent() {
        // Depositing 2000 of token A single-sided into a 10000:40000 pool
        // should mint roughly what a balanced 1000-A/4000-B deposit would,
        // minus fee drag from the implicit internal swap.
        let single_sided =
            get_single_sided_deposit_lp_amount(2_000, 10_000, 40_000, 20_000, 30).unwrap();
        let balanced = get_deposit_lp_amount(1_000, 4_000, 10_000, 40_000, 20_000).unwrap();
        assert!(single_sided > 0);
        // Fee drag means single-sided should mint slightly less.
        assert!(single_sided <= balanced);
    }

    #[test]
    fn integer_sqrt_known_values() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(4), 2);
        assert_eq!(integer_sqrt(400_000_000), 20_000);
        assert_eq!(integer_sqrt(399_999_999), 19_999); // floor, not round
    }
}
