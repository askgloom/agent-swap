//! Solana-specific utilities and client setup
//! 
//! Provides functions for interacting with the Solana blockchain.

use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Keypair, read_keypair_file},
        transaction::Transaction,
    },
    Client, Program,
};
use std::{path::Path, str::FromStr};
use anyhow::Result;

/// Setup Solana RPC client
pub fn setup_client(
    url: &str,
    commitment: CommitmentConfig,
) -> Result<Client> {
    let client = Client::new_with_options(
        url.to_string(),
        Keypair::new(),  // Payer, replaced in actual transactions
        commitment,
    );
    Ok(client)
}

/// Load wallet from file or generate new one
pub fn setup_wallet<P: AsRef<Path>>(
    path: Option<P>,
) -> Result<Keypair> {
    match path {
        Some(path) => {
            Ok(read_keypair_file(path)?)
        }
        None => {
            Ok(Keypair::new())
        }
    }
}

/// Get token balance for an account
pub async fn get_token_balance(
    client: &Client,
    account: &Pubkey,
) -> Result<u64> {
    let balance = client
        .get_token_account_balance(account)?
        .ui_amount_u64;
    Ok(balance)
}

/// Sign and send transaction
pub async fn send_and_confirm_transaction(
    client: &Client,
    transaction: Transaction,
    signers: &[&Keypair],
) -> Result<String> {
    let signature = client
        .send_and_confirm_transaction_with_signers(&transaction, signers)?;
    Ok(signature.to_string())
}

/// Get SOL balance
pub async fn get_sol_balance(
    client: &Client,
    pubkey: &Pubkey,
) -> Result<u64> {
    let balance = client.get_balance(pubkey)?;
    Ok(balance)
}

/// Ensure sufficient SOL for fees
pub async fn ensure_sol_for_fees(
    client: &Client,
    wallet: &Keypair,
    minimum_balance: u64,
) -> Result<()> {
    let balance = get_sol_balance(client, &wallet.pubkey()).await?;
    if balance < minimum_balance {
        anyhow::bail!("Insufficient SOL for fees");
    }
    Ok(())
}

/// Create associated token account if needed
pub async fn create_associated_token_account_idempotent(
    client: &Client,
    wallet: &Keypair,
    mint: &Pubkey,
) -> Result<Pubkey> {
    let ata = spl_associated_token_account::get_associated_token_address(
        &wallet.pubkey(),
        mint,
    );

    if client.get_account(&ata).is_err() {
        let ix = spl_associated_token_account::instruction::create_associated_token_account(
            &wallet.pubkey(),
            &wallet.pubkey(),
            mint,
            &spl_token::id(),
        );

        let transaction = Transaction::new_with_payer(
            &[ix],
            Some(&wallet.pubkey()),
        );

        send_and_confirm_transaction(client, transaction, &[wallet]).await?;
    }

    Ok(ata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_setup() {
        let wallet = setup_wallet::<String>(None).unwrap();
        assert!(wallet.pubkey() != Pubkey::default());
    }

    #[tokio::test]
    async fn test_client_setup() {
        let client = setup_client(
            "https://api.mainnet-beta.solana.com",
            CommitmentConfig::confirmed(),
        ).unwrap();
        
        // Test connection
        let version = client.get_version().unwrap();
        assert!(version.feature_set > 0);
    }
}