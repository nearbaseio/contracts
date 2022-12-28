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
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Balance, Promise, serde_json::json,
    BorshStorageKey };
use near_sdk::collections::{ UnorderedMap, UnorderedSet };
//use near_sdk::json_types::{U128};
use serde::Serialize;
use serde::Deserialize;
use near_sdk::json_types::{U128};
//use near_sdk::env::is_valid_account_id;

//near_sdk::setup_alloc!();

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
    owner_id: AccountId,
    id_domain: i128,
    domains_published: UnorderedMap<i128, DomainPublished>,
    domains_purchased: UnorderedMap<i128, DomainPurchased>,
    vault_id: AccountId,
    administrators: UnorderedSet<AccountId>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    PublishedKey,
    PurchasedKey,
    AdminKey,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, vault_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id: owner_id,
            id_domain: 0,
            domains_published: UnorderedMap::new(StorageKey::PublishedKey),
            domains_purchased: UnorderedMap::new(StorageKey::PurchasedKey),
            vault_id: vault_id,
            administrators: UnorderedSet::new(StorageKey::AdminKey)
        }
    }

    pub fn set_admin(&mut self, user_id: AccountId) {      
        assert!(self.owner_id == env::signer_account_id() || self.administrators.contains(&env::signer_account_id()), "Only administrators can set categories");
        if self.administrators.contains(&user_id) {
            env::panic_str("the user is already in the list of administrators");
        }
        self.administrators.insert(&user_id);
    }

    pub fn delete_admin(&mut self, user_id: AccountId) {      
        assert!(self.owner_id == env::signer_account_id() || self.administrators.contains(&env::signer_account_id()), "Only administrators can set categories");
        self.administrators.remove(&user_id);
    }

    pub fn publish_domain(&mut self, domain: AccountId, user_seller: AccountId, price: U128) -> DomainPublished {      
        assert!(self.owner_id == env::signer_account_id() || self.administrators.contains(&env::signer_account_id()), "NearBase: Only administrators can publish domains");

        self.id_domain += 1;

        let data = DomainPublished {
            id: self.id_domain,
            domain: domain,
            user_seller: user_seller,
            price: price.0,
            post_type: 1,
            is_active: true,
            date_time: env::block_timestamp(),
        };

        self.domains_published.insert(&self.id_domain, &data.clone());
        env::log_str(
            &json!({
                "type": "publish_domain",
                "params": {
                    "id": data.id.to_string(),
                    "domain": data.domain.to_string(),
                    "user_seller": data.user_seller.to_string(),
                    "price": data.price.to_string(),
                    "post_type": 1,
                    "is_active": true,
                    "date_time": data.date_time.to_string(),
                }
            }).to_string()
        );
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
        let domain = self.domains_purchased.get(&id).expect("NearBase: Domain does not exist");

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

                self.domains_purchased.remove(&id);
                env::log_str(
                    &json!({
                        "type": "resell_domain",
                        "params": {
                            "id": data.id.to_string(),
                            "domain": data.domain.to_string(),
                            "user_seller": data.user_seller.to_string(),
                            "price": data.price.to_string(),
                            "post_type": data.post_type.to_string(),
                            "is_active": data.is_active,
                            "date_time": data.date_time.to_string(),
                        }
                    }).to_string()
                );
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

                self.domains_purchased.remove(&id);

                env::log_str(
                    &json!({
                        "type": "resell_domain",
                        "params": {
                            "id": data.id.to_string(),
                            "domain": data.domain.to_string(),
                            "user_seller": data.user_seller.to_string(),
                            "price": data.price.to_string(),
                            "post_type": data.post_type.to_string(),
                            "is_active": data.is_active,
                            "date_time": data.date_time.to_string(),
                        }
                    }).to_string()
                );
                data
            } else {
                env::panic_str("NearBase: Post type not allowed")
            }
        } else {
            env::panic_str("NearBase: No permission")
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
                env::log_str(
                    &json!({
                        "type": "update_domain",
                        "params": {
                            "id": data.id.to_string(),
                            "domain": data.domain.to_string(),
                            "user_seller": data.user_seller.to_string(),
                            "price": data.price.to_string(),
                            "post_type": data.post_type.to_string(),
                            "is_active": data.is_active,
                            "date_time": data.date_time.to_string(),
                        }
                    }).to_string()
                );
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
                env::log_str(
                    &json!({
                        "type": "update_domain",
                        "params": {
                            "id": data.id.to_string(),
                            "domain": data.domain.to_string(),
                            "user_seller": data.user_seller.to_string(),
                            "price": data.price.to_string(),
                            "post_type": data.post_type.to_string(),
                            "is_active": data.is_active,
                            "date_time": data.date_time.to_string(),
                        }
                    }).to_string()
                );
                data

            } else {
                env::panic_str("NearBase: Post type not allowed")
            }
        } else {
            env::panic_str("NearBase: No permission")
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

            Promise::new(domain.user_seller.clone()).transfer(price_deducted);
            Promise::new(self.vault_id.clone()).transfer(for_vault);
            
            refund_deposit(env::storage_usage() - initial_storage_usage, price);

            let data = DomainPurchased {
                id: id,
                domain: domain.domain.clone(),
                user_seller: domain.user_seller.clone(),
                owner_id: env::signer_account_id(),
                purchase_price: domain.price.clone(),
                post_type: domain.post_type,
                retired: false,
                date_time: env::block_timestamp()
            };
            self.domains_purchased.insert(&id, &data.clone());
            self.domains_published.remove(&id);
            env::log_str(
                &json!({
                    "type": "domain_buy",
                    "params": {
                        "id": data.id.to_string(),
                        "domain": data.domain.to_string(),
                        "user_seller": data.user_seller.to_string(),
                        "owner_id": data.owner_id.to_string(),
                        "purchase_price": data.purchase_price.to_string(),
                        "post_type": data.post_type.to_string(),
                        "retired": data.retired,
                        "date_time": data.date_time.to_string(),
                    }
                }).to_string()
            );
        } else {
            env::panic_str("NearBase: Domain is not active");
        }
    }

    pub fn retired_domain(&mut self, id: i128) -> DomainPurchased {
        //self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("NearBase: Only administrators can publish domains");
        let mut domine = self.domains_purchased.get(&id).expect("Domain no exists");
        
        if domine.owner_id == env::signer_account_id() {
            domine.retired = true;
            self.domains_purchased.insert(&id, &domine);
            env::log_str(
                &json!({
                    "type": "retired_domain",
                    "params": {
                        "id": id.to_string(),
                        "owner_id": env::signer_account_id().to_string(),
                        "retired": true,
                        "date_time": env::block_timestamp().to_string(),
                    }
                }).to_string()
            );
            domine.clone()
        } else {
            env::panic_str("NearBase: No permission")
        }
    }

    pub fn cancel_domain(&mut self, id: i128) {      
        //self.administrators.iter().find(|&x| x == &env::signer_account_id()).expect("NearBase: Only administrators can publish domains");
        let domain = self.domains_published.get(&id).expect("NearBase: Domain does not exist");
        
        if domain.user_seller == env::signer_account_id() {
            self.domains_published.remove(&id);  
            env::log_str(
                &json!({
                    "type": "cancel_domain",
                    "params": {
                        "id": id.to_string(),
                        "user_seller": env::signer_account_id().to_string(),
                        "date_time": env::block_timestamp().to_string(),
                    }
                }).to_string()
            ); 
        } else {
            env::panic_str("NearBase: No permission")
        }
    }

    pub fn get_domains_published(
        self,
        user_seller: Option<AccountId>,
    ) -> Vec<DomainPublished> {
        if user_seller.is_some() {
            self.domains_published.iter().filter(|(_k, x)| x.user_seller == user_seller.clone().unwrap()).map(|(_k, x)| DomainPublished {
                id: x.id,
                domain: x.domain.clone(),
                user_seller: x.user_seller.clone(),
                price: x.price,
                is_active: x.is_active, 
                post_type: x.post_type,
                date_time: x.date_time,
            }).collect()
        } else {
            env::panic_str("NearBase: Not user");
        }
    }

    pub fn get_domains_purchased(
        self,
        owner_id: Option<AccountId>,
        user_seller: Option<AccountId>,
    ) -> Vec<DomainPurchased> {
        let mut result: Vec<DomainPurchased> = [].to_vec();
        let data = self.domains_purchased;
        if owner_id.is_some() {
            if result.len() > 0 { 
                result = result.iter().filter(|x| x.owner_id == owner_id.clone().unwrap()).map(|r| r.clone()).collect();
            } else {
                result = data.iter().filter(|(_k, x)| x.owner_id == owner_id.clone().unwrap()).map(|(_k, r)| r.clone()).collect();
            }
        }

        if user_seller.is_some() {
            if result.len() > 0 {
                result = result.iter().filter(|x| x.user_seller == user_seller.clone().unwrap()).map(|r| r.clone()).collect();
            } else {
                result = data.iter().filter(|(_k, x)| x.user_seller == user_seller.clone().unwrap()).map(|(_k, r)| r.clone()).collect();
            }
        }

        result.iter().map(|r| r.clone()).collect()
    }

    pub fn get_top_published(&self, top: Option<i32>) -> Vec<DomainPublished> {
        let top_limit = top.unwrap_or(5);

        let mut top_domains: Vec<DomainPublished> = self.domains_published.iter().filter(|(_k, x)| x.is_active == true).map(|(_k, x)| x.clone()).collect();

        top_domains.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        
        top_domains.iter()
        .take(top_limit as usize)
        .map(|x| x.clone()).collect()
    }

    pub fn get_top_purchased(&self, top: Option<i32>) -> Vec<DomainPurchased> {
        let top_limit = top.unwrap_or(5);

        let mut top_domains: Vec<DomainPurchased> = self.domains_purchased.iter().map(|(_k, x)| x.clone()).collect();

        top_domains.sort_by(|a, b| b.purchase_price.partial_cmp(&a.purchase_price).unwrap());
        
        top_domains.iter()
        .take(top_limit as usize)
        .map(|x| x.clone()).collect()
    }

    pub fn get_last_sold(&self,
        number_domains: i128,
    ) -> Vec<DomainPurchased> {

        if self.domains_purchased.len() as i128 > number_domains {
            let index: i128 = self.domains_purchased.len() as i128 - number_domains;
            let result: Vec<DomainPurchased> = self.domains_purchased.iter().map(|(_k, x)| x.clone()).collect();

            result.iter()
            .skip(index as usize)
            .map(|x| x.clone()).collect()
        } else {
            self.domains_purchased.iter().map(|(_k, x)| x.clone()).collect()
        }  
    }

    pub fn get_domain_id(
        self,
        id: i128,
    ) -> Vec<DomainPurchased> {
        self.domains_purchased.iter().filter(|(_k, x)| x.id == id)
        .map(|(_k, x)| x.clone()).collect()
    }

    pub fn get_domain_forsale(
        self,
        id: i128,
    ) -> Vec<DomainPublished> {
        self.domains_published.iter().filter(|(_k, x)| x.id == id).map(|(_k, x)| x.clone()).collect()
    }

    pub fn get_market(&self,
    ) -> Vec<DomainPublished> {
        let mut domains: Vec<DomainPublished> = self.domains_published.iter().filter(|(_k, x)| x.is_active == true).map(|(_k, x)| x.clone()).collect();

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