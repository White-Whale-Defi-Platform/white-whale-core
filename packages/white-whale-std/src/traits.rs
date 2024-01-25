use cosmwasm_std::Decimal;

/// A trait for converting an Option<Decimal> to a string.
pub trait OptionDecimal {
    fn to_string(self) -> String;
}

impl OptionDecimal for Option<Decimal> {
    fn to_string(self) -> String {
        match self {
            None => "None".to_string(),
            Some(d) => d.to_string(),
        }
    }
}
