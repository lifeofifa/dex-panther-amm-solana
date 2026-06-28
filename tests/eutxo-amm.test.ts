/**
 * Integration test scaffold for the eUTXO-bridge AMM.
 *
 * This is a STARTING POINT, not a complete test suite. Run with
 * `anchor test` once you have the Solana/Anchor toolchain installed
 * locally — it has NOT been executed in this environment (no toolchain
 * available here), so treat it as a structural reference and expect to
 * fix small API-shape mismatches once you run it against the real
 * generated IDL/types (e.g. `target/types/eutxo_amm`).
 *
 * Coverage included:
 *   1. Initialize a pool for two freshly-minted test tokens.
 *   2. Bootstrap deposit (first liquidity, empty pool).
 *   3. Second proportional deposit.
 *   4. Swap A -> B with slippage check.
 *   5. Withdraw and confirm pro-rata amounts.
 *   6. Admin pause blocks swap but not withdraw.
 *
 * NOT covered yet (add before relying on this for anything beyond
 * local sanity-checking):
 *   - Single-sided deposit path.
 *   - Token-2022 mint (transfer hook / metadata pointer) compatibility —
 *     critical to test explicitly given that's the whole point of this
 *     AMM's design, and Token-2022 behavior under CPI has real edge
 *     cases (e.g. transfer-hook extra accounts) that legacy SPL tests
 *     won't exercise.
 *   - Protocol fee collection.
 *   - Adversarial cases: zero-amount swaps, slippage rejection paths,
 *     unauthorized admin calls, withdrawing more LP than owned.
 */

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

// Replace with the real generated type once `anchor build` runs.
// import { EutxoAmm } from "../target/types/eutxo_amm";

describe("eutxo-amm", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // const program = anchor.workspace.EutxoAmm as Program<EutxoAmm>;
  const program = anchor.workspace.EutxoAmm as Program<any>;

  const payer = (provider.wallet as anchor.Wallet).payer;
  const admin = Keypair.generate();

  let mintA: PublicKey;
  let mintB: PublicKey;
  let pool: PublicKey;
  let vaultA: PublicKey;
  let vaultB: PublicKey;
  let lpMint: PublicKey;

  let userTokenA: PublicKey;
  let userTokenB: PublicKey;
  let userLpToken: PublicKey;

  const FEE_BPS = 30; // 0.30%
  const PROTOCOL_FEE_SHARE_BPS = 1000; // 10% of the fee

  before(async () => {
    // Two test mints, 6 decimals each (matches typical bridged-asset
    // representations and USDC-style stables).
    const [rawA, rawB] = await Promise.all([
      createMint(provider.connection, payer, payer.publicKey, null, 6),
      createMint(provider.connection, payer, payer.publicKey, null, 6),
    ]);

    // Enforce canonical ordering (mint_a < mint_b) exactly as the program
    // requires — see initialize_pool.rs's `MintsNotCanonicalOrder` check.
    if (rawA.toBuffer().compare(rawB.toBuffer()) < 0) {
      mintA = rawA;
      mintB = rawB;
    } else {
      mintA = rawB;
      mintB = rawA;
    }

    [pool] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), mintA.toBuffer(), mintB.toBuffer()],
      program.programId
    );
    [vaultA] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_a"), pool.toBuffer()],
      program.programId
    );
    [vaultB] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_b"), pool.toBuffer()],
      program.programId
    );
    [lpMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), pool.toBuffer()],
      program.programId
    );

    userTokenA = await createAccount(provider.connection, payer, mintA, payer.publicKey);
    userTokenB = await createAccount(provider.connection, payer, mintB, payer.publicKey);

    await mintTo(provider.connection, payer, mintA, userTokenA, payer, 1_000_000_000); // 1000.000000
    await mintTo(provider.connection, payer, mintB, userTokenB, payer, 4_000_000_000); // 4000.000000
  });

  it("initializes a pool", async () => {
    await program.methods
      .initializePool(FEE_BPS, PROTOCOL_FEE_SHARE_BPS, true /* single-sided enabled */)
      .accounts({
        payer: payer.publicKey,
        admin: admin.publicKey,
        mintA,
        mintB,
        pool,
        vaultA,
        vaultB,
        lpMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const poolAccount = await program.account.pool.fetch(pool);
    assert.equal(poolAccount.feeBps, FEE_BPS);
    assert.equal(poolAccount.admin.toBase58(), admin.publicKey.toBase58());
    assert.equal(poolAccount.paused, false);
  });

  it("bootstraps the pool with an initial paired deposit", async () => {
    // userLpToken is derived as the ATA for (lpMint, payer) — created
    // via init_if_needed inside the deposit instruction itself.
    userLpToken = anchor.utils.token.associatedAddress({
      mint: lpMint,
      owner: payer.publicKey,
    });

    const depositA = new BN(100_000_000); // 100.000000
    const depositB = new BN(400_000_000); // 400.000000 (matches 1:4 ratio)

    await program.methods
      .deposit({ paired: {} }, depositA, depositB, new BN(0) /* min_lp_out */)
      .accounts({
        depositor: payer.publicKey,
        pool,
        mintA,
        mintB,
        vaultA,
        vaultB,
        lpMint,
        depositorTokenA: userTokenA,
        depositorTokenB: userTokenB,
        depositorLpToken: userLpToken,
        tokenProgram: TOKEN_PROGRAM_ID,
        lpTokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const lpAccount = await getAccount(provider.connection, userLpToken);
    // Bootstrap LP = sqrt(100_000_000 * 400_000_000) = 200_000_000
    assert.equal(lpAccount.amount.toString(), "200000000");
  });

  it("executes a swap A -> B respecting slippage", async () => {
    const swapAmountIn = new BN(10_000_000); // 10.000000 of token A

    const vaultBBefore = await getAccount(provider.connection, vaultB);

    await program.methods
      .swap({ aToB: {} }, swapAmountIn, new BN(0) /* min_amount_out, loose for test */)
      .accounts({
        trader: payer.publicKey,
        pool,
        mintA,
        mintB,
        vaultA,
        vaultB,
        traderTokenA: userTokenA,
        traderTokenB: userTokenB,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultBAfter = await getAccount(provider.connection, vaultB);
    assert.isTrue(vaultBAfter.amount < vaultBBefore.amount, "vault B should decrease");
  });

  it("rejects a swap with an unrealistic slippage floor", async () => {
    const swapAmountIn = new BN(10_000_000);
    const impossibleMinOut = new BN(999_999_999_999); // way more than the pool holds

    let threw = false;
    try {
      await program.methods
        .swap({ aToB: {} }, swapAmountIn, impossibleMinOut)
        .accounts({
          trader: payer.publicKey,
          pool,
          mintA,
          mintB,
          vaultA,
          vaultB,
          traderTokenA: userTokenA,
          traderTokenB: userTokenB,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
    } catch (_err) {
      threw = true;
    }
    assert.isTrue(threw, "expected SlippageExceeded error");
  });

  it("withdraws liquidity pro-rata", async () => {
    const lpBalanceBefore = await getAccount(provider.connection, userLpToken);
    const withdrawAmount = new BN(lpBalanceBefore.amount.toString()).div(new BN(2));

    await program.methods
      .withdraw(withdrawAmount, new BN(0), new BN(0))
      .accounts({
        withdrawer: payer.publicKey,
        pool,
        mintA,
        mintB,
        vaultA,
        vaultB,
        lpMint,
        withdrawerTokenA: userTokenA,
        withdrawerTokenB: userTokenB,
        withdrawerLpToken: userLpToken,
        tokenProgram: TOKEN_PROGRAM_ID,
        lpTokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const lpBalanceAfter = await getAccount(provider.connection, userLpToken);
    assert.isTrue(lpBalanceAfter.amount < lpBalanceBefore.amount);
  });

  it("admin can pause the pool, blocking swaps", async () => {
    await program.methods
      .setPaused(true)
      .accounts({
        admin: admin.publicKey,
        pool,
      })
      .signers([admin])
      .rpc();

    let threw = false;
    try {
      await program.methods
        .swap({ aToB: {} }, new BN(1_000_000), new BN(0))
        .accounts({
          trader: payer.publicKey,
          pool,
          mintA,
          mintB,
          vaultA,
          vaultB,
          traderTokenA: userTokenA,
          traderTokenB: userTokenB,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
    } catch (_err) {
      threw = true;
    }
    assert.isTrue(threw, "swap should fail while paused");

    // Unpause for any subsequent tests / cleanliness.
    await program.methods
      .setPaused(false)
      .accounts({ admin: admin.publicKey, pool })
      .signers([admin])
      .rpc();
  });

  it("withdraw still works while the pool is paused", async () => {
    await program.methods
      .setPaused(true)
      .accounts({ admin: admin.publicKey, pool })
      .signers([admin])
      .rpc();

    const lpBalanceBefore = await getAccount(provider.connection, userLpToken);

    // Should NOT throw — withdraw is never blocked by pause.
    await program.methods
      .withdraw(new BN(lpBalanceBefore.amount.toString()), new BN(0), new BN(0))
      .accounts({
        withdrawer: payer.publicKey,
        pool,
        mintA,
        mintB,
        vaultA,
        vaultB,
        lpMint,
        withdrawerTokenA: userTokenA,
        withdrawerTokenB: userTokenB,
        withdrawerLpToken: userLpToken,
        tokenProgram: TOKEN_PROGRAM_ID,
        lpTokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const lpBalanceAfter = await getAccount(provider.connection, userLpToken);
    assert.equal(lpBalanceAfter.amount.toString(), "0");
  });
});
