// Tauri IPC invoke helper
const { invoke } = window.__TAURI__.core;

const DIM_NAMES = [
    "1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th"
];

let lastTimestamp = performance.now();

// --- DOM references ---
const antimatterEl = document.getElementById("antimatter");
const perSecEl = document.getElementById("antimatter-per-sec");
const tickspeedMultInfo = document.getElementById("tickspeed-mult-info");
const tickspeedTotalInfo = document.getElementById("tickspeed-total-info");
const buy10MultInfo = document.getElementById("buy10-mult-info");
const btnSacrifice = document.getElementById("btn-sacrifice");
const btnTickspeed = document.getElementById("btn-tickspeed");
const btnDimBoost = document.getElementById("btn-dim-boost");
const dimBoostInfo = document.getElementById("dim-boost-info");
const btnGalaxy = document.getElementById("btn-galaxy");
const galaxyInfo = document.getElementById("galaxy-info");
const dimensionsContainer = document.getElementById("dimensions");

// --- Build initial dimension rows ---
function buildDimensionRows() {
    let html = "";
    for (let i = 0; i < 8; i++) {
        html += `
        <div class="dim-row" id="dim-row-${i}">
            <div class="dim-name" id="dim-name-${i}"></div>
            <div class="dim-amount" id="dim-amount-${i}"></div>
            <div class="dim-rate" id="dim-rate-${i}"></div>
            <div class="dim-buttons">
                <button class="btn-dim" id="btn-dim-buy-${i}" onclick="buyDim(${i})">
                    Cost: 0
                </button>
                <button class="btn-dim" id="btn-dim-buy10-${i}" onclick="buyUntil10(${i})">
                    Until 10: 0
                </button>
            </div>
        </div>`;
    }
    dimensionsContainer.innerHTML = html;
}

buildDimensionRows();

// --- Render the game state ---
function render(state) {
    // Header
    antimatterEl.textContent = state.antimatter;
    perSecEl.textContent =
        `You are getting ${state.antimatter_per_sec} antimatter per second.`;

    const effectPerUpgrade = (1.0 / state.tickspeed_purchase_multiplier).toFixed(3);
    tickspeedMultInfo.textContent =
        `ADs produce ${effectPerUpgrade}x faster per Tickspeed upgrade`;
    tickspeedTotalInfo.textContent =
        `Total Tickspeed: ${state.tickspeed_effect} / sec`;

    // Sacrifice & Max All info
    let infoText = "Buy 10 Dimension purchase multiplier: 2.00x";
    if (state.sacrifice_unlocked) {
        infoText += ` | Dimensional Sacrifice multiplier: ×${state.sacrifice_multiplier}`;
    }
    buy10MultInfo.textContent = infoText;

    // Sacrifice button
    if (state.sacrifice_unlocked) {
        btnSacrifice.style.display = "";
        if (state.can_sacrifice) {
            btnSacrifice.disabled = false;
            btnSacrifice.textContent =
                `Dimensional Sacrifice (×${state.sacrifice_multiplier_if_sacrificed})`;
        } else {
            btnSacrifice.disabled = true;
            btnSacrifice.textContent =
                "Dimensional Sacrifice Disabled (no dimensions)";
        }
    } else {
        btnSacrifice.style.display = "none";
    }

    // Tickspeed
    btnTickspeed.textContent = `Tickspeed Cost: ${state.tickspeed_cost}`;
    btnTickspeed.disabled = !state.can_buy_tickspeed;

    // Dimensions
    for (let i = 0; i < 8; i++) {
        const row = document.getElementById(`dim-row-${i}`);
        const dim = state.dimensions[i];
        const unlocked = i < state.unlocked_dimensions;

        if (!unlocked) {
            row.classList.add("locked");
            document.getElementById(`dim-name-${i}`).textContent =
                `${DIM_NAMES[i]} Antimatter Dimension`;
            document.getElementById(`dim-amount-${i}`).textContent = "";
            document.getElementById(`dim-rate-${i}`).textContent = "";
            document.getElementById(`btn-dim-buy-${i}`).disabled = true;
            document.getElementById(`btn-dim-buy-${i}`).textContent = "Locked";
            document.getElementById(`btn-dim-buy10-${i}`).disabled = true;
            document.getElementById(`btn-dim-buy10-${i}`).textContent = "Locked";
            continue;
        }

        row.classList.remove("locked");
        document.getElementById(`dim-name-${i}`).textContent =
            `${DIM_NAMES[i]} Antimatter Dimension  ×${dim.multiplier}`;
        document.getElementById(`dim-amount-${i}`).textContent =
            `${dim.amount} (${dim.bought_mod_10})`;

        if (i < 7 && dim.rate_percent > 0.01) {
            document.getElementById(`dim-rate-${i}`).textContent =
                `+${dim.rate_percent.toFixed(2)}%/s`;
        } else {
            document.getElementById(`dim-rate-${i}`).textContent = "";
        }

        const btnBuy = document.getElementById(`btn-dim-buy-${i}`);
        btnBuy.textContent = `Cost: ${dim.cost}`;
        btnBuy.disabled = !dim.can_buy;

        const btnBuy10 = document.getElementById(`btn-dim-buy10-${i}`);
        btnBuy10.textContent = `Until 10: ${dim.cost_until_10}`;
        btnBuy10.disabled = !dim.can_buy_10;
    }

    // Prestige - Dim Boost
    dimBoostInfo.textContent =
        `Dimension Boost (${state.dim_boosts}): requires ${state.dim_boost_req_amount} ` +
        `${DIM_NAMES[state.dim_boost_req_tier]} Antimatter Dimensions`;
    btnDimBoost.disabled = !state.can_dim_boost;
    if (state.dim_boosts < 4) {
        btnDimBoost.textContent =
            `Reset to unlock ${DIM_NAMES[4 + state.dim_boosts]} Antimatter Dimension`;
    } else {
        btnDimBoost.textContent = "Reset your Dimensions to boost";
    }

    // Prestige - Galaxy
    galaxyInfo.textContent =
        `Antimatter Galaxies (${state.galaxies}): requires ` +
        `${state.galaxy_requirement} 8th Antimatter Dimensions`;
    btnGalaxy.disabled = !state.can_buy_galaxy;
}

// --- Game loop ---
async function gameLoop() {
    const now = performance.now();
    const dt = now - lastTimestamp;
    lastTimestamp = now;

    try {
        const state = await invoke("tick_and_get_state", { dtMs: dt });
        render(state);
    } catch (e) {
        console.error("tick error:", e);
    }

    requestAnimationFrame(gameLoop);
}

// --- Action handlers ---
async function buyDim(tier) {
    await invoke("buy_dimension", { tier });
}

async function buyUntil10(tier) {
    await invoke("buy_until_10", { tier });
}

async function buyTickspeed() {
    await invoke("buy_tickspeed");
}

async function buyMaxTickspeed() {
    await invoke("buy_max_tickspeed");
}

async function buyDimBoost() {
    await invoke("buy_dim_boost");
}

async function buyGalaxy() {
    await invoke("buy_galaxy");
}

async function doSacrifice() {
    await invoke("sacrifice");
}

async function maxAll() {
    await invoke("max_all");
}

// Wire up sacrifice button
btnSacrifice.addEventListener("click", doSacrifice);

// Keyboard shortcut: M for max all
document.addEventListener("keydown", (e) => {
    if (e.key === "m" || e.key === "M") {
        maxAll();
    }
});

// Start the game loop
requestAnimationFrame(gameLoop);
