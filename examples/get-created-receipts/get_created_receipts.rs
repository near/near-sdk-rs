#[test]
fn test_receipts_creation() {
    setup_context();
    
    // Execute a function that triggers a cross-contract call
    contract.some_method_that_calls_another_contract();
    
    // Retrieve the receipts
    let receipts = get_created_receipts();
    
    // Print or assert properties of the receipts
    assert!(!receipts.is_empty());
    println!("Receipts: {:?}", receipts);
}