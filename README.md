# Payflow Forecast

A Rust + Leptos personal finance planner for a dedicated recurring-payment account.

The first version focuses on forecasting the account balance from recurring bills and paycheck transfers. It recommends a stable transfer amount for each paycheck so the account stays above a configurable buffer over a five-year projection.

## Current Features

- Paycheck transfer model for the 15th and 30th of each month
- Editable recurring bills with biweekly, twice-monthly, monthly, quarterly, and yearly frequencies
- Annual bill increase rules
- Five-year daily balance simulation
- Stable per-paycheck transfer optimization
- Paycheck pressure warning when the recommended transfer is too high for one paycheck
- Configurable safety margin and minimum cash buffer
- Dashboard chart with projected balance, inflows, outflows, and low-point marker
- Historical bill increase comparison from seeded sample history
- Browser `localStorage` persistence for settings and recurring bills
- YNAB personal access token import for the target recurring-payment account
- Imported transaction classification into recurring payments, transfers, and misc activity

The production default state is empty. Use the `Sample` button only for testing and demos.

## YNAB Import

Create a Personal Access Token in the YNAB web app under Account Settings > Developer Settings, then paste it into the YNAB section of the app's Settings view.

The importer:

- Uses `https://api.ynab.com/v1`
- Searches every available YNAB plan for the account named `Desjardins Conjoint (LaSalle)`
- Shows the account names it found if the configured name does not match
- Can load YNAB budgets and accounts into dropdowns so the selected IDs are used directly
- Updates the planner starting balance from that YNAB account balance
- Imports transactions from that account
- Classifies unmatched outflows as `Misc`

The token is stored only in this browser's `localStorage` for now. Use "Forget token" to remove it from the saved planner state.

## Run Locally

```bash
trunk serve --address 127.0.0.1 --port 8080
```

Then open:

```text
http://127.0.0.1:8080
```

## Deploy

Pushes to `main` deploy the static Trunk build to GitHub Pages:

```text
https://ykoehler.github.io/payflow-forecast/
```

## Verify

```bash
cargo fmt --check
cargo test
env -u NO_COLOR trunk build --color never --skip-version-check
node scripts/e2e-smoke.mjs
playwright test
```

The end-to-end smoke test uses the existing app at `E2E_BASE_URL` when provided. Otherwise,
it runs `trunk build`, serves the generated `dist` directory with a tiny local Node server,
and verifies the served HTML, stylesheet, generated JavaScript, and WASM bundle are all reachable.

The Playwright suite covers browser workflows and layout checks. Install the runner with:

```bash
npm install
playwright install chromium
```

## Next Phase

- Add import/export backups for locally persisted planner data
- Add YNAB plan/account selection instead of first-plan and account-name matching
- Match actual payments to recurring bill rules
- Detect changed payment amounts and update bill history
