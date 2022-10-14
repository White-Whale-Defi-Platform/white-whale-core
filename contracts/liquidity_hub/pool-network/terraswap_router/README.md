# Terraswap Router <!-- omit in toc -->

The Router Contract contains the logic to facilitate multi-hop swap operations among the liquidity pools.

Imagine there are three tokens, A B and C. Say a bot wants to swap A for C, but only two pools are available, A-B and B-C.
Thus, the router can be used to swap A for B and then B for C.

The router can handle both native and cw20 tokens, provided the corresponding `AssetInfo`:

```rust
pub enum AssetInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}
```
### Example

Swap A => B => C
```
{
   "execute_swap_operations":{
      "operations":[
         {
            "terra_swap":{
               "offer_asset_info":{
                   "token":{
                     "contract_addr":"juno1...A"
                  }
               },
               "ask_asset_info":{
                  "token":{
                     "contract_addr":"juno1...B"
                  }
               }
            }
         },
         {
            "terra_swap":{
               "offer_asset_info":{
                  "token":{
                     "contract_addr":"juno1...B"
                  }
               },
               "ask_asset_info":{
                   "token":{
                     "contract_addr":"juno1...C"
                  }
               }
            }
         }
      ],
      "minimum_receive":"1"
   }
}
```
