# TerraSwap Pair

The TerraSwap Pair contract is used to create token pools. It is used by the TerraSwap Factory contract and not by its own.

## Handlers

### Initialize

It initializes the contract with the parameters required for its functioning, such as the pool fees and fee collector address.

```rust
pub struct InstantiateMsg {
  /// Asset infos
  pub asset_infos: [AssetInfo; 2],
  /// Token contract code id for initialization
  pub token_code_id: u64,
  pub asset_decimals: [u8; 2],
  pub pool_fees: PoolFee,
  pub fee_collector_addr: String,
}

```

### Liquidity Provision

A user or bot can provide liquidity by sending a `provide_liquidity` message and withdraw liquidity with `withdraw_liquidity`.
Whenever liquidity is deposited into a pool, special tokens known as liquidity (LP) tokens are minted to the provider’s address,
in proportion to how much liquidity it contributed to the pool. These tokens are a representation of a liquidity provider’s contribution to a pool.
Whenever a trade occurs, the `swap_fee` percentage, which is a parameter specified by the `PoolFee`, is distributed pro-rata
to all LPs in the pool at the moment of the trade. To receive the underlying liquidity back, plus commission fees that were
accrued while their liquidity was locked, LPs must burn their liquidity tokens.

> Note before executing the `provide_liqudity` operation, a user must allow the contract to use the liquidity amount of
> asset in the token contract.

#### Slippage Tolerance

If a user or bot specifies the slippage tolerance when providing liquidity, the contract restricts the operation when the
exchange rate is dropped more than the tolerance.

#### Request Format

- Increase allowance (to be executed on the cw20 token to be provided)

  ```json
  {
    "increase_allowance": {
      "spender": "juno1...",
      "amount": "1000000"
    }
  }
  ```

- Provide Liquidity

  1. Without Slippage Tolerance

  ```json
  {
    "provide_liquidity": {
      "assets": [
        {
          "info": {
            "token": {
              "contract_addr": "juno1..."
            }
          },
          "amount": "1000000"
        },
        {
          "info": {
            "native_token": {
              "denom": "ujuno"
            }
          },
          "amount": "1000000"
        }
      ]
    }
  }
  ```

  2. With Slippage Tolerance

  ```json
  {
    "provide_liquidity": {
      "assets": [
        {
          "info": {
            "token": {
              "contract_addr": "juno1..."
            }
          },
          "amount": "1000000"
        },
        {
          "info": {
            "native_token": {
              "denom": "ujuno"
            }
          },
          "amount": "1000000"
        }
      ]
    },
    "slippage_tolerance": "0.01"
  }
  ```

- Withdraw Liquidity (must be sent to liquidity (LP) token contract)
  ```json
  {
    "withdraw_liquidity": {}
  }
  ```

### Swap

Any user or bot can swap an asset by sending `swap` or invoking `send` message on the token contract with the `swap` hook
message.

- Native Token => cw20 Token

  ```json
  {
      "swap": {
          "offer_asset": {
              "info": {
                  "native_token": {
                      "denom": String
                  }
              },
              "amount": Uint128
          },
          "belief_price": Option<Decimal>,
          "max_spread": Option<Decimal>,
          "to": Option<HumanAddr>
      }
  }
  ```

- Token => Native Token

  **Must be sent to LP token contract**

  ```json
  {
      "send": {
          "contract": HumanAddr,
          "amount": Uint128,
          "msg": Binary({
              "swap": {
                  "belief_price": Option<Decimal>,
                  "max_spread": Option<Decimal>,
                  "to": Option<HumanAddr>
              }
          })
      }
  }
  ```

#### Swap Spread

The spread is determined with following uniswap mechanism:

```rust
pub fn compute_swap(
  offer_pool: Uint128,
  ask_pool: Uint128,
  offer_amount: Uint128,
  pool_fees: PoolFee,
) -> SwapComputation {
  let offer_pool: Uint256 = Uint256::from(offer_pool);
  let ask_pool: Uint256 = ask_pool.into();
  let offer_amount: Uint256 = offer_amount.into();

  // offer => ask
  // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount)) - swap_fee - protocol_fee
  let return_amount: Uint256 = Uint256::one()
    * Decimal256::from_ratio(ask_pool.mul(offer_amount), offer_pool + offer_amount);

  // calculate spread, swap and protocol fees
  let exchange_rate = Decimal256::from_ratio(ask_pool, offer_pool);
  let spread_amount: Uint256 = (offer_amount * exchange_rate) - return_amount;
  let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(return_amount);
  let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(return_amount);

  // swap and protocol fee will be absorbed by the pool
  let return_amount: Uint256 = return_amount - swap_fee_amount - protocol_fee_amount;

  SwapComputation {
    return_amount: return_amount.into(),
    spread_amount: spread_amount.into(),
    swap_fee_amount: swap_fee_amount.into(),
    protocol_fee_amount: protocol_fee_amount.into(),
  }
}
```

#### Fees

There are two fees associated to the pools, namely `swap_fee` and `protocol_fee`.

The `swap_fee` remains in the swap pool, causing a permanent increase in the constant product K. The value of this
permanently increased pool goes to all LPs.

The `protocol_fee` goes to the protocol, and it is to be collected by the Fee Collector contract of the Liquidity Hub.
