use std::u128;

use account::Account;
use env::predecessor_account_id;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{UnorderedMap, UnorderedSet, Vector},
    env,
    json_types::U128,
    near_bindgen, wee_alloc, AccountId, Promise,
};
mod account;
mod config;
mod error;
mod nep21;
mod refund_storage;

use error::Error;
use refund_storage::refund_storage;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// As found on https://near.github.io/near-api-js/modules/_utils_format_.html
const NEAR_NOMINATION_EXP: u32 = 24;

pub type CID = String;
const MAX_CID_LEN: usize = 64;
const NUMB_DECS: u32 = NEAR_NOMINATION_EXP + 12;
const DEFAULT_COST_PER_TOKEN: u128 = 1_000;

pub trait FungibleTokenTrait {
    fn inc_allowance(&mut self, art: CID, escrow_account_id: AccountId, amount: U128);
    fn dec_allowance(&mut self, art: CID, escrow_account_id: AccountId, amount: U128);
    fn transfer_from(
        &mut self,
        art: CID,
        owner_id: AccountId,
        new_owner_id: AccountId,
        amount: U128,
    );
    fn transfer(&mut self, art: CID, new_owner_id: AccountId, amount: U128);
    fn get_total_supply(&self, art: CID) -> U128;
    fn get_balance(&self, art: CID, owner_id: AccountId) -> U128;
    fn get_allowance(&self, art: CID, owner_id: AccountId, escrow_account_id: AccountId) -> U128;
    fn get_decimals_per_token(&self) -> u32;
}

pub trait Sale {
    fn put_on_sale(&mut self, art: CID, numb_coins: U128);
    fn get_amount_on_sale(&self, art: CID, seller: AccountId) -> U128;
    fn buy(&mut self, art: CID, numb_coins: u128, token_owner: AccountId);
    fn change_cost(&mut self, art: CID, cost_per_token: u128);
    fn cost_per_token(&self, art: CID) -> u128;
    fn get_all_sellers(&self, art: CID) -> Vec<(AccountId, u128)>;
}

pub trait TokenFactTrait {
    fn create_token(&mut self, artwork: CID, original: CID);
}

pub trait DesignTrait {
    fn get_artwork_cid_of_original_cid(&self, original_cid: CID) -> Option<CID>;
    fn get_designs(&self, artist: AccountId) -> Vec<CID>;
    fn get_design_tokens(&self, user: AccountId) -> Vec<(CID, U128)>;
}

pub trait Proile {
    fn update_profile(&mut self, profile: CID);
    fn get_profile(&self, artist: CID) -> CID;
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Token {
    artist: AccountId,
    artwork: CID,
    sellers: UnorderedSet<AccountId>,
    token: nep21::FungibleToken,
    // In near
    cost_per_token: u128,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct FurBall {
    total_supply_new_tok: U128,
    original_cid_to_artwork_cid: UnorderedMap<CID, CID>,
    artist_to_artist_cid: UnorderedMap<AccountId, CID>,
    art_cid_to_token: UnorderedMap<CID, Token>,
    art_cids: UnorderedSet<CID>,
}

impl Default for FurBall {
    fn default() -> Self {
        panic!("FurBall must be intialized before use!")
    }
}

/// Constructor implementation
#[near_bindgen]
impl FurBall {
    #[init]
    pub fn new() -> Self {
        let owner_id = env::predecessor_account_id();
        let total_supply_new_tok = 1_000_000_000.into();
        assert!(
            env::is_valid_account_id(owner_id.as_bytes()),
            "Owner's account ID is invalid."
        );
        assert!(!env::state_exists(), "Already initialized");
        let fb = Self {
            artist_to_artist_cid: UnorderedMap::new(b"artistCID-belongs-to".to_vec()),
            original_cid_to_artwork_cid: UnorderedMap::new(b"originalCID-of-artworkCID".to_vec()),
            art_cid_to_token: UnorderedMap::new(b"artCID-of-token".to_vec()),
            art_cids: UnorderedSet::new(b"all-art-cids".to_vec()),
            total_supply_new_tok,
        };
        fb
    }
}

/// Private function implementation
impl FurBall {
    fn get_art(&self, art: &CID) -> Result<Token, Error> {
        self.art_cid_to_token
            .get(&art)
            .ok_or(Error::ArtCIDNotFound(art.to_string()))
    }
    fn update_token(&mut self, art: &CID, token: &Token) {
        self.art_cid_to_token.insert(art, token);
    }
}

#[near_bindgen]
impl Sale for FurBall {
    fn get_amount_on_sale(&self, art: CID, seller: AccountId) -> U128 {
        let art_token = self.get_art(&art).unwrap();
        art_token
            .token
            .get_allowance(seller, env::current_account_id())
    }

    #[payable]
    fn put_on_sale(&mut self, art: CID, amount: U128) {
        let mut art_token = self.get_art(&art).unwrap();
        art_token
            .token
            .inc_allowance(env::current_account_id(), amount);
        art_token.sellers.insert(&env::predecessor_account_id());
        self.update_token(&art, &art_token);
    }

    #[payable]
    fn buy(&mut self, art: CID, amount: u128, token_owner: AccountId) {
        let mut art_token = self.get_art(&art).unwrap();
        // art_token.token.co
        let cost = art_token.cost_per_token * amount;
        assert_eq!(
            env::attached_deposit(),
            cost,
            "Please attach a deposit of {} tokens",
            cost
        );
        let account_id = env::predecessor_account_id();
        art_token
            .token
            .transfer_from_contract(token_owner.clone(), account_id, U128(amount));
        self.update_token(&art, &art_token);
        Promise::new(token_owner).transfer(cost);
    }

    #[payable]
    fn change_cost(&mut self, art: CID, cost_per_token: u128) {
        let mut art_tok = self.get_art(&art).unwrap();
        assert_eq!(
            art_tok.artist,
            env::predecessor_account_id(),
            "Sender must own the art"
        );
        art_tok.cost_per_token = cost_per_token;
        self.art_cid_to_token.insert(&art, &art_tok);
    }

    fn cost_per_token(&self, art: CID) -> u128 {
        let art_token = self.get_art(&art).unwrap();
        return art_token.cost_per_token;
    }

    // TODO: test
    fn get_all_sellers(&self, art: CID) -> Vec<(AccountId, u128)> {
        let art_token = self.get_art(&art).unwrap();
        let mut sellers: Vec<(AccountId, u128)> = Vec::new();
        for seller in art_token.sellers.iter() {
            let bal = art_token
                .token
                .get_allowance(seller.clone(), env::current_account_id())
                .0;
            if bal > 0 {
                sellers.push((seller, bal))
            }
        }
        sellers
    }
}

#[near_bindgen]
impl Proile for FurBall {
    #[payable]
    fn update_profile(&mut self, profile: CID) {
        let initial_storage = env::storage_usage();
        self.artist_to_artist_cid
            .insert(&env::predecessor_account_id(), &profile);
        refund_storage(initial_storage);
    }

    fn get_profile(&self, artist: CID) -> CID {
        self.artist_to_artist_cid.get(&artist).unwrap()
    }
}

#[near_bindgen]
impl DesignTrait for FurBall {
    fn get_designs(&self, artist: AccountId) -> Vec<CID> {
        let mut designs: Vec<CID> = Vec::new();
        for art_cid in self.art_cids.iter() {
            if let Some(token) = self.art_cid_to_token.get(&art_cid) {
                if token.artist == artist {
                    designs.push(art_cid.clone());
                }
            }
        }
        designs
    }
    fn get_design_tokens(&self, user: AccountId) -> Vec<(CID, U128)> {
        let mut designs = Vec::new();
        for (art_cid, token) in self.art_cid_to_token.iter() {
            if let Some(account) = token.token.accounts.get(&env::sha256(user.as_bytes())) {
                designs.push((art_cid.clone(), account.balance.into()));
            }
        }
        designs
    }
    fn get_artwork_cid_of_original_cid(&self, original_cid: CID) -> Option<CID> {
        self.original_cid_to_artwork_cid.get(&original_cid)
    }
}

#[near_bindgen]
impl TokenFactTrait for FurBall {
    #[payable]
    fn create_token(&mut self, artwork: CID, original: CID) {
        assert!(
            self.art_cid_to_token.get(&artwork).is_none(),
            format!("Artwork with CID {} cannot already have a token", artwork)
        );
        assert!(
            artwork.len() <= MAX_CID_LEN,
            format!(
                "Artwork CID must be shorter than {} characters",
                MAX_CID_LEN
            )
        );

        let tok = Token {
            artist: env::predecessor_account_id(),
            artwork: artwork.clone(),
            token: nep21::FungibleToken::new(
                env::predecessor_account_id(),
                self.total_supply_new_tok,
                artwork.clone(),
            ),
            sellers: UnorderedSet::new(format!("{}-sellers", artwork).into()),
            cost_per_token: DEFAULT_COST_PER_TOKEN,
        };
        self.original_cid_to_artwork_cid.insert(&original, &artwork);
        self.art_cid_to_token.insert(&artwork, &tok);
        self.art_cids.insert(&artwork);
    }
}

/// Add fungible token functionality for individual tokens
#[near_bindgen]
impl FungibleTokenTrait for FurBall {
    #[payable]
    fn inc_allowance(&mut self, art: CID, escrow_account_id: AccountId, amount: U128) {
        let mut art_token = self.get_art(&art).unwrap();
        art_token.token.inc_allowance(escrow_account_id, amount);
        self.update_token(&art, &art_token);
    }
    #[payable]
    fn dec_allowance(&mut self, art: CID, escrow_account_id: AccountId, amount: U128) {
        let mut art_token = self.get_art(&art).unwrap();
        art_token.token.dec_allowance(escrow_account_id, amount);
        self.update_token(&art, &art_token);
    }
    #[payable]
    fn transfer_from(
        &mut self,
        art: CID,
        owner_id: AccountId,
        new_owner_id: AccountId,
        amount: U128,
    ) {
        let mut art_token = self.get_art(&art).unwrap();
        art_token
            .token
            .transfer_from(owner_id, new_owner_id, amount);
        self.update_token(&art, &art_token);
    }
    #[payable]
    fn transfer(&mut self, art: CID, new_owner_id: AccountId, amount: U128) {
        let mut art_token = self.get_art(&art).unwrap();
        art_token.token.transfer(new_owner_id, amount);
        self.update_token(&art, &art_token);
    }
    fn get_total_supply(&self, art: CID) -> U128 {
        let art_token = self.get_art(&art).unwrap();
        art_token.token.get_total_supply()
    }

    fn get_balance(&self, art: CID, owner_id: AccountId) -> U128 {
        let art_token = self.get_art(&art).unwrap();
        art_token.token.get_balance(owner_id)
    }
    fn get_allowance(&self, art: CID, owner_id: AccountId, escrow_account_id: AccountId) -> U128 {
        let art_token = self.get_art(&art).unwrap();
        art_token.token.get_allowance(owner_id, escrow_account_id)
    }
    fn get_decimals_per_token(&self) -> u32 {
        return NUMB_DECS;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    use super::*;

    fn alice() -> AccountId {
        "alice.near".to_string()
    }
    fn bob() -> AccountId {
        "bob.near".to_string()
    }
    fn carol() -> AccountId {
        "carol.near".to_string()
    }

    fn get_context(predecessor_account_id: AccountId) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 1_000_000_000_000_000_000_000_000_000u128,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }

    #[test]
    fn test_update_artist_profile() {
        let context = get_context(carol());
        testing_env!(context);
        let mut contract = FurBall::new();

        contract.update_profile("MY Profile CID".to_string());
        assert_eq!(contract.get_profile(carol()), "MY Profile CID");
    }

    #[test]
    fn test_initialize_2_new_tokens() {
        let context = get_context(carol());
        testing_env!(context);
        let mut contract = FurBall::new();

        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art.clone());

        let art2 = "QqPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();

        contract.create_token(art2.clone());
    }

    #[test]
    fn test_get_artist_designs() {
        let context = get_context(carol());
        testing_env!(context);

        let mut contract = FurBall::new();
        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art.clone());

        let art2 = "mPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art2.clone());

        let carol_des = contract.get_designs(carol());
        println!("{:?}", carol_des.to_vec());
        assert!(carol_des.contains(&art.clone()));
        assert!(carol_des.contains(&art2.clone()));
        // assert_eq!(carol_des.);
        assert_eq!(contract.get_designs(bob()).to_vec().len(), 0);
    }

    #[test]
    fn test_change_token_cost() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000u128;
        let mut contract = FurBall::new();
        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art.clone());

        let cost = contract.cost_per_token(art.clone());
        assert_eq!(cost, DEFAULT_COST_PER_TOKEN);

        contract.change_cost(art.clone(), 1_000_000);
        assert_eq!(contract.cost_per_token(art.clone()), 1_000_000);
    }

    #[test]
    fn test_transfer_token() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000u128;
        let mut contract = FurBall::new();
        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art.clone());
        assert_eq!(contract.get_balance(art.clone(), carol()).0, total_supply);
        contract.transfer(art.clone(), bob(), U128(1_000));
        assert_eq!(
            contract.get_balance(art.clone(), carol()).0,
            total_supply - 1_000
        );
        assert_eq!(contract.get_balance(art.clone(), bob()).0, 1_000);
    }

    #[test]
    fn test_put_on_sale_and_buy() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000u128;
        let carol_init_bal = env::account_balance();
        let mut contract = FurBall::new();
        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art.clone());
        contract.put_on_sale(art.clone(), U128(1_000_000));
        assert_eq!(
            contract
                .get_allowance(art.clone(), carol(), env::current_account_id())
                .0,
            1_000_000
        );
        assert_eq!(
            contract.get_amount_on_sale(art.clone(), carol()).0,
            1_000_000
        );
        contract.change_cost(art.clone(), 100);
        assert_eq!(contract.cost_per_token(art.clone()), 100);

        assert_eq!(
            contract.get_all_sellers(art.clone())[0],
            (carol(), 1_000_000u128)
        );

        assert_eq!(contract.get_balance(art.clone(), bob()).0, 0);

        context.attached_deposit = 1_000 * 100;
        context.predecessor_account_id = bob();
        testing_env!(context.clone());
        contract.buy(art.clone(), 1_000, carol());
        context.account_balance = env::account_balance();
        context.storage_usage = env::storage_usage();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(
            contract.get_balance(art.clone(), carol()).0,
            total_supply - 1_000u128
        );
        assert_eq!(contract.get_balance(art.clone(), bob()).0, 1_000u128);

        context.predecessor_account_id = carol();
        testing_env!(context);
        // TODO: cross env costs
        // assert_eq!(env::account_balance() - carol_init_bal, 1_000 * 100);
    }

    #[test]
    #[should_panic]
    fn test_buy_more_than_on_sale() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000u128;
        let mut contract = FurBall::new();
        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art.clone());
        contract.put_on_sale(art.clone(), U128(100));
        assert_eq!(contract.get_amount_on_sale(art.clone(), carol()).0, 100);
        contract.change_cost(art.clone(), 100);
        assert_eq!(contract.cost_per_token(art.clone()), 100);

        assert_eq!(contract.get_balance(art.clone(), bob()).0, 0);

        context.attached_deposit = 1_000 * 100;
        context.predecessor_account_id = bob();
        testing_env!(context.clone());
        contract.buy(art.clone(), 1_000, carol());
    }

    #[test]
    #[should_panic]
    fn test_change_tok_cost_unauth() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000u128;
        let mut contract = FurBall::new();
        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art.clone());

        let context_bob = get_context(bob());
        testing_env!(context_bob);

        contract.change_cost(art.clone(), 1_000_000);
    }

    // TODO: this is not working and I have no clue why
    // #[test]
    // #[should_panic]
    // fn test_initialize_new_furball_twice_fails() {
    //     let context = get_context(carol());
    //     testing_env!(context);
    //     let total_supply = 1_000_000_000u128;
    //     {
    //         let _contract = FurBall::new(bob(), total_supply.into());
    //     }
    //     FurBall::new(bob(), total_supply.into());
    // }

    #[test]
    #[should_panic]
    fn test_initialize_coin_same_art_fails() {
        let context = get_context(carol());
        testing_env!(context);
        let mut contract = FurBall::new();
        let art = "QmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        let art_clone = art.clone();
        contract.create_token(art);
        contract.create_token(art_clone);
    }

    #[test]
    #[should_panic]
    fn test_initialize_coin_cid_too_long() {
        let context = get_context(carol());
        testing_env!(context);
        let mut contract = FurBall::new();
        let art = "QQmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqvQmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqvQmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqvmPAwR5un1YPJEF6iB7KvErDmAhiXxwL5J5qjA3Z9ceKqv".to_string();
        contract.create_token(art);
    }
}
