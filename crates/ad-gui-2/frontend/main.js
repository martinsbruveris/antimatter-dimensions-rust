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
const multiplierText = document.getElementById("multiplier-text");
const btnSacrifice = document.getElementById("btn-sacrifice");
const btnTickspeed = document.getElementById("btn-tickspeed");
const btnDimBoost = document.getElementById("btn-dim-boost");
const dimBoostInfo = document.getElementById("dim-boost-info");
const dimBoostReq = document.getElementById("dim-boost-req");
const btnGalaxy = document.getElementById("btn-galaxy");
const galaxyInfo = document.getElementById("galaxy-info");
const galaxyReq = document.getElementById("galaxy-req");
const dimensionsContainer = document.getElementById("dimensions");

// --- Build dimension rows using the original game's 7-column grid layout ---
function buildDimensionRows() {
    let html = "";
    for (let i = 0; i < 8; i++) {
        html += `
        <div class="c-dimension-row l-dimension-single-row c-antimatter-dim-row" id="dim-row-${i}">
            <div class="l-dimension-text-container">
                <div class="l-dim-row-text-box">
                    <span class="c-dim-row__name" id="dim-name-${i}">
                        ${DIM_NAMES[i]} Antimatter Dimension
                    </span>
                    <span class="c-dim-row__multiplier" id="dim-mult-${i}"></span>
                </div>
                <div class="l-dim-row-text-box">
                    <span class="c-dim-row__amount" id="dim-amount-${i}"></span>
                    <span class="c-dim-row__rate" id="dim-rate-${i}"></span>
                </div>
            </div>
            <div class="l-dim-row-multi-button-container">
                <button class="o-primary-btn o-primary-btn--new" id="btn-dim-${i}" onclick="buyDim(${i})">
                    <div class="button-content" id="btn-dim-content-${i}">
                        <div id="btn-dim-prefix-${i}">Buy 1</div>
                        <div id="btn-dim-cost-${i}">Cost: 0</div>
                    </div>
                    <div class="fill" id="btn-dim-fill-${i}">
                        <div class="fill-purchased" id="btn-dim-fill-purchased-${i}" style="width: 0%"></div>
                        <div class="fill-possible" id="btn-dim-fill-possible-${i}" style="width: 0%"></div>
                    </div>
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
        `${effectPerUpgrade}x faster / upgrade.`;
    tickspeedTotalInfo.textContent =
        `Tickspeed: ${state.tickspeed_effect} / sec`;

    // Multiplier text
    let mText = `Buy 10 Dimension purchase multiplier: ×2.00`;
    if (state.sacrifice_unlocked) {
        mText += ` | Dimensional Sacrifice multiplier: ×${state.sacrifice_multiplier}`;
    }
    multiplierText.textContent = mText;

    // Sacrifice button
    if (state.sacrifice_unlocked) {
        btnSacrifice.style.display = "";
        if (state.can_sacrifice) {
            btnSacrifice.classList.remove("o-primary-btn--disabled");
            btnSacrifice.textContent =
                `Dimensional Sacrifice (×${state.sacrifice_multiplier_if_sacrificed})`;
        } else {
            btnSacrifice.classList.add("o-primary-btn--disabled");
            btnSacrifice.textContent =
                "Dimensional Sacrifice Disabled (no dimensions)";
        }
    } else {
        btnSacrifice.style.display = "none";
    }

    // Tickspeed
    btnTickspeed.textContent = `Tickspeed Cost: ${state.tickspeed_cost}`;
    if (state.can_buy_tickspeed) {
        btnTickspeed.classList.remove("o-primary-btn--disabled");
    } else {
        btnTickspeed.classList.add("o-primary-btn--disabled");
    }

    // Dimensions
    for (let i = 0; i < 8; i++) {
        const row = document.getElementById(`dim-row-${i}`);
        const dim = state.dimensions[i];
        const unlocked = i < state.unlocked_dimensions;

        if (!unlocked) {
            row.classList.add("c-dim-row--not-reached");
            document.getElementById(`dim-mult-${i}`).textContent = "";
            document.getElementById(`dim-amount-${i}`).textContent = "";
            document.getElementById(`dim-rate-${i}`).textContent = "";
            const btn = document.getElementById(`btn-dim-${i}`);
            btn.classList.add("o-primary-btn--disabled");
            document.getElementById(`btn-dim-prefix-${i}`).textContent = "Locked";
            document.getElementById(`btn-dim-cost-${i}`).textContent = "";
            document.getElementById(`btn-dim-fill-purchased-${i}`).style.width = "0%";
            document.getElementById(`btn-dim-fill-possible-${i}`).style.width = "0%";
            continue;
        }

        row.classList.remove("c-dim-row--not-reached");
        document.getElementById(`dim-mult-${i}`).textContent = `×${dim.multiplier}`;
        document.getElementById(`dim-amount-${i}`).textContent =
            `${dim.amount} (${dim.bought_mod_10})`;

        if (i < 7 && dim.rate_percent > 0.01) {
            document.getElementById(`dim-rate-${i}`).textContent =
                `(+${dim.rate_percent.toFixed(2)}%/s)`;
        } else {
            document.getElementById(`dim-rate-${i}`).textContent = "";
        }

        const btn = document.getElementById(`btn-dim-${i}`);
        const howMany = dim.can_buy_10 ? (10 - dim.bought_mod_10) : (dim.can_buy ? 1 : 0);

        if (dim.can_buy) {
            btn.classList.remove("o-primary-btn--disabled");
        } else {
            btn.classList.add("o-primary-btn--disabled");
        }

        document.getElementById(`btn-dim-prefix-${i}`).textContent =
            `Buy ${howMany}`;
        document.getElementById(`btn-dim-cost-${i}`).textContent =
            `Cost: ${dim.cost_until_10} AM`;

        // Fill bars: purchased shows bought_mod_10, possible shows how many can buy
        document.getElementById(`btn-dim-fill-purchased-${i}`).style.width =
            `${dim.bought_mod_10 * 10}%`;
        document.getElementById(`btn-dim-fill-possible-${i}`).style.width =
            `${howMany * 10}%`;
    }

    // Prestige - Dim Boost
    dimBoostInfo.textContent = `Dimension Boost (${state.dim_boosts})`;
    dimBoostReq.textContent =
        `Requires: ${state.dim_boost_req_amount} ${DIM_NAMES[state.dim_boost_req_tier]} Antimatter D`;
    if (state.can_dim_boost) {
        btnDimBoost.classList.remove("o-primary-btn--disabled");
    } else {
        btnDimBoost.classList.add("o-primary-btn--disabled");
    }
    if (state.dim_boosts < 4) {
        btnDimBoost.textContent =
            `Reset to unlock ${DIM_NAMES[4 + state.dim_boosts]} Antimatter Dimension`;
    } else {
        btnDimBoost.textContent = "Reset your Dimensions to boost";
    }

    // Prestige - Galaxy
    galaxyInfo.textContent = `Antimatter Galaxies (${state.galaxies})`;
    galaxyReq.textContent =
        `Requires: ${state.galaxy_requirement} 8th Antimatter D`;
    if (state.can_buy_galaxy) {
        btnGalaxy.classList.remove("o-primary-btn--disabled");
    } else {
        btnGalaxy.classList.add("o-primary-btn--disabled");
    }
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
