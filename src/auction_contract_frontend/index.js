// Import the agent and the backend canister's actor
import { auction_contract_backend } from "../../declarations/auction_contract_backend";

// --- DOM Elements ---
const itemNameInput = document.getElementById("itemName");
const itemDescriptionInput = document.getElementById("itemDescription");
const listItemBtn = document.getElementById("listItemBtn");
const listItemStatus = document.getElementById("listItemStatus");

const refreshItemsBtn = document.getElementById("refreshItemsBtn");
const itemsContainer = document.getElementById("itemsContainer");

const bidItemIdInput = document.getElementById("bidItemId");
const bidAmountInput = document.getElementById("bidAmount");
const placeBidBtn = document.getElementById("placeBidBtn");
const bidStatus = document.getElementById("bidStatus");

// --- Event Listeners ---
listItemBtn.addEventListener("click", listItem);
refreshItemsBtn.addEventListener("click", refreshItems);
placeBidBtn.addEventListener("click", placeBid);

// --- Functions ---

/**
 * Lists a new item on the auction.
 */
async function listItem() {
    const name = itemNameInput.value.trim();
    const description = itemDescriptionInput.value.trim();

    if (!name || !description) {
        listItemStatus.textContent = "Please fill in both item name and description.";
        listItemStatus.style.color = "red";
        return;
    }

    listItemBtn.disabled = true;
    listItemStatus.textContent = "Listing item...";
    listItemStatus.style.color = "black";

    try {
        const itemId = await auction_contract_backend.list_item(name, description);
        listItemStatus.textContent = `Item listed successfully! Item ID: ${itemId}`;
        listItemStatus.style.color = "green";
        itemNameInput.value = "";
        itemDescriptionInput.value = "";
        refreshItems(); // Refresh the list after adding a new item
    } catch (error) {
        console.error("Error listing item:", error);
        listItemStatus.textContent = `Error listing item: ${error.message || error}`;
        listItemStatus.style.color = "red";
    } finally {
        listItemBtn.disabled = false;
    }
}

/**
 * Refreshes the list of all items from the canister.
 */
async function refreshItems() {
    itemsContainer.innerHTML = "<p>Loading items...</p>";
    refreshItemsBtn.disabled = true;

    try {
        const items = await auction_contract_backend.list_all_items();
        itemsContainer.innerHTML = ""; // Clear previous items

        if (items.length === 0) {
            itemsContainer.innerHTML = "<p>No items listed yet.</p>";
            return;
        }

        items.forEach(item => {
            const itemCard = document.createElement("div");
            itemCard.classList.add("item-card");
            if (!item.active) {
                itemCard.classList.add("inactive");
            }

            itemCard.innerHTML = `
                <h3>${item.name} (ID: ${item.id})</h3>
                <p><strong>Description:</strong> ${item.description}</p>
                <p><strong>Owner:</strong> <span class="owner-id">${item.owner.toText()}</span></p>
                <p><strong>Current Highest Bid:</strong> ${item.current_highest_bid} ICP</p>
                <p><strong>Highest Bidder:</strong> ${item.highest_bidder.length > 0 ? `<span class="bidder-id">${item.highest_bidder[0].toText()}</span>` : "N/A"}</p>
                <p><strong>Status:</strong> ${item.active ? "Active" : "Ended"}</p>
                ${!item.active && item.new_owner.length > 0 ? `<p><strong>New Owner:</strong> <span class="owner-id">${item.new_owner[0].toText()}</span></p>` : ""}
            `;
            itemsContainer.appendChild(itemCard);
        });

    } catch (error) {
        console.error("Error fetching items:", error);
        itemsContainer.innerHTML = `<p style="color:red;">Error loading items: ${error.message || error}</p>`;
    } finally {
        refreshItemsBtn.disabled = false;
    }
}

/**
 * Places a bid on a specified item.
 */
async function placeBid() {
    const itemId = parseInt(bidItemIdInput.value);
    const amount = parseInt(bidAmountInput.value);

    if (isNaN(itemId) || isNaN(amount) || amount <= 0) {
        bidStatus.textContent = "Please enter valid item ID and bid amount (greater than 0).";
        bidStatus.style.color = "red";
        return;
    }

    placeBidBtn.disabled = true;
    bidStatus.textContent = "Placing bid...";
    bidStatus.style.color = "black";

    try {
        // The DFX generated JS declarations return a Result type.
        // { 'Ok': "..." } or { 'Err': "..." }
        const result = await auction_contract_backend.bid_for_item(itemId, amount);

        if (result.Ok) {
            bidStatus.textContent = `Bid placed successfully: ${result.Ok}`;
            bidStatus.style.color = "green";
            bidItemIdInput.value = "";
            bidAmountInput.value = "";
            refreshItems(); // Refresh items to show new bid
        } else if (result.Err) {
            bidStatus.textContent = `Error placing bid: ${result.Err}`;
            bidStatus.style.color = "red";
        } else {
            bidStatus.textContent = `Unexpected result: ${JSON.stringify(result)}`;
            bidStatus.style.color = "orange";
        }
    } catch (error) {
        console.error("Error during bid:", error);
        bidStatus.textContent = `An unexpected error occurred: ${error.message || error}`;
        bidStatus.style.color = "red";
    } finally {
        placeBidBtn.disabled = false;
    }
}


// Initial load
document.addEventListener("DOMContentLoaded", refreshItems);