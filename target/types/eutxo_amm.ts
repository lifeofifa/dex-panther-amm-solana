/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/eutxo_amm.json`.
 */
export type EutxoAmm = {
  "address": "6cmEegP2W9pBaP2CB7ZkyupZxM8NZUwU3NiqSE7ci5iR",
  "metadata": {
    "name": "eutxoAmm",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Constant-product AMM for bridged eUTXO-chain assets on Solana"
  },
  "instructions": [
    {
      "name": "collectProtocolFees",
      "discriminator": [
        22,
        67,
        23,
        98,
        150,
        178,
        70,
        220
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "pool.mintA",
                "account": "pool"
              },
              {
                "kind": "account",
                "path": "pool.mintB",
                "account": "pool"
              }
            ]
          }
        },
        {
          "name": "mintA"
        },
        {
          "name": "mintB"
        },
        {
          "name": "vaultA",
          "writable": true
        },
        {
          "name": "vaultB",
          "writable": true
        },
        {
          "name": "feeDestinationA",
          "docs": [
            "Destination for collected token-A fees. Validated only on mint —",
            "ownership can be any account the admin (e.g. a DAO treasury",
            "multisig) controls."
          ],
          "writable": true
        },
        {
          "name": "feeDestinationB",
          "writable": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": []
    },
    {
      "name": "deposit",
      "discriminator": [
        242,
        35,
        198,
        137,
        82,
        225,
        242,
        182
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "pool.mintA",
                "account": "pool"
              },
              {
                "kind": "account",
                "path": "pool.mintB",
                "account": "pool"
              }
            ]
          }
        },
        {
          "name": "mintA"
        },
        {
          "name": "mintB"
        },
        {
          "name": "vaultA",
          "writable": true
        },
        {
          "name": "vaultB",
          "writable": true
        },
        {
          "name": "lpMint",
          "writable": true
        },
        {
          "name": "depositorTokenA",
          "docs": [
            "Depositor's token A account. Required even for SingleSidedB deposits",
            "(Anchor needs a fixed account set per instruction); unused token",
            "movement is simply zero in that case."
          ],
          "writable": true
        },
        {
          "name": "depositorTokenB",
          "writable": true
        },
        {
          "name": "depositorLpToken",
          "docs": [
            "Depositor's LP token account (legacy SPL Token, see initialize_pool",
            "for rationale). Created if it doesn't exist yet."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "depositor"
              },
              {
                "kind": "account",
                "path": "lpTokenProgram"
              },
              {
                "kind": "account",
                "path": "lpMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program governing mint_a / mint_b — may be legacy SPL Token",
            "or Token-2022 depending on the pool's configured mints."
          ]
        },
        {
          "name": "lpTokenProgram",
          "docs": [
            "Token program governing the LP mint specifically — always legacy",
            "SPL Token (see initialize_pool)."
          ]
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "mode",
          "type": {
            "defined": {
              "name": "depositMode"
            }
          }
        },
        {
          "name": "amountA",
          "type": "u64"
        },
        {
          "name": "amountB",
          "type": "u64"
        },
        {
          "name": "minLpOut",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializePool",
      "docs": [
        "Create a new pool for a (mint_a, mint_b) pair. Mints must be passed",
        "in canonical order (mint_a < mint_b by pubkey) so a pair maps to",
        "exactly one pool address.",
        "",
        "`fee_bps`: total swap fee in basis points (e.g. 30 = 0.30%).",
        "`protocol_fee_share_bps`: share of that fee routed to protocol",
        "revenue rather than left for LPs, in bps of the fee itself",
        "(e.g. 1000 = 10% of the fee).",
        "`single_sided_deposits_enabled`: whether this pool accepts",
        "one-sided deposits — recommended `true` for thin/bridged-asset",
        "pools, `false` for deep majors where paired deposits are the norm."
      ],
      "discriminator": [
        95,
        180,
        10,
        172,
        84,
        174,
        232,
        40
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "admin",
          "docs": [
            "The address that will administer this pool (collect protocol fees,",
            "pause/unpause). In production this MUST be a multisig or DAO",
            "address, never a single hot wallet — pass that address here",
            "explicitly rather than defaulting to `payer`."
          ]
        },
        {
          "name": "mintA"
        },
        {
          "name": "mintB"
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "mintA"
              },
              {
                "kind": "account",
                "path": "mintB"
              }
            ]
          }
        },
        {
          "name": "vaultA",
          "docs": [
            "Pool-owned vault for token A. The pool PDA is the authority."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "vaultB",
          "docs": [
            "Pool-owned vault for token B. The pool PDA is the authority."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  98
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "lpMint",
          "docs": [
            "LP share mint. Always a plain SPL Token mint (not Token-2022) for",
            "simplicity and maximum downstream compatibility (wallets, explorers,",
            "and Jupiter's LP-token handling all assume legacy SPL for LP",
            "tokens) — this only governs the LP receipt token, not the underlying",
            "pooled assets, which can still be Token-2022 mints."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  108,
                  112,
                  95,
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "feeBps",
          "type": "u16"
        },
        {
          "name": "protocolFeeShareBps",
          "type": "u16"
        },
        {
          "name": "singleSidedDepositsEnabled",
          "type": "bool"
        }
      ]
    },
    {
      "name": "setPaused",
      "discriminator": [
        91,
        60,
        125,
        192,
        176,
        225,
        166,
        218
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "pool.mintA",
                "account": "pool"
              },
              {
                "kind": "account",
                "path": "pool.mintB",
                "account": "pool"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "paused",
          "type": "bool"
        }
      ]
    },
    {
      "name": "swap",
      "discriminator": [
        248,
        198,
        158,
        145,
        225,
        117,
        135,
        200
      ],
      "accounts": [
        {
          "name": "trader",
          "signer": true
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "pool.mintA",
                "account": "pool"
              },
              {
                "kind": "account",
                "path": "pool.mintB",
                "account": "pool"
              }
            ]
          }
        },
        {
          "name": "mintA"
        },
        {
          "name": "mintB"
        },
        {
          "name": "vaultA",
          "writable": true
        },
        {
          "name": "vaultB",
          "writable": true
        },
        {
          "name": "traderTokenA",
          "writable": true
        },
        {
          "name": "traderTokenB",
          "writable": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "direction",
          "type": {
            "defined": {
              "name": "swapDirection"
            }
          }
        },
        {
          "name": "amountIn",
          "type": "u64"
        },
        {
          "name": "minAmountOut",
          "type": "u64"
        }
      ]
    },
    {
      "name": "withdraw",
      "discriminator": [
        183,
        18,
        70,
        156,
        148,
        109,
        161,
        34
      ],
      "accounts": [
        {
          "name": "withdrawer",
          "signer": true
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "pool.mintA",
                "account": "pool"
              },
              {
                "kind": "account",
                "path": "pool.mintB",
                "account": "pool"
              }
            ]
          }
        },
        {
          "name": "mintA"
        },
        {
          "name": "mintB"
        },
        {
          "name": "vaultA",
          "writable": true
        },
        {
          "name": "vaultB",
          "writable": true
        },
        {
          "name": "lpMint",
          "writable": true
        },
        {
          "name": "withdrawerTokenA",
          "writable": true
        },
        {
          "name": "withdrawerTokenB",
          "writable": true
        },
        {
          "name": "withdrawerLpToken",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "lpTokenProgram"
        }
      ],
      "args": [
        {
          "name": "lpAmount",
          "type": "u64"
        },
        {
          "name": "minAmountAOut",
          "type": "u64"
        },
        {
          "name": "minAmountBOut",
          "type": "u64"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "pool",
      "discriminator": [
        241,
        154,
        109,
        4,
        17,
        177,
        109,
        188
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "identicalMints",
      "msg": "Token A mint and token B mint must differ"
    },
    {
      "code": 6001,
      "name": "mintsNotCanonicalOrder",
      "msg": "Mints must be provided in canonical order (mint_a < mint_b)"
    },
    {
      "code": 6002,
      "name": "feeTooHigh",
      "msg": "Fee in basis points must be less than 10000 (100%)"
    },
    {
      "code": 6003,
      "name": "protocolFeeShareTooHigh",
      "msg": "Protocol fee share in basis points must be less than or equal to 10000"
    },
    {
      "code": 6004,
      "name": "zeroDepositAmount",
      "msg": "Deposit amounts must be greater than zero"
    },
    {
      "code": 6005,
      "name": "zeroSwapAmount",
      "msg": "Swap input amount must be greater than zero"
    },
    {
      "code": 6006,
      "name": "singleSidedDepositsDisabled",
      "msg": "This pool does not allow single-sided deposits"
    },
    {
      "code": 6007,
      "name": "zeroLpTokensMinted",
      "msg": "Resulting LP token amount is zero — deposit too small relative to pool size"
    },
    {
      "code": 6008,
      "name": "slippageExceeded",
      "msg": "Slippage tolerance exceeded: output amount below minimum specified"
    },
    {
      "code": 6009,
      "name": "excessiveInputRequired",
      "msg": "Slippage tolerance exceeded: required input above maximum specified"
    },
    {
      "code": 6010,
      "name": "insufficientLiquidity",
      "msg": "Pool reserves cannot be zero for this operation"
    },
    {
      "code": 6011,
      "name": "mathOverflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6012,
      "name": "mathUnderflow",
      "msg": "Arithmetic underflow"
    },
    {
      "code": 6013,
      "name": "poolPaused",
      "msg": "Pool is currently paused for swaps and deposits"
    },
    {
      "code": 6014,
      "name": "zeroLpBurnAmount",
      "msg": "LP token amount to burn must be greater than zero"
    },
    {
      "code": 6015,
      "name": "insufficientLpBalance",
      "msg": "Insufficient LP token balance for this withdrawal"
    },
    {
      "code": 6016,
      "name": "unauthorized",
      "msg": "Only the pool admin may perform this action"
    },
    {
      "code": 6017,
      "name": "noFeesToCollect",
      "msg": "No protocol fees available to collect"
    },
    {
      "code": 6018,
      "name": "mintMismatch",
      "msg": "Provided token mint does not match this pool's configured mint"
    }
  ],
  "types": [
    {
      "name": "depositMode",
      "docs": [
        "Deposit mode, passed by the client to select paired vs. single-sided.",
        "Encoded as an instruction argument rather than two separate instructions",
        "so the account context (and therefore the IDL surface) stays identical",
        "regardless of mode — simpler clients, simpler CPI callers."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "paired"
          },
          {
            "name": "singleSidedA"
          },
          {
            "name": "singleSidedB"
          }
        ]
      }
    },
    {
      "name": "pool",
      "docs": [
        "Core pool state, stored in a PDA.",
        "This account is both the data store and, via PDA signing, the",
        "authority over the two reserve token accounts and the LP mint."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "bump",
            "docs": [
              "Bump seed for this pool's PDA, cached to avoid re-deriving on every ix."
            ],
            "type": "u8"
          },
          {
            "name": "mintA",
            "docs": [
              "Mint of token A (by convention, store the lexicographically smaller",
              "mint pubkey as token A — enforced at init time — so a given pair",
              "always resolves to exactly one pool address, never two)."
            ],
            "type": "pubkey"
          },
          {
            "name": "mintB",
            "docs": [
              "Mint of token B."
            ],
            "type": "pubkey"
          },
          {
            "name": "vaultA",
            "docs": [
              "Pool-owned reserve token account holding token A."
            ],
            "type": "pubkey"
          },
          {
            "name": "vaultB",
            "docs": [
              "Pool-owned reserve token account holding token B."
            ],
            "type": "pubkey"
          },
          {
            "name": "lpMint",
            "docs": [
              "Mint for this pool's LP (liquidity provider) share token.",
              "The pool PDA is the mint authority."
            ],
            "type": "pubkey"
          },
          {
            "name": "feeBps",
            "docs": [
              "Swap fee in basis points (1 bps = 0.01%). E.g. 30 = 0.30%."
            ],
            "type": "u16"
          },
          {
            "name": "protocolFeeShareBps",
            "docs": [
              "Protocol fee cut of the swap fee, in basis points of the fee itself",
              "(not of the trade). E.g. 1000 = 10% of the 0.30% fee, i.e. 0.03%",
              "of trade volume, accrues to the protocol; the rest stays in the pool",
              "for LPs."
            ],
            "type": "u16"
          },
          {
            "name": "protocolFeesA",
            "docs": [
              "Accrued protocol fees, denominated in token A, awaiting withdrawal."
            ],
            "type": "u64"
          },
          {
            "name": "protocolFeesB",
            "docs": [
              "Accrued protocol fees, denominated in token B, awaiting withdrawal."
            ],
            "type": "u64"
          },
          {
            "name": "admin",
            "docs": [
              "Authority allowed to withdraw protocol fees and pause the pool.",
              "Set to a multisig/DAO address in production, never a single hot key."
            ],
            "type": "pubkey"
          },
          {
            "name": "paused",
            "docs": [
              "Emergency pause switch. When true, swap and deposit are blocked;",
              "withdraw always remains available so LPs are never locked out",
              "of their own funds."
            ],
            "type": "bool"
          },
          {
            "name": "singleSidedDepositsEnabled",
            "docs": [
              "Whether this pool allows single-sided deposits (deposit only one",
              "of the two assets). Disabled by default; enabled per-pool by the",
              "admin for thin/bridged-asset pools where requiring a matched pair",
              "would be a bad UX bar for first liquidity."
            ],
            "type": "bool"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future fields so this account can grow without",
              "a breaking migration. Shrink this as fields are added."
            ],
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "swapDirection",
      "docs": [
        "Which direction the swap goes. Encoded explicitly rather than inferred",
        "from account ordering, so the instruction's behavior is unambiguous",
        "from the IDL alone."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "aToB"
          },
          {
            "name": "bToA"
          }
        ]
      }
    }
  ]
};
