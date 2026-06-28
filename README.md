# eutxo-amm

A constant-product (Uniswap-v2-style) AMM for Solana, built in Anchor,
designed for pools pairing bridged eUTXO-chain assets (Ergo/Cardano-native
tokens arriving via Rosen Bridge) against SOL/USDC/majors.

**This was written without a Rust/Solana toolchain available to compile or
test it.** It has not been built, deployed, or run. Treat it as a careful
first draft, not working software, until you've taken the steps below.

## What's here

```
programs/eutxo-amm/src/
  lib.rs              Program entrypoint, wires instructions together
  state.rs             Pool account schema
  errors.rs            Custom error enum
  math.rs               Pure constant-product math (unit tested, no Anchor deps)
  instructions/
    initialize_pool.rs  Create a new pool for a token pair
    deposit.rs          Paired and single-sided liquidity deposits
    swap.rs             Constant-product swap with fee split + slippage guard
    withdraw.rs          Burn LP tokens for pro-rata reserves (never pausable)
    admin.rs             Pause toggle + protocol fee collection
tests/
  eutxo-amm.test.ts     Integration test scaffold (TypeScript, Anchor/Mocha)
```

## Design decisions worth knowing about

- **Canonical mint ordering.** `initialize_pool` requires `mint_a < mint_b`
  by pubkey bytes. This guarantees one pool address per pair — no risk of
  liquidity splitting across two pools because of argument order. Compute
  this client-side before calling.
- **Token-2022 compatible.** Pool token mints use `token_interface` types,
  so either mint can be legacy SPL Token or Token-2022 (with extensions
  like transfer hooks or metadata pointers) — relevant since bridged-asset
  representations are a likely Token-2022 use case. The **LP share mint is
  always legacy SPL Token** regardless, for wallet/explorer/Jupiter
  compatibility on the LP receipt token itself.
- **Withdraw is never blocked by pause.** `set_paused` stops new swaps and
  deposits but withdrawal always works. A pause mechanism that can trap
  funds is not a safety feature.
- **Protocol fees are bookkeeping, not a separate transfer.** The
  protocol's cut of the swap fee stays in the vault and is only tracked
  in `protocol_fees_a` / `protocol_fees_b` until `collect_protocol_fees`
  is called — it can never exceed what swaps have actually earmarked.
- **Single-sided deposit math is simplified** (modeled as an implicit
  half-swap then a balanced deposit). This is fine for an MVP with thin
  pools but is not the same rigor as production CLMM single-sided
  liquidity math. It's isolated in one function in `math.rs` specifically
  so it's easy to find and upgrade later.

## Before you do anything else

1. **Install the toolchain and build it.**
   ```
   anchor build
   ```
   This will surface any compile errors I couldn't catch by manual review
   (I have no Rust compiler in this environment). Expect to fix small
   things — Anchor macro edge cases, trait bound mismatches, or account
   constraint typos are realistic even after careful review.

2. **Replace the placeholder program ID.** `EuTXoAMM111...` in both
   `Anchor.toml` and `declare_id!()` in `lib.rs` is a placeholder, not a
   real keypair. After your first successful build:
   ```
   anchor keys list
   ```
   and update both files to match.

3. **Run the Rust unit tests** (these need no deployed program, just a
   Rust toolchain):
   ```
   cargo test
   ```
   These cover the math module thoroughly — swap quotes, deposit/withdraw
   math, bootstrap LP minting, the integer sqrt helper. This is the
   highest-value test suite to get green first, since it's where a silent
   bug would be most expensive.

4. **Run the integration tests** against a local validator:
   ```
   anchor test
   ```
   The TS test file is a *scaffold*, not comprehensive — see the doc
   comment at the top of `tests/eutxo-amm.test.ts` for what's covered and
   what's explicitly not (Token-2022 mint behavior, single-sided deposits,
   fee collection, and adversarial/unauthorized-access cases all still
   need tests added).

## Before any mainnet deployment with real funds

This is a from-scratch program handling other people's money. At minimum,
before it touches anything beyond devnet/testnet with play money:

- **Professional security audit.** Non-negotiable for an AMM. Get at
  least one reputable Solana-focused auditing firm to review the program
  before any real liquidity touches it.
- **Test against real Token-2022 mints with extensions enabled** —
  transfer hooks in particular have CPI account-resolution behavior that
  doesn't show up in legacy-SPL-only testing. This is the part of the
  design most likely to have a Solana-specific gotcha that pure code
  review (mine or anyone else's) won't catch.
- **Replace the multisig/governance story.** `admin` is currently a
  single pubkey. Before real funds, that should be a Squads (or
  equivalent) multisig, matching the governance maturity model you've
  already seen Ergo's own USE stablecoin use (3-of-5 multisig, with a
  plan to transition further).
- **Decide and test the upgrade-authority story** for the program itself
  — whether it stays upgradeable, who controls that key, and whether
  there's a timelock — before TVL makes that decision expensive to change.
- **Load-test the math at extreme reserve ratios and dust amounts.** The
  constant-product invariant check in `get_swap_quote` will reject swaps
  where rounding would shrink the pool's `k` — confirm this doesn't
  produce surprising failures for very small trades against very large
  pools, or very imbalanced pools right after launch.

## What's deliberately not built yet

- No CLMM (concentrated liquidity) pool type — start with this CPMM,
  add CLMM later once a pool has consistent volume to justify the
  complexity (see prior discussion: Raydium and Orca both run CPMM/CLMM
  side by side, this isn't unusual).
- No on-chain governance token / fee-share mechanism — get real pools
  and real fee revenue first, decide on tokenomics once there's something
  to share.
- No custom routing or aggregation layer — Jupiter will route through
  these pools automatically once they have liquidity; building your own
  router duplicates work Jupiter already does for you.
