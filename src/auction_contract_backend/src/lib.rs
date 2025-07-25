use ic_cdk::{
    caller,
    update,
    query,
    storage,
    trap, 
};
use candid::{self, CandidType, Deserialize, Principal};
use ic_cdk_macros::{pre_upgrade, post_upgrade};

use std::{collections::BTreeMap, sync::Mutex};


// represent an item listed for auction
#[derive(CandidType, Deserialize, Clone, Debug)] 
pub struct Item {
    id: u64,
    owner: Principal,
    name: String,
    description: String,
    current_highest_bid: u64,
    highest_bidder: Option<Principal>, 
    active: bool, 
    new_owner: Option<Principal>,
}

// Rep. a bid on an item
#[derive(CandidType, Deserialize, Clone, Debug)] 
pub struct Bid {
    bidder: Principal,
    amount: u64,
}

// main state of  canister
#[derive(CandidType, Deserialize)] 
struct CanisterState {
    items: BTreeMap<u64, Item>,
    item_bids: BTreeMap<u64, BTreeMap<Principal, Bid>>,
    next_item_id: u64,
}

// initialize the state as a thread-local static.
thread_local! {
    static STATE: Mutex<CanisterState> = Mutex::new(CanisterState {
        items: BTreeMap::new(),
        item_bids: BTreeMap::new(),
        next_item_id: 0,
    });
}


// Get current caller's principal
fn get_caller() -> Principal {
    caller()
}


#[pre_upgrade]
fn pre_upgrade() {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        storage::stable_save((&*state,))
            .expect("Failed to encode state for stable save");
    });
}

#[post_upgrade]
fn post_upgrade() {
    STATE.with(|state_mutex| {
        let mut state = state_mutex.lock().unwrap();
        match storage::stable_restore::<(CanisterState,)>() {
            Ok((restored_state,)) => {
                *state = restored_state;
            },
            Err(e) => {
                if format!("{}", e).contains("stable memory is empty") || format!("{}", e).contains("empty_stream") {
                    ic_cdk::println!("Stable memory empty or malformed, initializing new state.");
                    *state = CanisterState {
                        items: BTreeMap::new(),
                        item_bids: BTreeMap::new(),
                        next_item_id: 0,
                    };
                } else {
                    ic_cdk::trap(&format!("Failed to decode state from stable memory: {}", e));
                }
            }
        };
    });
}



// 1. List Items
#[update]
fn list_item(name: String, description: String) -> u64 {
    let caller = get_caller();
    STATE.with(|state_mutex| {
        let mut state = state_mutex.lock().unwrap();

        let item_id = state.next_item_id;
        state.next_item_id += 1;

        let new_item = Item {
            id: item_id,
            owner: caller,
            name,
            description,
            current_highest_bid: 0,
            highest_bidder: None,
            active: true,
            new_owner: None,
        };

        state.items.insert(item_id, new_item);
        state.item_bids.insert(item_id, BTreeMap::new()); 

        ic_cdk::println!("Item listed: {} by {}", item_id, caller);
        item_id
    })
}

// 2. Bid for an item
#[update]
fn bid_for_item(item_id: u64, amount: u64) -> Result<String, String> {
    let caller = get_caller();

    STATE.with(|state_mutex| {
        let mut state = state_mutex.lock().unwrap();

        let mut item = state.items.get(&item_id)
            .ok_or_else(|| "Item not found.".to_string())?
            .clone(); 

        if !item.active {
            return Err("Auction for this item is no longer active.".to_string());
        }
        if item.owner == caller {
            return Err("Cannot bid on your own item.".to_string());
        }
        if amount <= item.current_highest_bid {
            return Err(format!("Bid amount ({}) must be higher than the current highest bid ({}).", amount, item.current_highest_bid));
        }

        //  Modify the cloned 'item'.
        item.current_highest_bid = amount;
        item.highest_bidder = Some(caller);

        // Update the original item in the BTreeMap with the modified clone.
        state.items.insert(item_id, item); 

        let item_bids_map = state.item_bids.entry(item_id).or_insert_with(BTreeMap::new);

        let new_bid = Bid {
            bidder: caller,
            amount,
        };
        item_bids_map.insert(caller, new_bid); 

        ic_cdk::println!("Bid placed: {} for item {} by {}", amount, item_id, caller);
        Ok("Bid placed successfully.".to_string())
    })
}
// 3. Update the listing of an item
#[update]
fn update_listing(item_id: u64, new_name: Option<String>, new_description: Option<String>) -> Result<String, String> {
    let caller = get_caller();
    STATE.with(|state_mutex| {
        let mut state = state_mutex.lock().unwrap();

        let item = state.items.get_mut(&item_id)
            .ok_or_else(|| "Item not found.".to_string())?;

        // Security check, only owner updates
        if item.owner != caller {
            return Err("Only the owner can update this listing.".to_string());
        }
        if !item.active {
            return Err("Cannot update a listing that is no longer active.".to_string());
        }

        if let Some(name) = new_name {
            item.name = name;
        }
        if let Some(description) = new_description {
            item.description = description;
        }

        ic_cdk::println!("Listing updated for item: {} by {}", item_id, caller);
        Ok("Listing updated successfully.".to_string())
    })
}

// 4. Stop the listing of an item
#[update]
fn stop_listing(item_id: u64) -> Result<String, String> {
    let caller = get_caller();
    STATE.with(|state_mutex| {
        let mut state = state_mutex.lock().unwrap();

        let item = state.items.get_mut(&item_id)
            .ok_or_else(|| "Item not found.".to_string())?;

        //  Only the owner can stop
        if item.owner != caller {
            return Err("Only the owner can stop this listing.".to_string());
        }
        if !item.active {
            return Err("Listing is already stopped.".to_string());
        }

        item.active = false; 
        item.new_owner = item.highest_bidder; 

        ic_cdk::println!("Listing stopped for item: {} by {}", item_id, caller);
        Ok("Listing stopped successfully. Highest bidder is now the owner.".to_string())
    })
}


// Retrieve a specific item
#[query]
fn get_item(item_id: u64) -> Option<Item> {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        state.items.get(&item_id).cloned()
    })
}

// Retrieve a list of items
#[query]
fn list_all_items() -> Vec<Item> {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        state.items.values().cloned().collect()
    })
}

// Retrieve the length of items listed on the contract
#[query]
fn get_listed_items_count() -> u64 {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        state.items.len() as u64
    })
}

// Retrieve the item sold for the most 
#[query]
fn get_most_expensive_sold_item() -> Option<Item> {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        state.items.values()
            .filter(|item| !item.active && item.new_owner.is_some())
            .max_by_key(|item| item.current_highest_bid)
            .cloned()
    })
}

// Retrieve the item that has been bid on the most
#[query]
fn get_item_with_most_bids() -> Option<Item> {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        let mut item_with_most_bids: Option<Item> = None;
        let mut max_bids_count: u64 = 0;

        for item in state.items.values() {
            if let Some(bids) = state.item_bids.get(&item.id) {
                let current_bids_count = bids.len() as u64;
                if current_bids_count > max_bids_count {
                    max_bids_count = current_bids_count;
                    item_with_most_bids = Some(item.clone());
                }
            }
        }
        item_with_most_bids
    })
}

// Get all bids for specific item
#[query]
fn get_bids_for_item(item_id: u64) -> Vec<Bid> {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        state.item_bids.get(&item_id)
            .map(|bids_map| bids_map.values().cloned().collect())
            .unwrap_or_else(Vec::new)
    })
}

// get highest bid for specific item
#[query]
fn get_highest_bid_for_item(item_id: u64) -> Option<Bid> {
    STATE.with(|state_mutex| {
        let state = state_mutex.lock().unwrap();
        state.items.get(&item_id).and_then(|item| {
            item.highest_bidder.and_then(|bidder| {
                state.item_bids.get(&item.id).and_then(|bids_map| {
                    bids_map.get(&bidder).cloned()
                })
            })
        })
    })
}

// generate The candid interface
ic_cdk::export_candid!();