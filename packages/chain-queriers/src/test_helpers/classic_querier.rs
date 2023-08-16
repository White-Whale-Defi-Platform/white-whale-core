use std::{collections::HashMap, str::FromStr};

use classic_bindings::{TaxCapResponse, TaxRateResponse, TerraQuery};
use cosmwasm_std::{to_binary, Decimal, QuerierResult, Uint128, SystemResult, SystemError};

pub fn err_unsupported_query<T: std::fmt::Debug>(request: T) -> QuerierResult {
    SystemResult::Err(SystemError::InvalidRequest {
        error: format!("[mock] unsupported query: {:?}", request),
        request: Default::default(),
    })
}
#[derive(Default)]
pub struct TerraQuerier {
    /// Maps (base_denom, quote_denom) pair to exchange rate
    pub exchange_rates: HashMap<(String, String), Decimal>,
}

impl TerraQuerier {
    /// We only implement the `exchange_rates` query as that is the only one we need in the unit tests
    ///
    /// NOTE: When querying exchange rates, Terra's oracle module behaves in the following way:
    /// - If `quote_denoms` contains _at least one_ known denom (meaning a denom that has exchange
    ///   rate defined), the query will be successful, and the response will contain the exchange
    ///   rates of only known denoms and omit denoms that are not known;
    /// - If `quote_denoms` only contains unknown denoms, the query fails.
    ///
    /// Examples:
    /// - [Success](https://bombay-fcd.terra.dev/wasm/contracts/terra1xf8kh2r7n06wk0mdhq0tgcrcyv90snjzfxfacg/store?query_msg=%7B%22ExchangeRates%22:[%22uusd%22,%22ukrw%22,%22ibc%2F0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B%22]%7D),
    ///   where the unknown denom (`ibc/...`) is omitted from the response
    /// - [Fail](https://bombay-fcd.terra.dev/wasm/contracts/terra1xf8kh2r7n06wk0mdhq0tgcrcyv90snjzfxfacg/store?query_msg=%7B%22ExchangeRates%22:[%22ibc%2F0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B%22]%7D),
    ///   if the query only contains the unknown denom
    ///
    /// We emulate this behaviour in our mock querier.
    pub fn handle_query(&self, query: &TerraQuery) -> QuerierResult {
        // if let TerraQueryWrapper::ExchangeRates {
        //     base_denom,
        //     quote_denoms,
        // } = query
        // {
        //     let exchange_rates: Vec<ExchangeRateItem> = quote_denoms
        //         .iter()
        //         .filter_map(|quote_denom| {
        //             self.exchange_rates.get(&(base_denom.clone(), quote_denom.clone())).map(
        //                 |rate| ExchangeRateItem {
        //                     quote_denom: quote_denom.clone(),
        //                     exchange_rate: *rate,
        //                 },
        //             )
        //         })
        //         .collect();

        //     if exchange_rates.is_empty() {
        //         return SystemResult::Err(SystemError::InvalidRequest {
        //             error: "[mock] quote_denoms are all unknown".to_string(),
        //             request: Default::default(),
        //         });
        //     }

        //     return Ok(to_binary(&ExchangeRatesResponse {
        //         base_denom: base_denom.into(),
        //         exchange_rates,
        //     })
        //     .into())
        //     .into();
        // }

        if let TerraQuery::TaxCap {
            denom: _,
        } = query
        {
            return Ok(to_binary(&TaxCapResponse {
                cap: Uint128::new(100),
            })
            .into())
            .into();
        }

        if let TerraQuery::TaxRate {} = query {
            return Ok(to_binary(&TaxRateResponse {
                rate: Decimal::from_str("0.01").unwrap(),
            })
            .into())
            .into();
        }

        err_unsupported_query(query)
    }
}