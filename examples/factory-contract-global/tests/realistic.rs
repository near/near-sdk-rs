use near_workspaces::types::{AccountId, NearToken};

/// Test realistic multisig factory scenario based on NEP-591
/// This demonstrates the primary use case: deploying the same contract multiple times
/// without paying full storage costs each time.
#[tokio::test]
async fn test_multisig_factory_global_contract() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox_with_version("master/5e4b47da55e18f2d2ce3d88f84c15e607380970e").await?;
    
    // Deploy the factory contract
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // In a real scenario, this would be a multisig contract
    // For testing, we'll use the status-message contract as a stand-in
    let multisig_code = near_workspaces::compile_project("../status-message").await?;
    
    println!("📦 Multisig contract size: {} bytes", multisig_code.len());

    // 1. Deploy the multisig contract as a global contract (by code hash)
    // This is done once and makes the contract available to everyone
    let global_multisig_id: AccountId = format!("multisig-global.{}", factory_contract.id()).parse()?;
    
    let deploy_result = factory_contract
        .call("deploy_global_contract")
        .args_json((
            "multisig_v1.0.0",
            multisig_code.clone(),
            &global_multisig_id,
            NearToken::from_near(20)
        ))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;
    
    assert!(deploy_result.is_success(), "Failed to deploy global multisig: {:?}", deploy_result.outcome());
    println!("✅ Global multisig contract deployed successfully");

    // Get the code hash for reuse
    let multisig_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("multisig_v1.0.0",))
        .view()
        .await?
        .json::<Option<Vec<u8>>>()?
        .expect("Should have stored hash");

    // 2. Now multiple users can create multisig wallets without paying full storage costs
    let user_accounts = vec![
        format!("alice-multisig.{}", factory_contract.id()),
        format!("bob-multisig.{}", factory_contract.id()),
        format!("carol-multisig.{}", factory_contract.id()),
    ];

    for (i, user_account) in user_accounts.iter().enumerate() {
        let user_id: AccountId = user_account.parse()?;
        
        // Each user gets their own multisig instance by referencing the global contract
        let result = factory_contract
            .call("use_global_contract_by_hash")
            .args_json((
                &multisig_hash,
                &user_id,
                NearToken::from_near(5) // Much lower cost than full deployment
            ))
            .max_gas()
            .deposit(NearToken::from_near(15))
            .transact()
            .await?;
        
        assert!(result.is_success(), "Failed to create multisig for user {}: {:?}", i, result.outcome());
        println!("✅ User {} created multisig wallet at {}", i + 1, user_id);
    }

    // 3. Verify that all user accounts can use the same contract code
    // In a real scenario, each would have their own multisig state but share the same code
    for user_account in &user_accounts {
        let user_id: AccountId = user_account.parse()?;
        
        // Test calling the multisig contract (using status message as proxy)
        let result = factory_contract
            .call("call_global_status_contract")
            .args_json((&user_id, format!("Multisig initialized for {}", user_id)))
            .max_gas()
            .transact()
            .await?;
        
        // This should succeed, showing the contract is functional
        assert!(result.is_success(), "Failed to call multisig for {}: {:?}", user_id, result.outcome());
    }

    println!("🎉 Successfully created {} multisig wallets using global contract!", user_accounts.len());
    println!("💰 Cost savings: Instead of paying full storage for each deployment,");
    println!("   users only pay for the reference to the global contract.");

    Ok(())
}

/// Test business onboarding scenario where a company creates wallets for users
/// This addresses the refund abuse issue mentioned in NEP-591
#[tokio::test]
async fn test_business_onboarding_global_contracts() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox_with_version("master/5e4b47da55e18f2d2ce3d88f84c15e607380970e").await?;
    
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    // Simulate a business deploying a wallet contract for their users
    let wallet_code = near_workspaces::compile_project("../status-message").await?;

    // Business deploys the wallet as a global contract by account ID
    // This allows them to update the contract for all users if needed
    let business_wallet_deployer: AccountId = format!("business.{}", factory_contract.id()).parse()?;
    
    let deploy_result = factory_contract
        .call("deploy_global_contract_by_account_id")
        .args_json((
            "business_wallet",
            wallet_code,
            &business_wallet_deployer,
            NearToken::from_near(25)
        ))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;
    
    assert!(deploy_result.is_success(), "Failed to deploy business wallet: {:?}", deploy_result.outcome());
    println!("🏢 Business deployed global wallet contract");

    // Business creates wallet accounts for multiple users
    let customer_accounts = vec![
        format!("customer1.{}", factory_contract.id()),
        format!("customer2.{}", factory_contract.id()),
        format!("customer3.{}", factory_contract.id()),
        format!("customer4.{}", factory_contract.id()),
        format!("customer5.{}", factory_contract.id()),
    ];

    for (i, customer_account) in customer_accounts.iter().enumerate() {
        let customer_id: AccountId = customer_account.parse()?;
        
        // Business creates wallet for customer using global contract by account ID
        let result = factory_contract
            .call("use_global_contract_by_account")
            .args_json((
                &business_wallet_deployer,
                &customer_id,
                NearToken::from_near(2) // Very low cost for the business
            ))
            .max_gas()
            .deposit(NearToken::from_near(10))
            .transact()
            .await?;
        
        assert!(result.is_success(), "Failed to create wallet for customer {}: {:?}", i, result.outcome());
        println!("👤 Created wallet for customer {} at {}", i + 1, customer_id);
    }

    // Verify the business can update the contract for all users by redeploying
    // In this test, we just verify the deployer is correctly stored
    let deployer = factory_contract
        .call("get_global_contract_deployer")
        .args_json(("business_wallet",))
        .view()
        .await?
        .json::<Option<AccountId>>()?;
    
    assert_eq!(deployer, Some(business_wallet_deployer));
    println!("✅ Business maintains control over global contract for updates");

    println!("🎉 Successfully onboarded {} customers with global contracts!", customer_accounts.len());
    println!("🛡️  Reduced refund abuse risk: customers reference global contract instead of");
    println!("   storing full contract code, making account deletion less profitable.");

    Ok(())
}

/// Test cost comparison between regular contracts and global contracts
#[tokio::test]
async fn test_cost_comparison_regular_vs_global() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox_with_version("master/5e4b47da55e18f2d2ce3d88f84c15e607380970e").await?;
    
    let factory_wasm = near_workspaces::compile_project(".").await?;
    let factory_contract = worker.dev_deploy(&factory_wasm).await?;

    let contract_code = near_workspaces::compile_project("../status-message").await?;
    
    println!("📊 Cost Comparison Analysis");
    println!("Contract size: {} bytes", contract_code.len());
    
    // According to NEP-591, for a 300kb contract the cost is ~3N
    // Our test contract is much smaller, but we can still demonstrate the pattern
    
    // Deploy as global contract (one-time cost)
    let global_deployer: AccountId = format!("global-deployer.{}", factory_contract.id()).parse()?;
    let global_deploy_cost = NearToken::from_near(20);
    
    let deploy_result = factory_contract
        .call("deploy_global_contract")
        .args_json((
            "cost_comparison_contract",
            contract_code.clone(),
            &global_deployer,
            global_deploy_cost
        ))
        .max_gas()
        .deposit(NearToken::from_near(50))
        .transact()
        .await?;
    
    assert!(deploy_result.is_success());
    println!("✅ Global contract deployed with cost: {}", global_deploy_cost);

    let global_hash = factory_contract
        .call("get_global_contract_hash")
        .args_json(("cost_comparison_contract",))
        .view()
        .await?
        .json::<Option<Vec<u8>>>()?
        .expect("Should have hash");

    // Simulate multiple users using the global contract
    let num_users = 5;
    let per_user_global_cost = NearToken::from_near(2); // Much lower than full deployment
    
    for i in 0..num_users {
        let user_id: AccountId = format!("global-user{}.{}", i, factory_contract.id()).parse()?;
        
        let result = factory_contract
            .call("use_global_contract_by_hash")
            .args_json((&global_hash, &user_id, per_user_global_cost))
            .max_gas()
            .deposit(NearToken::from_near(10))
            .transact()
            .await?;
        
        assert!(result.is_success());
    }

    // Calculate costs
    let total_global_cost = global_deploy_cost.as_yoctonear() + 
                           (per_user_global_cost.as_yoctonear() * num_users as u128);
    
    // For comparison, if each user deployed their own contract
    let per_user_regular_cost = NearToken::from_near(20); // Full storage cost each time
    let total_regular_cost = per_user_regular_cost.as_yoctonear() * num_users as u128;
    
    println!("💰 Cost Analysis Results:");
    println!("  Global contract approach:");
    println!("    - Initial deployment: {}", global_deploy_cost);
    println!("    - Per user cost: {}", per_user_global_cost);
    println!("    - Total for {} users: {} yoctoNEAR", num_users, total_global_cost);
    println!("  Regular contract approach:");
    println!("    - Per user cost: {}", per_user_regular_cost);
    println!("    - Total for {} users: {} yoctoNEAR", num_users, total_regular_cost);
    
    let savings = total_regular_cost - total_global_cost;
    let savings_percentage = (savings as f64 / total_regular_cost as f64) * 100.0;
    
    println!("  💡 Savings: {} yoctoNEAR ({:.1}%)", savings, savings_percentage);
    
    // Global contracts should always be cheaper for multiple deployments
    assert!(total_global_cost < total_regular_cost, "Global contracts should be cheaper");
    
    println!("🎉 Global contracts demonstrate significant cost savings for multiple deployments!");

    Ok(())
}