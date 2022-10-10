use cosmwasm_std::{testing::mock_info, Addr, MessageInfo};

/// Creates a mock creator
pub fn mock_creator() -> MessageInfo {
    mock_info("creator", &[])
}

pub fn mock_admin() -> Addr {
    Addr::unchecked("mock_admin")
}
