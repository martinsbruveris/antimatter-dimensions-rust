// "How to Play" (H2P) entries, copied from the original game
// (../antimatter-dimensions/src/core/secret-formula/h2p.js) and trimmed to
// the mechanics this reimplementation has. Keep this in sync with the
// original — see AGENTS.md on UI fidelity.
//
// The original builds each `info` string with formatting helpers
// (formatInt, formatX, formatPercents, …) that resolve at runtime against
// the player's notation/settings. Most bodies here are static HTML, so those
// helpers are resolved to the values the original renders with default
// settings (e.g. formatInt(10) -> "10", formatX(2) -> "×2",
// formatPercents(1) -> "100%"); these are all small numbers that read the
// same under every notation. The exceptions are the large numbers whose
// rendering *does* vary with notation — the Antimatter Dimensions base-price /
// cost-multiplier lists (from ad-core AD_BASE_COSTS / AD_COST_MULTIPLIERS) and
// the Infinity antimatter cap — which are formatted live via formatDecimal so
// they follow the player's chosen notation.
//
// Each entry has:
//   name  Display name, shown in the tab list and as the body title.
//   info  HTML body of the entry. Either a string, or a function of the
//         unlock `flags` (see below) returning a string, mirroring the
//         original's `info()` which interpolates live game state — used
//         where the body grows as you progress (e.g. Common Abbreviations).
//   isUnlocked  Function of the unlock `flags` returning whether the entry
//         is visible/searchable, mirroring the original's `isUnlocked()`.
//   tags  Keywords matched by the search bar (in addition to the name).
//   tab   "tabKey/subtabKey" (or "tabKey") this entry is the default for;
//         opening the modal selects the entry matching the current tab.
//
// `flags` is derived from the engine snapshot in H2PModal.vue:
//   { tickspeedUnlocked, sacrificeUnlocked, infinityUnlocked }
// matching the original's progress predicates (Tickspeed.isUnlocked,
// Sacrifice.isVisible, PlayerProgress.infinityUnlocked()). Only the gates
// for currently-implemented mechanics are wired; every other entry is
// always unlocked.
import { formatDecimal } from "../util/format.js";

// Raw { mantissa, exponent } pairs (value = m × 10^e), mirroring ad-core's
// AD_BASE_COSTS / AD_COST_MULTIPLIERS (crates/ad-core/src/data/constants.rs).
// Kept as literals because those Rust constants aren't exposed to JS;
// formatDecimal renders them under the player's current notation rather than
// hardcoding a single notation's text.
const AD_BASE_COSTS = [
  { m: 1, e: 1 }, { m: 1, e: 2 }, { m: 1, e: 4 }, { m: 1, e: 6 },
  { m: 1, e: 9 }, { m: 1, e: 13 }, { m: 1, e: 18 }, { m: 1, e: 24 },
];
const AD_COST_MULTIPLIERS = [
  { m: 1, e: 3 }, { m: 1, e: 4 }, { m: 1, e: 5 }, { m: 1, e: 6 },
  { m: 1, e: 8 }, { m: 1, e: 10 }, { m: 1, e: 12 }, { m: 1, e: 15 },
];

// Number.MAX_VALUE (2^1024 − 2^971 ≈ 1.797693e308), the largest finite f64 and
// the antimatter cap that forces the first Big Crunch. formatDecimal(_, 6)
// renders the 6-place mantissa the original shows.
const INFINITY_THRESHOLD = { m: 1.7976931348623157, e: 308 };

export const h2pTabs = [
  {
    name: "This Modal",
    info: `
Welcome to the How to Play!
<br>
<br>
This modal (pop-up window) contains in-depth explanations and additional details for everything you will encounter
as you progress through the game. As you unlock new features and mechanics, you will also gain access to additional
pages here. If you ever feel lost or confused about how something in the game works, you may find a helpful
explanation within the related entry in here.
<br>
<br>
For now, opening the How to Play will always start you on this page. After you get your first Dimension Boost,
opening this modal will instead place you on the How to Play entry most relevant to the game content on your currently
visible tab and subtab, if such an entry exists.
`,
    isUnlocked: () => true,
    tags: ["h2p", "how", "to", "play", "modal"],
    tab: "",
  },
  {
    name: "Common Abbreviations",
    info: (f) => `
Many resources within the game may appear in an abbreviated format as text in order to save space. This How to
Play entry will update itself with additional entries for new resources as you encounter them for the first time.
<br>
- <b>AM</b>: Antimatter<br>
- <b>AD</b>: Antimatter Dimension<br>
- <b>AG</b>: Antimatter Galaxy<br>
${f.infinityUnlocked ? "- <b>IP</b>: Infinity Point<br>" : ""}
`,
    isUnlocked: () => true,
    tags: ["abbreviation", "shorten", "am", "ad", "ag", "ip"],
    tab: "",
  },
  {
    name: "Antimatter Dimensions",
    info: () => `
Antimatter is a resource that is used throughout the entire game for purchasing various things as you progress. You
start with 10 antimatter when you first open the game, and you can
spend it to buy the 1st Antimatter Dimension to start the game.
<br>
<br>
Antimatter Dimensions are your production units in game. The 1st Antimatter Dimension produces your antimatter.
Each consecutive Antimatter Dimension produces the previous one, allowing you to have steady growth.
There are eight Antimatter Dimensions total.
<br>
<br>
<b>Dimension Multiplier:</b> Beside the Dimension there is a multiplier (example: 1st Dimension ×1.0).
The base production of each Dimension is multiplied by this number.
This multiplier increases by ×2 for every 10 of that Dimension purchased.
Each time this occurs, the price of the dimension will increase.
<br>
<br>
<b>Accumulated Dimension Quantity:</b> The next column is your current amount of that Dimension you own.
This is a combination of how many you have purchased with antimatter,
as well as produced from the higher Dimension.
<br>
<br>
<b>Purchased Dimensions Quantity:</b> Next to each accumulated quantity of owned Dimensions,
the amount of that Dimension purchased toward the next multiplier upgrade is displayed in brackets.
For example if you have (4) next to your accumulated dimension quantity,
you will need 6 more of that dimension for the next multiplier increase.
<br>
<br>
<b>Dimension Growth Percent:</b> This number represents the amount of growth that each
Dimension experiences per second. 100% means the dimension is doubling each second.
This allows you to judge overall growth.
<br>
<br>
<b>Cost &amp; until 10:</b>
You can buy a single quantity of each Dimension with antimatter when the cost button is highlighted.
Alternatively, if the Until 10 button is highlighted,
you can buy whatever quantity gets you to that Dimension's next Dimension multiplier.
<br>
<br>
<b>Max all:</b> Max all will buy until 10 of the 1st Antimatter Dimension until it cannot anymore,
then second, and so on until the 8th Antimatter Dimension, and then buy max Tickspeed Upgrades.
<br>
<br>
<b>Dimension base prices:</b> ${AD_BASE_COSTS.map((n) => formatDecimal(n)).join(", ")}
<br>
<b>Base per 10 bought dimension price increases:</b> ${AD_COST_MULTIPLIERS.map((n) => formatDecimal(n)).join(", ")}
<br>
<br>
<b>Hotkeys: 1, 2, 3, 4, 5, 6, 7, 8</b> for buy until 10 Xth Dimension
(you can also hold down Shift while buying Dimensions, which will only buy
1 instead of 10), <b>M</b> for Max all
`,
    isUnlocked: () => true,
    tags: ["dims", "normal", "antimatter", "ad"],
    tab: "dimensions/antimatter",
  },
  {
    name: "Tickspeed",
    info: `
Production in the game happens on each "tick", which initially occurs once per second. By buying Tickspeed Upgrades,
you can make your Antimatter Dimensions produce faster, as if multiple ticks occur in each second.
<br>
<br>
<b>Tickspeed:</b> This states how many game ticks are occurring every second. Fractional ticks are accounted for,
boosting production as if part of a game tick has passed. Note that the actual tickspeed time is simulated and the
game always runs calculations at the update rate you've chosen in the Options tab.
<br>
<br>
<b>Cost:</b> The cost of antimatter for multiplying ticks/sec by the displayed multiplier.
(without any Galaxies, this is ×1.125 per purchase)
<br>
<br>
<b>Buy Max:</b> This will buy the maximum amount of Tickspeed Upgrades available
with your current amount of antimatter.
<br>
<br>
<b>Hotkeys: T</b> will purchase as many Tickspeed Upgrades as possible, or <b>Shift+T</b> to buy a single upgrade.
<b>M</b> for Max all.
`,
    isUnlocked: (f) => f.tickspeedUnlocked,
    tags: ["dimension", "earlygame", "time"],
    tab: "dimensions/antimatter",
  },
  {
    name: "Dimension Boosts",
    info: `
<b>Dimension Boost:</b> This resets your antimatter and all of your Antimatter Dimensions, but unlocks another
Antimatter Dimension for you to purchase and boosts your Dimension multipliers.
The 1st Dimension Boost requires 20 4th Dimensions, the 2nd requires 20 5th Dimensions, etc.
After unlocking all 8 Dimensions,
every additional boost will cost 15 more 8th Dimensions than the previous Boost and will no longer
unlock a Dimension, but will continue to increase your Dimension multipliers.
<br>
<br>
You gain a ×2 multiplier to the 1st Dimension for every Dimension Boost you have. Each higher
Dimension will have the multiplier applied one less time as the previous, down to a minimum of 0.
For example, with 3 Boosts, the 1st Dimension will gain ×8, the 2nd Dimension ×4,
the 3rd Dimension ×2, and all other Dimensions are unaffected.
<br>
<br>
<b>Hotkey: D</b> will try to purchase a Dimension Boost.
`,
    isUnlocked: () => true,
    tags: ["dimboost", "reset", "earlygame"],
    tab: "dimensions/antimatter",
  },
  {
    name: "Antimatter Galaxies",
    info: `
Purchasing an Antimatter Galaxy will reset your game back to the point where only 4 Dimensions are
available, but will increase the effect of your Tickspeed Upgrades by +0.02 for your first two
Galaxies. As you get more Galaxies, the multiplier will continue becoming stronger and stronger.
<br>
<br>
Though it will have very little impact for the first few Tickspeed purchases,
the increase is multiplicative and will not take long to be visible.
<br>
<br>
Your first Antimatter Galaxy requires 80 Eighth Dimensions, and each additional Galaxy will cost
another 60 more.
<br>
<b>Distant Galaxy scaling:</b> Above 100 Antimatter Galaxies the cost increase between Galaxies will
increase by 2 per Galaxy, making the next Galaxy cost 62 more, then 64 more,
etc.
<br>
<b>Remote Galaxy scaling:</b> Above 800 Antimatter Galaxies, the <i>total</i> cost
increases by another 0.2% per Galaxy, on top of Distant scaling.
<br>
<br>
<b>Hotkey: G</b> will try to purchase an Antimatter Galaxy.
`,
    isUnlocked: () => true,
    tags: ["8th", "reset", "galaxy", "earlygame"],
    tab: "dimensions/antimatter",
  },
  {
    name: "Dimensional Sacrifice",
    info: `
<b>You unlock Dimensional Sacrifice after your fifth Dimension Boost.</b>
<br>
<br>
Sacrificing will immediately reset the owned quantity of all non-Eighth Dimensions to zero, without reducing the
multiplier or the current cost. In return, it will multiply the Eighth Dimension Multiplier by the shown value.
It will take time to get back to the production you previously had, but you will end up with a net increase.
<br>
<br>
The Dimensional Sacrifice multiplier scales with the number of 1st Dimensions you had at the time of sacrifice,
and the scaling can be improved by completing certain Achievements and challenges. The multiplier is kept between
sacrifices, meaning that sacrificing once at ×10 and then once at ×4 will be the same as
×8 then ×5; in both cases you will end up with a total sacrifice multiplier of ×40.
<br>
<br>
<b>Hotkey: S</b> will try to sacrifice.
`,
    isUnlocked: (f) => f.sacrificeUnlocked,
    tags: ["8th", "reset", "earlygame", "gods"],
    tab: "dimensions/antimatter",
  },
  {
    name: "Achievements",
    info: `
Each Achievement has requirements to unlock. Once unlocked, some Achievements give a reward.
Requirements and rewards vary in difficulty and benefit significantly.
<br>
<br>
In addition to any specific rewards for individual Achievements, you will receive a ×1.03 multiplier
to all Antimatter Dimensions. Each fully completed row also gives another ×1.25. The total multiplier
effect from all Achievements together is shown above all the Achievement images.
<br>
<br>
Secret Achievements offer no gameplay benefits or advantages and are simply there for fun. Hovering over a Secret
Achievement will give a hint on how to attain them.
`,
    isUnlocked: () => true,
    tags: ["earlygame", "awards"],
    tab: "achievements",
  },
  {
    name: "Infinity",
    info: () => `
Once you have too much antimatter for the world to handle (2<sup>1024</sup>
or about ${formatDecimal(INFINITY_THRESHOLD, 6)},
sometimes called "Infinity"), you will be forced to do a "Big Crunch". This will reset your antimatter, Antimatter
Dimensions, Dimension Boosts, and your Antimatter Galaxies. Doing a Big Crunch is also sometimes referred to as
"Infinitying".
<br>
<br>
You will eventually be able to pass ${formatDecimal(INFINITY_THRESHOLD, 6)}, but until then any larger numbers will
display as Infinity.
<br>
<br>
Each Infinity completed will give an Infinity Point, which can be spent on upgrades in the new Infinity tab.
You must purchase these upgrades from top to bottom. You will also gain one "Infinity", which is effectively
the number of times you have crunched.
<br>
<br>
<b>Hotkey: C</b> will try to perform a Big Crunch.
`,
    isUnlocked: (f) => f.infinityUnlocked,
    tags: ["crunch", "big", "upgrades", "ip", "reset", "prestige", "earlygame"],
    tab: "infinity/upgrades",
  },
  {
    name: "Autobuyers",
    info: `
Autobuyers allow you to automatically purchase dimensions, upgrades, or prestiges. All autobuyer
controls are located under the "Autobuyers" subtab of the "Automation" tab, including any additional autobuyers
unlocked later in the game.
<br>
<br>
Antimatter Dimension Autobuyers and the Tickspeed Upgrade Autobuyer can be unlocked based on your total antimatter,
but most other autobuyers require upgrades to be purchased or challenges to be beaten.
<br>
<br>
Most Autobuyers have similar attributes:
<br>
<br>
<b>Autobuyer Interval:</b> The cooldown period before the autobuyer attempts to make another purchase.
Antimatter Dimension Autobuyers and the Tickspeed Upgrade Autobuyer require their respective challenges to be beaten
before their interval can be upgraded.
<br>
<br>
<b>Antimatter Dimension Autobuyer Bulk Buy:</b> Once the interval of an autobuyer reaches its minimum
(at 100 ms), all future upgrades will double the maximum amount the autobuyer can purchase per tick.
This can be disabled.
<br>
<br>
<b>Antimatter Dimension Autobuyer Buy Quantity:</b> Autobuyers for Dimensions can be set to buy a single Dimension,
or until 10. Bulk buy is disabled when the autobuyer is set to singles.
<br>
<br>
<b>Tickspeed Autobuyer Buy Quantity:</b> The tickspeed autobuyer can be set to buy a single upgrade per activation
or to buy the max possible once the Tickspeed Challenge (C9) has been beaten.
<br>
<br>
<b>Pause/Resume Autobuyers:</b> This button will pause or resume autobuyers which are turned on.
It does not change individual autobuyer settings. Think of it like a master switch.
<br>
<br>
<b>Enable/Disable All Autobuyers:</b> This button will turn all of your autobuyers on or off individually.
<br>
<br>
<b>Hotkey: A</b> (for pausing/resuming autobuyers).
Additionally, holding <b>Alt</b> when pressing a hotkey associated with an upgrade, dimension, or prestige will
toggle the associated autobuyer.
`,
    isUnlocked: () => true,
    tags: ["infinity", "automation", "challenges", "rewards", "interval", "earlygame"],
    tab: "automation/autobuyers",
  },
];
