//! This contract implements simple counter backed by storage on blockchain.
//!
//! The contract provides methods to [increment] / [decrement] counter and
//! [get it's current value][get_num] or [reset].
//!
//! [increment]: struct.Counter.html#method.increment
//! [decrement]: struct.Counter.html#method.decrement
//! [get_num]: struct.Counter.html#method.get_num
//! [reset]: struct.Counter.html#method.reset

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Balance, Promise};
use near_sdk::collections::{ UnorderedMap};
//use near_sdk::json_types::{U128};
use serde::Serialize;
use serde::Deserialize;
use near_sdk::json_types::{ValidAccountId, U128};
//use near_sdk::env::is_valid_account_id;

near_sdk::setup_alloc!();

pub const VAULT_FEE: u128 = 500;

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DomainPublished {
    id: i128,
    domain: AccountId,
    user_seller: AccountId,
    price: Balance,
    is_active: bool, 
    date_fech: String,
    date_year: String,
    date_month: String,
    date_day: String
}

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DomainPurchased {
    id: i128,
    domain: AccountId,
    user_seller: AccountId,
    owner_id: AccountId,
    purchase_price: Balance,
    retired: bool,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    id_domain: i128,
    domains_published: UnorderedMap<i128, DomainPublished>,
    domains_purchased: Vec<DomainPurchased>,
    vault_id: AccountId,
    administrators: Vec<AccountId>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(vault_id: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            id_domain: 0,
            domains_published: UnorderedMap::new(b"s".to_vec()),
            //domains_purchased: UnorderedMap::new(b"s".to_vec()),
            domains_purchased: Vec::new(),
            vault_id: vault_id.to_string(),
            administrators: vec![
                                    "nearbase.testnet".to_string(),
                                    "juanochando.testnet".to_string(),
                                ],
        }
    }

    pub fn set_admin(&mut self, user_id: AccountId) {      
        self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("Only administrators can set categories");
        let valid = self.administrators.iter().find(|&x| x == &user_id);
        if valid.is_some() {
            env::panic(b"the user is already in the list of administrators");
        }
        self.administrators.push(user_id);
    }

    pub fn delete_admin(&mut self, user_id: AccountId) {      
        self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("Only administrators can set categories");
        let index = self.administrators.iter().position(|x| x == &user_id.to_string()).expect("the user is not in the list of administrators");
        self.administrators.remove(index);
    }

    pub fn publish_domain(&mut self, domain: AccountId, user_seller: AccountId, price: U128, date_fech: String, date_year: String, date_month: String, date_day: String) -> DomainPublished {      
        self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("NearBase: Only administrators can publish domains");

        self.id_domain += 1;

        let data = DomainPublished {
            id: self.id_domain,
            domain: domain.to_string(),
            user_seller: user_seller.to_string(),
            price: price.0,
            is_active: true,
            date_fech: date_fech,
            date_year: date_year,
            date_month: date_month,
            date_day: date_day
        };

        self.domains_published.insert(&self.id_domain, &data);
        env::log(b"published domain");
        data
    }

    pub fn update_domain(&mut self, id: i128, price: U128, is_active: bool) -> DomainPublished {      
        //self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("NearBase: Only administrators can publish domains");
        let domain = self.domains_published.get(&id).expect("NearBase: Domain does not exist");
        
        if domain.user_seller == env::signer_account_id() {
            let data = DomainPublished {
                id: domain.id,
                domain: domain.domain,
                user_seller: domain.user_seller,
                price: price.0,
                is_active: is_active,
                date_fech: domain.date_fech,
                date_year: domain.date_year,
                date_month: domain.date_month,
                date_day: domain.date_day
            };
    
            self.domains_published.insert(&domain.id, &data);
            env::log(b"NearBase: Update domain");
            data
        } else {
            env::panic(b"NearBase: No permission")
        }
    }

    #[payable]
    pub fn domain_buy(
        &mut self, 
        id: i128,
    ) {
        let initial_storage_usage = env::storage_usage();
        let domain = self.domains_published.get(&id).expect("NearBase: Domain does not exist");

        if domain.is_active == true {
            let price: Balance = domain.price;
            let attached_deposit = env::attached_deposit();

            assert!(
                attached_deposit >= price,
                "NearBase: attached deposit is less than price : {}",
                price
            );

            let for_vault = price as u128 * VAULT_FEE / 10_000u128;
            let price_deducted = price - for_vault;

            Promise::new(domain.user_seller.to_string()).transfer(price_deducted);
            Promise::new(self.vault_id.clone()).transfer(for_vault);
            
            refund_deposit(env::storage_usage() - initial_storage_usage, price);

            let data = DomainPurchased {
                id: id,
                domain: domain.domain.clone(),
                user_seller: domain.user_seller.clone(),
                owner_id: env::signer_account_id().to_string(),
                purchase_price: domain.price.clone(),
                retired: false,
            };
            self.domains_purchased.push(data);
            self.domains_published.remove(&id);        
        } else {
            env::panic(b"NearBase: Domain is not active");
        }
    }

    pub fn get_domains_published(
        self,
        user_seller: Option<AccountId>,
    ) -> Vec<DomainPublished> {
        if user_seller.is_some() {
            self.domains_published.iter().filter(|(_k, x)| x.user_seller == user_seller.clone().unwrap().to_string()).map(|(_k, x)| DomainPublished {
                id: x.id,
                domain: x.domain.to_string(),
                user_seller: x.user_seller.to_string(),
                price: x.price,
                is_active: x.is_active, 
                date_fech: x.date_fech,
                date_year: x.date_year,
                date_month: x.date_month,
                date_day: x.date_day
            }).collect()
        } else {
            env::panic(b"NearBase: Not user");
        }
    }

    pub fn get_domains_purchased(
        self,
        owner_id: Option<AccountId>,
        user_seller: Option<AccountId>,
    ) -> Vec<DomainPurchased> {
        let mut result: Vec<DomainPurchased> = self.domains_purchased;

        if owner_id.is_some() {
            result = result.iter().filter(|x| x.owner_id == owner_id.as_ref().unwrap().to_string())
                        .map(|r| DomainPurchased { 
                            id: r.id,
                            domain: r.domain.clone(),
                            user_seller: r.user_seller.clone(),
                            owner_id: r.owner_id.clone(),
                            purchase_price: r.purchase_price,
                            retired: r.retired,
                        }).collect();
        }

        if user_seller.is_some() {
            result = result.iter().filter(|x| x.user_seller == user_seller.as_ref().unwrap().to_string())
                        .map(|r| DomainPurchased { 
                            id: r.id,
                            domain: r.domain.clone(),
                            user_seller: r.user_seller.clone(),
                            owner_id: r.owner_id.clone(),
                            purchase_price: r.purchase_price,
                            retired: r.retired,
                        }).collect();
        }

        result.iter().map(|r| DomainPurchased { 
            id: r.id,
            domain: r.domain.clone(),
            user_seller: r.user_seller.clone(),
            owner_id: r.owner_id.clone(),
            purchase_price: r.purchase_price,
            retired: r.retired,
        }).collect()
    }

    pub fn get_domain_id(
        self,
        id: i128,
    ) -> Vec<DomainPurchased> {
        self.domains_purchased.iter().filter(|x| x.id == id)
        .map(|x| DomainPurchased {
            id: x.id,
            domain: x.domain.to_string(),
            user_seller: x.user_seller.to_string(),
            owner_id: x.owner_id.to_string(),
            purchase_price: x.purchase_price,
            retired: x.retired,
        }).collect()
    }

    pub fn get_market(&self,
    ) -> Vec<DomainPublished> {

        self.domains_published.iter().filter(|(_k, x)| x.is_active == true).map(|(_k, x)| DomainPublished {
            id: x.id,
            domain: x.domain.to_string(),
            user_seller: x.user_seller.to_string(),
            price: x.price,
            is_active: x.is_active, 
            date_fech: x.date_fech,
            date_year: x.date_year,
            date_month: x.date_month,
            date_day: x.date_day
        }).collect()
    }
}

fn refund_deposit(storage_used: u64, extra_spend: Balance) {
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
    let attached_deposit = env::attached_deposit() - extra_spend;

    assert!(
        required_cost <= attached_deposit,
        "Must attach {} yoctoNEAR to cover storage",
        required_cost,
    );

    let refund = attached_deposit - required_cost;
    if refund > 1 {
        Promise::new(env::predecessor_account_id()).transfer(refund);
    }
}

// unlike the struct's functions above, this function cannot use attributes #[derive(…)] or #[near_bindgen]
// any attempts will throw helpful warnings upon 'cargo build'
// while this function cannot be invoked directly on the blockchain, it can be called from an invoked function

/*
 * the rest of this file sets up unit tests
 * to run these, the command will be:
 * cargo test --package rust-counter-tutorial -- --nocapture
 * Note: 'rust-counter-tutorial' comes from cargo.toml's 'name' key
 */

// use the attribute below for unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    // part of writing unit tests is setting up a mock context
    // in this example, this is only needed for env::log in the contract
    // this is also a useful list to peek at when wondering what's available in env::*
    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice.testnet".to_string(),
            signer_account_id: "robert.testnet".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "jane.testnet".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 19,
        }
    }

    // mark individual unit tests with #[test] for them to be registered and fired
    #[test]
    fn increment() {
        // set up the mock context into the testing environment
        let context = get_context(vec![], false);
        testing_env!(context);
        // instantiate a contract variable with the counter at zero
        let mut contract = Counter { val: 0 };
        contract.increment();
        println!("Value after increment: {}", contract.get_num());
        // confirm that we received 1 when calling get_num
        assert_eq!(1, contract.get_num());
    }

    #[test]
    fn decrement() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = Counter { val: 0 };
        contract.decrement();
        println!("Value after decrement: {}", contract.get_num());
        // confirm that we received -1 when calling get_num
        assert_eq!(-1, contract.get_num());
    }

    #[test]
    fn increment_and_reset() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = Counter { val: 0 };
        contract.increment();
        contract.reset();
        println!("Value after reset: {}", contract.get_num());
        // confirm that we received -1 when calling get_num
        assert_eq!(0, contract.get_num());
    }
}