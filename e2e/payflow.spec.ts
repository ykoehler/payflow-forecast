import { expect, test, type Page } from "@playwright/test";

const STORAGE_KEY = "payflow-forecast-state-v1";

type Frequency = "Biweekly" | "Semimonthly" | "Monthly" | "Quarterly" | "Yearly";
type TransactionClass = "Recurring" | "Paycheck" | "Transfer" | "Misc";

type PlannerState = {
  settings: {
    starting_balance: number;
    minimum_buffer: number;
    margin_percent: number;
    forecast_years: number;
    paycheck_amount: number;
  };
  bills: Bill[];
  paychecks: Bill[];
  ynab: {
    access_token: string;
    plan_id: string | null;
    account_id: string | null;
    account_name: string;
    available_plans: { id: string; name: string }[];
    available_accounts: { id: string; name: string }[];
    last_import_status: string;
    last_imported_at: string | null;
  };
  transactions: TrackedTransaction[];
  recurring_candidates: unknown[];
};

type Bill = {
  id: number;
  name: string;
  amount: number;
  due_day: number;
  frequency: Frequency;
  annual_increase: number;
  renewal_month: number;
  anchor_date: string | null;
  history: { year: number; amount: number }[];
};

type TrackedTransaction = {
  id: string;
  date: string;
  payee_name: string;
  category_name: string;
  memo: string;
  amount: number;
  classification: TransactionClass;
  matched_bill_id: number | null;
  manual_classification: TransactionClass | null;
};

function bill(overrides: Partial<Bill>): Bill {
  return {
    id: 1,
    name: "Internet",
    amount: 91,
    due_day: 18,
    frequency: "Monthly",
    annual_increase: 3,
    renewal_month: 7,
    anchor_date: "2026-05-18",
    history: [],
    ...overrides,
  };
}

function transaction(overrides: Partial<TrackedTransaction>): TrackedTransaction {
  return {
    id: "tx-1",
    date: "2026-05-02",
    payee_name: "Coffee Shop",
    category_name: "Misc",
    memo: "",
    amount: -8.25,
    classification: "Misc",
    matched_bill_id: null,
    manual_classification: null,
    ...overrides,
  };
}

function plannerState(overrides: Partial<PlannerState> = {}): PlannerState {
  return {
    settings: {
      starting_balance: 1072.52,
      minimum_buffer: 250,
      margin_percent: 8,
      forecast_years: 5,
      paycheck_amount: 3818,
    },
    bills: [bill({ id: 1 })],
    paychecks: [],
    ynab: {
      access_token: "",
      plan_id: null,
      account_id: null,
      account_name: "Desjardins Conjoint (LaSalle)",
      available_plans: [],
      available_accounts: [],
      last_import_status: "Not connected",
      last_imported_at: null,
    },
    transactions: [],
    recurring_candidates: [],
    ...overrides,
  };
}

async function seedPlannerState(page: Page, state: PlannerState) {
  await page.addInitScript(
    ([key, value]) => window.localStorage.setItem(key, JSON.stringify(value)),
    [STORAGE_KEY, state],
  );
}

async function openApp(page: Page) {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
}

async function mockYnabApi(page: Page) {
  const corsHeaders = {
    "access-control-allow-origin": "*",
    "access-control-allow-headers": "authorization,content-type",
    "content-type": "application/json",
  };

  await page.route("https://api.ynab.com/v1/**", async (route) => {
    const url = new URL(route.request().url());

    if (url.pathname === "/v1/plans") {
      await route.fulfill({
        headers: corsHeaders,
        json: { data: { plans: [{ id: "budget-1", name: "Family Budget" }] } },
      });
      return;
    }

    if (url.pathname === "/v1/plans/budget-1/accounts") {
      await route.fulfill({
        headers: corsHeaders,
        json: {
          data: {
            accounts: [
              {
                id: "acc-1",
                name: "Desjardins Conjoint (LaSalle)",
                balance: 1_072_520,
              },
              { id: "acc-2", name: "Everyday Chequing", balance: 3_000_000 },
            ],
          },
        },
      });
      return;
    }

    if (url.pathname === "/v1/plans/budget-1/accounts/acc-1/transactions") {
      await route.fulfill({
        headers: corsHeaders,
        json: {
          data: {
            transactions: [
              ...["2026-01-15", "2026-01-30", "2026-02-15", "2026-02-28", "2026-03-15", "2026-03-30"].map(
                (date) => ({
                  id: `payroll-${date}`,
                  date,
                  amount: 1_620_250,
                  payee_name: "Employer Payroll",
                  category_name: "Inflow: Ready to Assign",
                  memo: null,
                  deleted: false,
                }),
              ),
              ...["2026-01-03", "2026-01-17", "2026-01-31", "2026-02-14"].map((date) => ({
                id: `royal-bank-${date}`,
                date,
                amount: -1_200_000,
                payee_name: "Royal Bank",
                category_name: "Mortgage",
                memo: null,
                deleted: false,
              })),
              { id: "hydro-1", date: "2026-01-15", amount: -55_000, payee_name: "Hydro-Quebec", category_name: "Hydro-Quebec", memo: null, deleted: false },
              { id: "hydro-2", date: "2026-02-16", amount: -180_000, payee_name: "Hydro-Quebec", category_name: "Hydro-Quebec", memo: null, deleted: false },
              { id: "hydro-3", date: "2026-03-15", amount: -92_000, payee_name: "Hydro-Quebec", category_name: "Hydro-Quebec", memo: null, deleted: false },
              { id: "trip-1", date: "2026-03-20", amount: -40_000, payee_name: "Gaspesie Trip", category_name: "Gaspesie", memo: null, deleted: false },
            ],
          },
        },
      });
      return;
    }

    await route.fulfill({ status: 404, headers: corsHeaders, json: { error: { detail: "Not mocked" } } });
  });
}

function paginationTransactions(count: number): TrackedTransaction[] {
  return Array.from({ length: count }, (_, index) => {
    const day = String(1 + (index % 28)).padStart(2, "0");
    const month = String(1 + Math.floor(index / 28)).padStart(2, "0");
    return transaction({
      id: `page-${index}`,
      date: `2026-${month}-${day}`,
      payee_name: `Transaction ${String(index + 1).padStart(2, "0")}`,
      amount: -(index + 1),
      category_name: "Non-Recurring",
      manual_classification: "Misc",
    });
  });
}

async function expectNoDocumentHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
  }));
  expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.viewportWidth + 2);
}

async function expectScrollableWhenNeeded(locator: ReturnType<Page["locator"]>) {
  const metrics = await locator.evaluate((element) => {
    const initial = element.scrollLeft;
    element.scrollLeft = element.scrollWidth;
    const scrolled = element.scrollLeft;
    element.scrollLeft = initial;
    return {
      clientWidth: element.clientWidth,
      scrollWidth: element.scrollWidth,
      overflowX: getComputedStyle(element).overflowX,
      scrolled,
    };
  });

  if (metrics.scrollWidth > metrics.clientWidth + 2) {
    expect(["auto", "scroll"]).toContain(metrics.overflowX);
    expect(metrics.scrolled).toBeGreaterThan(0);
  }
}

async function expectNoButtonTextOverflow(page: Page) {
  const offenders = await page.locator("button:visible").evaluateAll((buttons) =>
    buttons
      .map((button) => {
        const rect = button.getBoundingClientRect();
        return {
          text: button.textContent?.trim() || button.getAttribute("aria-label") || "",
          clientWidth: button.clientWidth,
          scrollWidth: button.scrollWidth,
          width: rect.width,
        };
      })
      .filter((button) => button.width > 0 && button.scrollWidth > button.clientWidth + 2)
      .slice(0, 8),
  );

  expect(offenders).toEqual([]);
}

test("imports YNAB data and turns recurring activity into bills, paycheck transfers, and unassigned transactions", async ({ page }) => {
  await mockYnabApi(page);
  await openApp(page);

  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByLabel("Personal access token").fill("fake-token");
  await page.getByRole("button", { name: "Load budgets/accounts" }).click();

  await expect(page.getByText("Loaded 1 budgets and 2 accounts")).toBeVisible();
  await expect(page.getByLabel("Budget")).toHaveValue("budget-1");
  await expect(page.getByLabel("Account")).toHaveValue("acc-1");

  await page.getByRole("button", { name: "Import from YNAB" }).click();
  await expect(page.getByText("Imported 14 transactions from Desjardins Conjoint (LaSalle)")).toBeVisible();

  await page.getByRole("button", { name: "Bills" }).click();
  await expect(page.getByTestId("bills-table")).toContainText("Mortgage");
  await expect(page.getByTestId("bills-table")).toContainText("Hydro-Quebec");
  await expect(page.getByTestId("paycheck-transfers-table")).toContainText("Employer Payroll");
  await expect(page.getByTestId("paycheck-transfers-table")).toContainText("Twice monthly");

  await page.getByRole("button", { name: "Transactions" }).click();
  await expect(page.getByTestId("transaction-group-row").filter({ hasText: "Paycheck Transfers" })).toBeVisible();
  await expect(page.getByTestId("transaction-group-row").filter({ hasText: "Mortgage" })).toBeVisible();
  await expect(page.getByTestId("transaction-group-row").filter({ hasText: "Hydro-Quebec" })).toBeVisible();
  await expect(page.getByTestId("transaction-group-row").filter({ hasText: "Unassigned" })).toContainText("Pinned last");
  await expect(page.getByTestId("transaction-group-row").filter({ hasText: "Gaspesie" })).toHaveCount(0);
});

test("supports editing bills, expanding transaction details, assigning a transaction to a created bill, and adding rows", async ({ page }) => {
  await seedPlannerState(
    page,
    plannerState({
      bills: [bill({ id: 1, name: "Internet", amount: 91 })],
      transactions: [
        transaction({
          id: "coffee-1",
          date: "2026-05-02",
          payee_name: "Coffee Shop",
          category_name: "Misc",
          amount: -8.25,
        }),
      ],
    }),
  );
  await openApp(page);

  await page.getByRole("button", { name: "Bills" }).click();
  await page.getByTestId("bills-table").getByRole("button", { name: "$91.00" }).click();
  await page.locator("input.money-edit").fill("94.50");
  await page.locator("input.money-edit").blur();
  await expect(page.getByTestId("bills-table").getByRole("button", { name: "$94.50" })).toBeVisible();

  await page.getByRole("button", { name: "Show advanced bill fields" }).first().click();
  await expect(page.getByRole("button", { name: "Hide advanced bill fields" })).toBeVisible();
  await expect(page.getByTestId("bills-table")).toContainText("Annual increase");

  await page.getByRole("button", { name: "Transactions" }).click();
  await page.getByLabel("Bills and paycheck transfers").selectOption("__create_bill__");
  await expect(page.getByTestId("transaction-group-row").filter({ hasText: "Coffee Shop" })).toBeVisible();

  await page.getByRole("button", { name: "Collapse Coffee Shop" }).click();
  await expect(page.getByTestId("transaction-row")).toHaveCount(0);
  await page.getByRole("button", { name: "Expand Coffee Shop" }).click();
  await expect(page.getByTestId("transaction-row")).toHaveCount(1);

  await page.getByRole("button", { name: "Show transaction details" }).click();
  await expect(page.getByRole("button", { name: "Delete" })).toBeVisible();

  await page.getByRole("button", { name: "Add Transaction" }).click();
  await expect(page.getByTestId("transaction-row")).toHaveCount(2);
});

test("paginates ungrouped transactions and sorts by date from the table header", async ({ page }) => {
  await seedPlannerState(
    page,
    plannerState({
      bills: [],
      transactions: paginationTransactions(65),
    }),
  );
  await openApp(page);

  await page.getByRole("button", { name: "Transactions" }).click();
  await page.getByLabel("Group transactions").selectOption("none");

  await expect(page.getByText("1-50 of 65")).toBeVisible();
  await expect(page.getByTestId("transaction-row")).toHaveCount(50);
  await expect(page.getByTestId("transaction-row").first()).toContainText("2026-03-09");

  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByText("51-65 of 65")).toBeVisible();
  await expect(page.getByTestId("transaction-row")).toHaveCount(15);

  await page.getByRole("button", { name: "Previous" }).click();
  await page.getByRole("button", { name: "Date Desc" }).click();
  await expect(page.getByTestId("transaction-row").first()).toContainText("2026-01-01");
});

test("keeps transaction layout scrollable on small screens without page overflow or clipped controls", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await seedPlannerState(
    page,
    plannerState({
      bills: [
        bill({
          id: 1,
          name: "A very long bill name that still needs to remain usable",
          amount: 1234.56,
        }),
      ],
      transactions: paginationTransactions(8).map((item, index) => ({
        ...item,
        payee_name: `Long imported payee name ${index} with enough text to stress the ledger`,
        category_name: index % 2 === 0 ? "A very long bill name that still needs to remain usable" : "Misc",
        matched_bill_id: index % 2 === 0 ? 1 : null,
        classification: index % 2 === 0 ? "Recurring" : "Misc",
      })),
    }),
  );
  await openApp(page);

  await page.getByRole("button", { name: "Transactions" }).click();
  await page.getByLabel("Group transactions").selectOption("none");

  await expectNoDocumentHorizontalOverflow(page);
  await expectScrollableWhenNeeded(page.getByTestId("transactions-table"));
  await expectNoButtonTextOverflow(page);
});

test("renders chart lines, activity bars, low-point marker, and visual baseline", async ({ page }) => {
  await seedPlannerState(
    page,
    plannerState({
      settings: {
        starting_balance: 1072.52,
        minimum_buffer: 250,
        margin_percent: 8,
        forecast_years: 5,
        paycheck_amount: 3818,
      },
      bills: [
        bill({ id: 1, name: "Internet", amount: 94.5, due_day: 18, frequency: "Monthly" }),
        bill({ id: 2, name: "Mortgage", amount: 1200, due_day: 3, frequency: "Biweekly", anchor_date: "2026-05-03" }),
      ],
      transactions: [
        transaction({ id: "pay-1", date: "2026-04-15", payee_name: "Paycheck transfer", amount: 1620.25, classification: "Paycheck" }),
        transaction({ id: "internet-1", date: "2026-04-18", payee_name: "Internet", category_name: "Internet", amount: -94.5, classification: "Recurring", matched_bill_id: 1 }),
        transaction({ id: "mortgage-1", date: "2026-05-03", payee_name: "Royal Bank", category_name: "Mortgage", amount: -1200, classification: "Recurring", matched_bill_id: 2 }),
      ],
    }),
  );
  await openApp(page);

  const chart = page.getByTestId("balance-chart");
  await expect(chart).toBeVisible();

  const geometry = await chart.locator("svg").evaluate((svg) => {
    const paths = [...svg.querySelectorAll("path")].map((path) => ({
      stroke: path.getAttribute("stroke"),
      d: path.getAttribute("d") || "",
    }));
    const activityBars = [...svg.querySelectorAll("rect")].filter((rect) =>
      ["#3066be", "#c78022"].includes(rect.getAttribute("fill") || ""),
    );
    const marker = svg.querySelector(".low-point-marker circle[fill='#bd3d2a']");
    const cx = Number(marker?.getAttribute("cx"));
    const cy = Number(marker?.getAttribute("cy"));

    return {
      paths,
      activityBarCount: activityBars.length,
      marker: { cx, cy },
      viewBox: svg.getAttribute("viewBox"),
      text: svg.textContent || "",
    };
  });

  expect(geometry.viewBox).toBe("0 0 1120 420");
  expect(geometry.paths.some((path) => path.stroke === "#4f6f52" && path.d.startsWith("M "))).toBeTruthy();
  expect(geometry.paths.some((path) => path.stroke === "#087f7a" && path.d.startsWith("M "))).toBeTruthy();
  expect(geometry.activityBarCount).toBeGreaterThan(2);
  expect(geometry.marker.cx).toBeGreaterThanOrEqual(64);
  expect(geometry.marker.cx).toBeLessThanOrEqual(1096);
  expect(geometry.marker.cy).toBeGreaterThanOrEqual(34);
  expect(geometry.marker.cy).toBeLessThanOrEqual(364);
  expect(geometry.text).toContain("Past 12 months");
  expect(geometry.text).toContain("Forecast");

  await expect(chart).toHaveScreenshot("balance-chart.png");
});
