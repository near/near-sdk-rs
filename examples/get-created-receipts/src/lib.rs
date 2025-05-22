

#[cfg(test)]
mod tests {
    use super::*;

#[test]
fn test_created_receipts() {
    // Set up the test context
    let context = VMContextBuilder::new()
        .predecessor_account_id(AccountId::new_unchecked("alice.near".to_string()))
        .build();

    // Initialize the environment
    testing_env!(context);

    // Simulate contract logic that creates a promise (e.g., a transfer or function call)
    Promise::new(AccountId::new_unchecked("bob.near".to_string()))
        .transfer(1_000_000_000_000_000_000_000_000); // 1 NEAR

    // Now, let's inspect the receipts created during the execution
    let receipts = get_created_receipts();

    // Assert that one receipt was created
    assert_eq!(receipts.len(), 1);

    // Grab the first receipt
    let receipt = &receipts[0];

    // Assert that the receipt is going to the correct receiver (in this case, "bob.near")
    assert_eq!(receipt.receiver_id.as_str(), "bob.near");

    // Optionally, you can inspect actions within the receipt.
    // Let's assume you want to check if the receipt contains a transfer action
    let actions = &receipt.actions;
    assert!(actions.iter().any(|action| matches!(action, near_sdk::env::ReceiptAction::Transfer { .. })));
}
}  