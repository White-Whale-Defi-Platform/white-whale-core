# TerraSwap Factory

The factory contract can create instances of the Terraswap Pair contract. It is also used as directory contract for all
pools ever created.

The Factory is permissioned.

## InstantiateMsg

```json
{
  "pair_code_id": 123,
  "token_code_id": 123,
  "fee_collector_addr": "juno1..."
}
```

## ExecuteMsg

### `update_config`

```json
{
  "update_config": {
    "owner": "juno1...",
    "fee_collector_addr": "juno1...",
    "token_id": 123,
    "pair_code_id": 123
  }
}
```

### `create_pair`

```json
{
  "create_pair": {
    "asset_infos": [
      {
        "token": {
          "contract_addr": "juno1..."
        }
      },
      {
        "native_token": {
          "denom": "ujuno"
        }
      }
    ]
  }
}
```
### `add_native_token_decimals`

```json
{
  "add_native_token_decimals": {
    "denom": "ujuno",
    "decimals": 6
  }
}
```

## QueryMsg

### `config`

```json
{
  "config": {}
}
```

### `pair`

```json
{
  "pair": {
    "asset_infos": [
      {
        "token": {
          "contract_addr": "juno1..."
        }
      },
      {
        "native_token": {
          "denom": "ujuno"
        }
      }
    ]
  }
}
```

### `pairs`

```json
{
  "pairs": {
    "start_after": [
      {
        "token": {
          "contract_addr": "juno1..."
        }
      },
      {
        "native_token": {
          "denom": "ujuno"
        }
      }
    ],
    "limit": 30
  }
}
```

### `native_token_decimals`

```json
{
  "native_token_decimals": {
    "denom": "ujuno"
  }
}
```
