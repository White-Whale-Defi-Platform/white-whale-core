use cosmwasm_std::{testing::mock_info, MessageInfo};

/// Creates a mock creator
pub fn mock_creator() -> MessageInfo {
    mock_info("creator", &[])
}

/// Creates alice's mock
pub fn mock_alice() -> MessageInfo {
    mock_info("alice", &[])
}

pub fn mock_admin() -> MessageInfo {
    mock_info("admin", &[])
}
