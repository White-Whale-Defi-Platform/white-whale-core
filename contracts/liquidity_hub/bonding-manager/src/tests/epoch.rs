use crate::ContractError;

use crate::tests::suite::TestingSuite;

#[test]
fn test_call_on_epoch_created_hook_unauthorized() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

    suite.instantiate_default().add_one_day().create_new_epoch();

    suite.on_epoch_created(creator, |result| {
        let err = result.unwrap_err().downcast::<ContractError>().unwrap();

        match err {
            ContractError::Unauthorized { .. } => {}
            _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
        }
    });
}
