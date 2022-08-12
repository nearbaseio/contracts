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
    post_type: i8,
    is_active: bool, 
    date_time: u64,
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
    post_type: i8,
    date_time: u64,
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

    pub fn publish_domain(&mut self, domain: AccountId, user_seller: AccountId, price: U128) -> DomainPublished {      
        self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("NearBase: Only administrators can publish domains");

        self.id_domain += 1;

        let data = DomainPublished {
            id: self.id_domain,
            domain: domain.to_string(),
            user_seller: user_seller.to_string(),
            price: price.0,
            post_type: 1,
            is_active: true,
            date_time: env::block_timestamp(),
        };

        self.domains_published.insert(&self.id_domain, &data);
        env::log(b"published domain");
        data
    }

    #[payable]
    pub fn resell_domain(
        &mut self, 
        id: i128,
        price: U128,
        post_type: i8,
    ) -> DomainPublished {  
        let initial_storage_usage = env::storage_usage();  
        let domain = self.domains_purchased.iter().find(|&x| x.id == id).expect("NearBase: Domain does not exist");

        if domain.owner_id == env::signer_account_id() && domain.retired == false {
            if post_type == 1 {
                let data = DomainPublished {
                    id: domain.id,
                    domain: domain.domain.clone(),
                    user_seller: domain.owner_id.clone(),
                    price: price.0,
                    post_type: post_type,
                    is_active: true,
                    date_time: env::block_timestamp(),
                };
        
                self.domains_published.insert(&domain.id, &data);

                let index = self.domains_purchased.iter().position(|x| x.id == id).expect("Not domain");
                self.domains_purchased.remove(index);
                env::log(b"NearBase: Published domain");
                data
            } else if post_type == 2 {
                let deposit_premium: Balance = 1000000000000000000000000;
                let attached_deposit = env::attached_deposit();

                assert!(
                    attached_deposit >= deposit_premium,
                    "NearBase: attached deposit is less than deposit_premium : {}",
                    deposit_premium
                );

                Promise::new(self.vault_id.clone()).transfer(deposit_premium);
                refund_deposit(env::storage_usage() - initial_storage_usage, deposit_premium);

                let data = DomainPublished {
                    id: domain.id,
                    domain: domain.domain.clone(),
                    user_seller: domain.owner_id.clone(),
                    price: price.0,
                    post_type: post_type,
                    is_active: true,
                    date_time: env::block_timestamp(),
                };
        
                self.domains_published.insert(&domain.id, &data);

                let index = self.domains_purchased.iter().position(|x| x.id == id).expect("Not domain");
                self.domains_purchased.remove(index);
                env::log(b"NearBase: Published domain");
                data
            } else {
                env::panic(b"NearBase: Post type not allowed")
            }
        } else {
            env::panic(b"NearBase: No permission")
        }
    }

    #[payable]
    pub fn update_domain(&mut self, id: i128, price: U128, is_active: bool, post_type: i8) -> DomainPublished {      
        let initial_storage_usage = env::storage_usage();
        let domain = self.domains_published.get(&id).expect("NearBase: Domain does not exist");
        
        if domain.user_seller == env::signer_account_id() {
            let data = DomainPublished {
                id: domain.id,
                domain: domain.domain,
                user_seller: domain.user_seller,
                price: price.0,
                is_active: is_active,
                post_type: post_type,
                date_time: domain.date_time,
            };

            if domain.post_type == 2 || post_type == 1{
                self.domains_published.insert(&domain.id, &data);
                env::log(b"NearBase: Update domain");
                data
            } else if domain.post_type == 1 && post_type == 2{
                let deposit_premium: Balance = 1000000000000000000000000;
                let attached_deposit = env::attached_deposit();

                assert!(
                    attached_deposit >= deposit_premium,
                    "NearBase: attached deposit is less thans deposit_premium : {}",
                    deposit_premium
                );

                Promise::new(self.vault_id.clone()).transfer(deposit_premium);
                refund_deposit(env::storage_usage() - initial_storage_usage, deposit_premium);

                self.domains_published.insert(&domain.id, &data);
                env::log(b"NearBase: Update domain");
                data

            } else {
                env::panic(b"NearBase: Post type not allowed")
            }
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
                post_type: domain.post_type,
                retired: false,
                date_time: env::block_timestamp()
            };
            self.domains_purchased.push(data);
            self.domains_published.remove(&id);        
        } else {
            env::panic(b"NearBase: Domain is not active");
        }
    }

    pub fn retired_domain(&mut self, id: i128) -> DomainPurchased {      
        //self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("NearBase: Only administrators can publish domains");
        let index = self.domains_purchased.iter().position(|x| x.id == id).expect("Domain no exists");
        
        if self.domains_purchased[index].owner_id == env::signer_account_id() {
            self.domains_purchased[index].retired = true;
            self.domains_purchased[index].clone()
        } else {
            env::panic(b"NearBase: No permission")
        }
    }

    pub fn cancel_domain(&mut self, id: i128) {      
        //self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("NearBase: Only administrators can publish domains");
        let domain = self.domains_published.get(&id).expect("NearBase: Domain does not exist");
        
        if domain.user_seller == env::signer_account_id() {
            self.domains_published.remove(&domain.id);  
            env::log(b"removed domain"); 
        } else {
            env::panic(b"NearBase: No permission")
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
                post_type: x.post_type,
                date_time: x.date_time,
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
                            post_type: r.post_type,
                            retired: r.retired,
                            date_time: r.date_time,
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
                            post_type: r.post_type,
                            retired: r.retired,
                            date_time: r.date_time,
                        }).collect();
        }

        result.iter().map(|r| DomainPurchased { 
            id: r.id,
            domain: r.domain.clone(),
            user_seller: r.user_seller.clone(),
            owner_id: r.owner_id.clone(),
            purchase_price: r.purchase_price,
            post_type: r.post_type,
            retired: r.retired,
            date_time: r.date_time,
        }).collect()
    }

    pub fn get_top_published(&self, top: Option<i32>) -> Vec<DomainPublished> {
        let top_limit = top.unwrap_or(5);

        let mut top_domains: Vec<DomainPublished> = self.domains_published.iter().filter(|(_k, x)| x.is_active == true).map(|(_k, x)| DomainPublished {
            id: x.id,
            domain: x.domain.to_string(),
            user_seller: x.user_seller.to_string(),
            price: x.price,
            post_type: x.post_type,
            is_active: x.is_active, 
            date_time: x.date_time,
        }).collect();

        top_domains.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        
        top_domains.iter()
        .take(top_limit as usize)
        .map(|x| DomainPublished {
            id: x.id,
            domain: x.domain.to_string(),
            user_seller: x.user_seller.to_string(),
            price: x.price,
            post_type: x.post_type,
            is_active: x.is_active, 
            date_time: x.date_time,
        }).collect()
    }

    pub fn get_top_purchased(&self, top: Option<i32>) -> Vec<DomainPurchased> {
        let top_limit = top.unwrap_or(5);

        let mut top_domains: Vec<DomainPurchased> = self.domains_purchased.clone();

        top_domains.sort_by(|a, b| b.purchase_price.partial_cmp(&a.purchase_price).unwrap());
        
        top_domains.iter()
        .take(top_limit as usize)
        .map(|x| DomainPurchased {
            id: x.id,
            domain: x.domain.to_string(),
            user_seller: x.user_seller.to_string(),
            owner_id: x.owner_id.to_string(),
            purchase_price: x.purchase_price,
            post_type: x.post_type,
            retired: x.retired,
            date_time: x.date_time,
        }).collect()
    }

    pub fn get_last_sold(&self,
        number_domains: i128,
    ) -> Vec<DomainPurchased> {

        if self.domains_purchased.len() as i128 > number_domains {
            let index: i128 = self.domains_purchased.len() as i128 - number_domains;
            let result: Vec<DomainPurchased> = self.domains_purchased.clone();

            result.iter()
            .skip(index as usize)
            .map(|x| DomainPurchased {
                id: x.id,
                domain: x.domain.clone(),
                user_seller: x.user_seller.clone(),
                owner_id: x.owner_id.clone(),
                purchase_price: x.purchase_price,
                post_type: x.post_type,
                retired: x.retired,
                date_time: x.date_time,
            }).collect()
        } else {
            self.domains_purchased.iter().map(|x| DomainPurchased {
                id: x.id,
                domain: x.domain.clone(),
                user_seller: x.user_seller.clone(),
                owner_id: x.owner_id.clone(),
                purchase_price: x.purchase_price,
                post_type: x.post_type,
                retired: x.retired,
                date_time: x.date_time,
            }).collect()
        }  
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
            post_type: x.post_type,
            retired: x.retired,
            date_time: x.date_time,
        }).collect()
    }

    pub fn get_domain_forsale(
        self,
        id: i128,
    ) -> Vec<DomainPublished> {
        self.domains_published.iter().filter(|(_k, x)| x.id == id).map(|(_k, x)| DomainPublished {
            id: x.id,
            domain: x.domain.to_string(),
            user_seller: x.user_seller.to_string(),
            price: x.price,
            post_type: x.post_type,
            is_active: x.is_active, 
            date_time: x.date_time,
        }).collect()
    }

    pub fn get_market(&self,
    ) -> Vec<DomainPublished> {

        let mut domains: Vec<DomainPublished> = self.domains_published.iter().filter(|(_k, x)| x.is_active == true).map(|(_k, x)| DomainPublished {
            id: x.id,
            domain: x.domain.to_string(),
            user_seller: x.user_seller.to_string(),
            price: x.price,
            post_type: x.post_type,
            is_active: x.is_active, 
            date_time: x.date_time,
        }).collect();

        domains.sort_by(|a, b| b.post_type.partial_cmp(&a.post_type).unwrap());

        domains
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