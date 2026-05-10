use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde::Deserialize;
#[cfg(test)]
use serde::Deserialize;

use crate::forecast::{
    days_in_month, historical_increase_summary, month_label, optimize_transfer, required_floor,
    simulate, Date, EventType, Forecast,
};
use crate::models::{
    Bill, Frequency, PlannerState, RecurringCadence, RecurringCandidate, TrackedTransaction,
    TransactionClass, YnabChoice, DEFAULT_CATEGORY_NAME,
};

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "payflow-forecast-state-v1";
#[cfg(target_arch = "wasm32")]
const LANGUAGE_COOKIE_NAME: &str = "payflow-language";

const NON_RECURRING_CATEGORY: &str = "Non-Recurring";
const UNCATEGORIZED_CATEGORY: &str = "Uncategorized";
const BILL_SELECT_UNASSIGNED: &str = "__unassigned__";
const BILL_SELECT_NON_RECURRING: &str = "__non_recurring__";
const BILL_SELECT_CREATE: &str = "__create_bill__";
const PAYCHECK_SELECT_PREFIX: &str = "paycheck:";

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Language {
    English,
    French,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LanguagePreference {
    Browser,
    English,
    French,
}

impl LanguagePreference {
    fn language(self) -> Language {
        match self {
            LanguagePreference::Browser => detect_browser_language(),
            LanguagePreference::English => Language::English,
            LanguagePreference::French => Language::French,
        }
    }

    fn code(self) -> &'static str {
        match self {
            LanguagePreference::Browser => "browser",
            LanguagePreference::English => "en",
            LanguagePreference::French => "fr",
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let state = RwSignal::new(load_planner_state().unwrap_or_default());
    let language_preference = RwSignal::new(load_language_preference());
    let language = RwSignal::new(language_preference.get_untracked().language());
    provide_context(language_preference);
    provide_context(language);
    let start = Date::today();
    let active_view = RwSignal::new(ViewName::Dashboard);
    let is_importing = RwSignal::new(false);
    let sidebar_collapsed = RwSignal::new(false);
    let show_tutorial = RwSignal::new(!state.get().onboarding.introduction_done);
    let tutorial_step = RwSignal::new(TutorialStep::Welcome);

    Effect::new(move |_| {
        persist_planner_state(&state.get());
    });

    Effect::new(move |_| {
        let preference = language_preference.get();
        language.set(preference.language());
        persist_language_preference(preference);
    });

    Effect::new(move |_| {
        if show_tutorial.get() {
            if let Some(view) = tutorial_step.get().view() {
                active_view.set(view);
            }
        }
    });

    let transfer = Memo::new(move |_| optimize_transfer(&state.get(), start));
    let forecast = Memo::new(move |_| simulate(&state.get(), start));

    view! {
        <main class=move || {
            if sidebar_collapsed.get() {
                "app-shell sidebar-collapsed"
            } else {
                "app-shell"
            }
        }>
            <aside class="sidebar">
                <div class="brand">
                    <span class="brand-mark">"PF"</span>
                    <div class="brand-copy">
                        <h1>"Payflow Forecast"</h1>
                        <p>{move || t("Recurring account planner")}</p>
                    </div>
                </div>

                <nav class="nav-tabs" aria-label=move || t("Views")>
                    <TabButton view=ViewName::Dashboard active_view />
                    <TabButton view=ViewName::Bills active_view />
                    <TabButton view=ViewName::Transactions active_view />
                    <TabButton view=ViewName::Trends active_view />
                    <TabButton view=ViewName::Settings active_view />
                </nav>

                <InstallAppEntry />

                <button
                    class="sidebar-toggle"
                    type="button"
                    aria-label=move || {
                        if sidebar_collapsed.get() {
                            t("Expand sidebar")
                        } else {
                            t("Collapse sidebar")
                        }
                    }
                    title=move || {
                        if sidebar_collapsed.get() {
                            t("Expand sidebar")
                        } else {
                            t("Collapse sidebar")
                        }
                    }
                    aria-expanded=move || (!sidebar_collapsed.get()).to_string()
                    on:click=move |_| sidebar_collapsed.update(|collapsed| *collapsed = !*collapsed)
                >
                    <span aria-hidden="true">{move || if sidebar_collapsed.get() { ">" } else { "<" }}</span>
                    <span class="sidebar-toggle-label">
                        {move || if sidebar_collapsed.get() { t("Expand") } else { t("Collapse") }}
                    </span>
                </button>
            </aside>

            <section class="content">
                <header class="topbar">
                    <div>
                        <h2>{move || active_view.get().label(current_language())}</h2>
                    </div>
                </header>

                <InstallAppBanner />

                {move || match active_view.get() {
                    ViewName::Dashboard => view! { <Dashboard state forecast transfer /> }.into_any(),
                    ViewName::Bills => view! { <BillsView state /> }.into_any(),
                    ViewName::Transactions => view! { <TransactionsView state /> }.into_any(),
                    ViewName::Trends => view! { <TrendsView state /> }.into_any(),
                    ViewName::Settings => view! { <SettingsView state is_importing show_tutorial tutorial_step /> }.into_any(),
                }}
            </section>
            <TutorialOverlay state show_tutorial tutorial_step />
        </main>
    }
}

#[component]
fn InstallAppEntry() -> impl IntoView {
    view! {
        <div class="install-entry">
            <button
                class="install-button"
                type="button"
                data-install-app
                aria-label=move || t("Install app")
                title=move || t("Install app")
            >
                <span class="install-icon" aria-hidden="true">"+"</span>
                <span class="install-copy">
                    <strong>{move || t("Install app")}</strong>
                    <small>{move || t("Add this site to your home screen.")}</small>
                </span>
            </button>
            <section class="install-help" data-install-help hidden>
                <div class="install-help-card" role="dialog" aria-modal="true" aria-labelledby="install-help-title">
                    <div class="install-help-heading">
                        <h3 id="install-help-title">{move || t("Add to Home Screen")}</h3>
                        <button
                            class="icon-button install-help-close"
                            type="button"
                            data-install-help-close
                            aria-label=move || t("Close")
                            title=move || t("Close")
                        >
                            "x"
                        </button>
                    </div>
                    <p>{move || t("Open Payflow Forecast from your device home screen like a regular app.")}</p>
                    <ol>
                        <li>{move || t("iPhone or iPad: open Safari, tap Share, then Add to Home Screen.")}</li>
                        <li>{move || t("Android: open the browser menu, then tap Install app or Add to Home screen.")}</li>
                    </ol>
                </div>
            </section>
        </div>
    }
}

#[component]
fn InstallAppBanner() -> impl IntoView {
    view! {
        <section class="mobile-install-card">
            <div>
                <strong>{move || t("Use Payflow like an app")}</strong>
                <p>{move || t("Add it to your home screen for a full-screen shortcut on this device.")}</p>
            </div>
            <button class="primary-button" type="button" data-install-app>
                {move || t("Add to Home Screen")}
            </button>
        </section>
    }
}

#[component]
fn TabButton(view: ViewName, active_view: RwSignal<ViewName>) -> impl IntoView {
    view! {
        <button
            class=move || {
                if active_view.get() == view {
                    "tab-button active"
                } else {
                    "tab-button"
                }
            }
            type="button"
            aria-label=move || view.label(current_language())
            title=move || view.label(current_language())
            on:click=move |_| active_view.set(view)
        >
            <span class="tab-icon" aria-hidden="true">{view.icon()}</span>
            <span class="tab-label tab-label-full">{move || view.label(current_language())}</span>
            <span class="tab-label tab-label-short">{move || view.short_label(current_language())}</span>
        </button>
    }
}

#[component]
fn TutorialOverlay(
    state: RwSignal<PlannerState>,
    show_tutorial: RwSignal<bool>,
    tutorial_step: RwSignal<TutorialStep>,
) -> impl IntoView {
    let skip = move |_| {
        complete_tutorial(state);
        show_tutorial.set(false);
    };

    let next = move |_| {
        let step = tutorial_step.get();
        if step.is_last() {
            complete_tutorial(state);
            show_tutorial.set(false);
        } else {
            tutorial_step.set(step.next());
        }
    };

    view! {
        {move || {
            if show_tutorial.get() {
                let step = tutorial_step.get();
                view! {
                    <section class="tutorial-backdrop" role="dialog" aria-modal="true" aria-labelledby="tutorial-title">
                        <article class="tutorial-card">
                            <div class="tutorial-progress">
                                <span>{move || format!("{} {} {} {}", t("Step"), step.index() + 1, t("of"), TutorialStep::COUNT)}</span>
                                <button class="text-button" type="button" on:click=skip>{move || t("Skip introduction")}</button>
                            </div>
                            <h3 id="tutorial-title">{move || t(step.title())}</h3>
                            <p>{move || t(step.body())}</p>
                            <div class="tutorial-callout">
                                <strong>{move || t(step.callout_title())}</strong>
                                <span>{move || t(step.callout_body())}</span>
                            </div>
                            <div class="tutorial-actions">
                                <button
                                    class="secondary-button"
                                    type="button"
                                    disabled=move || tutorial_step.get() == TutorialStep::Welcome
                                    on:click=move |_| tutorial_step.update(|step| *step = step.previous())
                                >
                                    {move || t("Back")}
                                </button>
                                <button class="primary-button" type="button" on:click=next>
                                    {move || if tutorial_step.get().is_last() { t("Finish") } else { t("Next") }}
                                </button>
                            </div>
                        </article>
                    </section>
                }.into_any()
            } else {
                view! {}.into_any()
            }
        }}
    }
}

#[component]
fn Dashboard(
    state: RwSignal<PlannerState>,
    forecast: Memo<Forecast>,
    transfer: Memo<f64>,
) -> impl IntoView {
    view! {
        <section class="view active">
            <Show
                when=move || !state.get().paychecks.is_empty()
                fallback=move || view! { <DashboardPaycheckSetup state /> }
            >
                <div class="metrics-grid">
                    <Metric
                        label="Recommended transfer"
                        value=move || recommended_transfer_value(&forecast.get(), &state.get(), transfer.get())
                        note=move || recommended_transfer_note(&forecast.get(), &state.get())
                    />
                    <Metric
                        label="Lowest projected balance"
                        value=move || money(forecast.get().low_point.balance)
                        note=move || forecast.get().low_point.date.label()
                    />
                    <Metric
                        label="Paycheck pressure"
                        value=move || paycheck_pressure_value(&state.get(), transfer.get())
                        note=move || {
                            paycheck_pressure_note(&state.get(), transfer.get())
                        }
                    />
                </div>

                <section class="chart-section">
                    <div class="section-heading">
                        <div>
                            <h3>{move || t("Balance and activity")}</h3>
                            <p>{move || t("Adaptive transfers keep the account near the required reserve instead of accumulating excess cash.")}</p>
                        </div>
                        <div class="legend">
                            <span><i class="actual-key"></i>{move || t("Actual balance")}</span>
                            <span><i class="line-key"></i>{move || t("Projected balance")}</span>
                            <span><i class="inflow-key"></i>{move || t("Inflow")}</span>
                            <span><i class="outflow-key"></i>{move || t("Outflow")}</span>
                        </div>
                    </div>
                    {move || {
                        let snapshot = state.get();
                        chart_svg(
                            &forecast.get(),
                            required_floor(&snapshot.settings, &snapshot.bills),
                            snapshot.settings.starting_balance,
                            &snapshot.transactions,
                        )
                    }}
                </section>

                <section class="split-layout">
                    <article class="table-panel full-span">
                        <div class="section-heading"><h3>{move || t("Upcoming payments")}</h3></div>
                        <div class="compact-list">
                            {move || forecast.get().events
                                .into_iter()
                                .filter(|event| event.event_type == EventType::Payment)
                                .take(8)
                                .map(|event| view! {
                                    <div class="list-row">
                                        <div class="row-top"><span>{event.name}</span><span class="negative">{money(event.amount.abs())}</span></div>
                                        <div class="row-sub">{format!("{} {}: {}", event.date.label(), t("after payment"), money(event.balance))}</div>
                                    </div>
                                })
                                .collect_view()}
                        </div>
                    </article>
                </section>
            </Show>
        </section>
    }
}

#[component]
fn DashboardPaycheckSetup(state: RwSignal<PlannerState>) -> impl IntoView {
    view! {
        <section class="setup-panel">
            <div>
                <h3>{move || t("Add a paycheck transfer to forecast")}</h3>
                <p>{move || t("Payflow needs at least one paycheck transfer before it can estimate funding dates, recommended transfers, and projected balances.")}</p>
            </div>
            <button class="primary-button" type="button" on:click=move |_| add_paycheck(state)>
                {move || t("Add Paycheck")}
            </button>
        </section>
    }
}

#[component]
fn Metric(
    label: &'static str,
    value: impl Fn() -> String + Copy + Send + 'static,
    note: impl Fn() -> String + Copy + Send + 'static,
) -> impl IntoView {
    view! {
        <article class="metric-card">
            <span>{move || t(label)}</span>
            <strong>{value}</strong>
            <small>{note}</small>
        </article>
    }
}

#[component]
fn BillsView(state: RwSignal<PlannerState>) -> impl IntoView {
    view! {
        <section class="view active">
            <div class="section-heading">
                <div>
                    <h3>{move || t("Recurring payments")}</h3>
                    <p>{move || t("Click any value to edit it. Expand a row for schedule and increase details.")}</p>
                </div>
                <button class="primary-button" type="button" on:click=move |_| add_bill(state)>{move || t("Add Bill")}</button>
            </div>
            <div class="table-wrap bill-table-wrap" data-testid="bills-table">
                <table class="bill-table">
                    <thead>
                        <tr>
                            <th class="bill-expand-head"></th>
                            <th>{move || t("Name")}</th>
                            <th>{move || t("Amount")}</th>
                            <th>{move || t("Next due date")}</th>
                            <th>{move || t("Frequency")}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each={move || state.get().bills.into_iter().map(|bill| bill.id).collect::<Vec<_>>()}
                            key=|id| *id
                            children={move |id| view! { <BillRow state=state id=id /> }}
                        />
                    </tbody>
                </table>
            </div>
            <div class="section-heading subsection-heading">
                <div>
                    <h3>{move || t("Paycheck Transfers")}</h3>
                    <p>{move || t("Add or review recurring incoming transfers for this account.")}</p>
                </div>
                <button class="primary-button" type="button" on:click=move |_| add_paycheck(state)>{move || t("Add Paycheck")}</button>
            </div>
            <div class="table-wrap bill-table-wrap" data-testid="paycheck-transfers-table">
                <table class="bill-table">
                    <thead>
                        <tr>
                            <th class="bill-expand-head"></th>
                            <th>{move || t("Name")}</th>
                            <th>{move || t("Amount")}</th>
                            <th>{move || t("Next date")}</th>
                            <th>{move || t("Frequency")}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <Show
                            when=move || !state.get().paychecks.is_empty()
                            fallback=move || view! {
                                <tr class="empty-table-row">
                                    <td colspan="5">{move || t("No paycheck transfers yet.")}</td>
                                </tr>
                            }
                        >
                            <For
                                each={move || state.get().paychecks.into_iter().map(|paycheck| paycheck.id).collect::<Vec<_>>()}
                                key=|id| *id
                                children={move |id| view! { <PaycheckRow state=state id=id /> }}
                            />
                        </Show>
                    </tbody>
                </table>
            </div>
        </section>
    }
}

#[component]
fn BillRow(state: RwSignal<PlannerState>, id: u32) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let bill = move || state.get().bills.into_iter().find(|bill| bill.id == id);
    let name = move || bill().map(|bill| bill.name).unwrap_or_default();
    let amount = move || bill().map(|bill| bill.amount).unwrap_or_default();
    let frequency = move || {
        bill()
            .map(|bill| bill.frequency)
            .unwrap_or(Frequency::Monthly)
    };
    let annual_increase = move || bill().map(|bill| bill.annual_increase).unwrap_or_default();
    let renewal_month = move || bill().map(|bill| bill.renewal_month).unwrap_or(1);
    let next_due = move || {
        bill()
            .map(|bill| next_bill_due_date(&bill, Date::today()))
            .unwrap_or_else(Date::today)
    };

    view! {
        <tr class=move || if expanded.get() { "bill-row is-expanded" } else { "bill-row" }>
            <td class="bill-expand-cell">
                <button
                    class="bill-expand-button"
                    type="button"
                    aria-label=move || if expanded.get() { t("Hide advanced bill fields") } else { t("Show advanced bill fields") }
                    aria-expanded=move || expanded.get().to_string()
                    on:click=move |_| expanded.update(|value| *value = !*value)
                >
                </button>
            </td>
            <td class="bill-name-cell">
                <InlineTextCell
                    value=name
                    on_input=move |value| update_bill(state, id, |bill| bill.name = value)
                />
            </td>
            <td class="bill-money-cell">
                <InlineMoneyCell
                    value=amount
                    on_input=move |value| update_bill(state, id, |bill| bill.amount = value)
                />
            </td>
            <td>
                <InlineDateCell
                    value=next_due
                    on_input=move |value| update_bill_next_due_date(state, id, value)
                />
            </td>
            <td>
                <InlineFrequencyCell
                    value=frequency
                    on_input=move |frequency| update_bill_frequency(state, id, frequency)
                />
            </td>
        </tr>
        <tr class=move || if expanded.get() { "bill-advanced-row" } else { "bill-advanced-row is-hidden" }>
            <td></td>
            <td colspan="4">
                <div class="bill-advanced-fields">
                    <label>
                        <span>{move || t("Annual increase")}</span>
                        <InlineNumberField
                            value=annual_increase
                            on_input=move |value| update_bill(state, id, |bill| bill.annual_increase = value.max(0.0))
                        />
                    </label>
                    <label>
                        <span>{move || t("Schedule / increase month")}</span>
                        <select
                            class="compact-select"
                            prop:value=move || renewal_month().to_string()
                            on:change=move |event| update_bill(state, id, |bill| {
                                bill.renewal_month = event_target_value(&event)
                                    .parse::<u32>()
                                    .unwrap_or(1)
                                    .clamp(1, 12);
                            })
                        >
                            {(1..=12).map(|month| view! {
                                <option value=month.to_string()>{month_label(month)}</option>
                            }).collect_view()}
                        </select>
                    </label>
                    <p>
                        {move || t("For yearly and quarterly bills, this month anchors the payment schedule. For monthly bills, it is when yearly increases begin in the projection.")}
                    </p>
                    <div class="bill-danger-zone">
                        <button class="small-button danger" type="button" on:click=move |_| {
                            state.update(|state| state.bills.retain(|bill| bill.id != id));
                        }>{move || t("Delete")}</button>
                    </div>
                </div>
            </td>
        </tr>
    }
}

#[component]
fn PaycheckRow(state: RwSignal<PlannerState>, id: u32) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let paycheck = move || {
        state
            .get()
            .paychecks
            .into_iter()
            .find(|paycheck| paycheck.id == id)
    };
    let name = move || paycheck().map(|paycheck| paycheck.name).unwrap_or_default();
    let amount = move || {
        paycheck()
            .map(|paycheck| paycheck.amount)
            .unwrap_or_default()
    };
    let frequency = move || {
        paycheck()
            .map(|paycheck| paycheck.frequency)
            .unwrap_or(Frequency::Semimonthly)
    };
    let annual_increase = move || {
        paycheck()
            .map(|paycheck| paycheck.annual_increase)
            .unwrap_or_default()
    };
    let renewal_month = move || {
        paycheck()
            .map(|paycheck| paycheck.renewal_month)
            .unwrap_or(1)
    };
    let next_due = move || {
        paycheck()
            .map(|paycheck| next_bill_due_date(&paycheck, Date::today()))
            .unwrap_or_else(Date::today)
    };

    view! {
        <tr class=move || if expanded.get() { "bill-row paycheck-row is-expanded" } else { "bill-row paycheck-row" }>
            <td class="bill-expand-cell">
                <button
                    class="bill-expand-button"
                    type="button"
                    aria-label=move || if expanded.get() { t("Hide paycheck transfer details") } else { t("Show paycheck transfer details") }
                    aria-expanded=move || expanded.get().to_string()
                    on:click=move |_| expanded.update(|value| *value = !*value)
                >
                </button>
            </td>
            <td class="bill-name-cell">
                <InlineTextCell
                    value=name
                    on_input=move |value| update_paycheck(state, id, |paycheck| paycheck.name = value)
                />
            </td>
            <td class="bill-money-cell">
                <InlineMoneyCell
                    value=amount
                    on_input=move |value| update_paycheck(state, id, |paycheck| paycheck.amount = value)
                />
            </td>
            <td>
                <InlineDateCell
                    value=next_due
                    on_input=move |value| update_paycheck_next_date(state, id, value)
                />
            </td>
            <td>
                <InlineFrequencyCell
                    value=frequency
                    on_input=move |frequency| update_paycheck_frequency(state, id, frequency)
                />
            </td>
        </tr>
        <tr class=move || if expanded.get() { "bill-advanced-row" } else { "bill-advanced-row is-hidden" }>
            <td></td>
            <td colspan="4">
                <div class="bill-advanced-fields">
                    <label>
                        <span>{move || t("Expected increase")}</span>
                        <InlineNumberField
                            value=annual_increase
                            on_input=move |value| update_paycheck(state, id, |paycheck| paycheck.annual_increase = value.max(0.0))
                        />
                    </label>
                    <label>
                        <span>{move || t("Schedule / increase month")}</span>
                        <select
                            class="compact-select"
                            prop:value=move || renewal_month().to_string()
                            on:change=move |event| update_paycheck(state, id, |paycheck| {
                                paycheck.renewal_month = event_target_value(&event)
                                    .parse::<u32>()
                                    .unwrap_or(1)
                                    .clamp(1, 12);
                            })
                        >
                            {(1..=12).map(|month| view! {
                                <option value=month.to_string()>{month_label(month)}</option>
                            }).collect_view()}
                        </select>
                    </label>
                    <p>
                        {move || t("Twice monthly paycheck transfers are scheduled on the 15th and 30th.")}
                    </p>
                    <div class="bill-danger-zone">
                        <button class="small-button danger" type="button" on:click=move |_| {
                            state.update(|state| state.paychecks.retain(|paycheck| paycheck.id != id));
                        }>{move || t("Delete")}</button>
                    </div>
                </div>
            </td>
        </tr>
    }
}

#[component]
fn InlineTextCell(
    value: impl Fn() -> String + Copy + Send + 'static,
    on_input: impl Fn(String) + Copy + Send + 'static,
) -> impl IntoView {
    let editing = RwSignal::new(false);

    view! {
        {move || if editing.get() {
            view! {
                <input
                    class="inline-edit"
                    autofocus=true
                    prop:value=value
                    on:input=move |event| on_input(event_target_value(&event))
                    on:blur=move |_| editing.set(false)
                />
            }.into_any()
        } else {
            view! {
                <button class="inline-display" type="button" on:click=move |_| editing.set(true)>
                    {move || {
                        let value = value();
                        if value.trim().is_empty() { t("Click to edit").to_string() } else { value }
                    }}
                </button>
            }.into_any()
        }}
    }
}

#[component]
fn InlineMoneyCell(
    value: impl Fn() -> f64 + Copy + Send + 'static,
    on_input: impl Fn(f64) + Copy + Send + 'static,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let raw_value = RwSignal::new(money_input(value()));
    let commit = move || {
        if let Some(value) = parse_money(&raw_value.get_untracked()) {
            on_input(value);
        }
        editing.set(false);
    };

    view! {
        {move || if editing.get() {
            view! {
                <input
                    class="inline-edit money-edit"
                    autofocus=true
                    inputmode="decimal"
                    prop:value=move || raw_value.get()
                    on:input=move |event| raw_value.set(event_target_value(&event))
                    on:keydown=move |event| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            commit();
                        } else if event.key() == "Escape" {
                            event.prevent_default();
                            editing.set(false);
                        }
                    }
                    on:blur=move |_| commit()
                />
            }.into_any()
        } else {
            view! {
                <button class="inline-display money-display" type="button" on:click=move |_| {
                    raw_value.set(money_input(value()));
                    editing.set(true);
                }>
                    {move || money(value())}
                </button>
            }.into_any()
        }}
    }
}

#[component]
fn InlineDateCell(
    value: impl Fn() -> Date + Copy + Send + 'static,
    on_input: impl Fn(String) + Copy + Send + 'static,
) -> impl IntoView {
    let editing = RwSignal::new(false);

    view! {
        {move || if editing.get() {
            view! {
                <input
                    class="inline-edit date-edit"
                    type="date"
                    autofocus=true
                    prop:value=move || date_input_value(value())
                    on:change=move |event| on_input(event_target_value(&event))
                    on:blur=move |_| editing.set(false)
                />
            }.into_any()
        } else {
            view! {
                <button class="inline-display date-display" type="button" on:click=move |_| editing.set(true)>
                    {move || relative_date_label(value(), Date::today())}
                </button>
            }.into_any()
        }}
    }
}

#[component]
fn InlineIsoDateCell(
    value: impl Fn() -> String + Copy + Send + 'static,
    on_input: impl Fn(String) + Copy + Send + 'static,
) -> impl IntoView {
    let editing = RwSignal::new(false);

    view! {
        {move || if editing.get() {
            view! {
                <input
                    class="inline-edit date-edit"
                    type="date"
                    autofocus=true
                    prop:value=value
                    on:change=move |event| on_input(event_target_value(&event))
                    on:blur=move |_| editing.set(false)
                />
            }.into_any()
        } else {
            view! {
                <button class="inline-display date-display" type="button" on:click=move |_| editing.set(true)>
                    {move || value()}
                </button>
            }.into_any()
        }}
    }
}

#[component]
fn InlineSignedMoneyCell(
    value: impl Fn() -> f64 + Copy + Send + 'static,
    on_input: impl Fn(f64) + Copy + Send + 'static,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let raw_value = RwSignal::new(money_input(value()));
    let commit = move || {
        if let Some(value) = parse_money(&raw_value.get_untracked()) {
            on_input(value);
        }
        editing.set(false);
    };

    view! {
        {move || if editing.get() {
            view! {
                <input
                    class="inline-edit money-edit"
                    autofocus=true
                    inputmode="decimal"
                    prop:value=move || raw_value.get()
                    on:input=move |event| raw_value.set(event_target_value(&event))
                    on:keydown=move |event| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            commit();
                        } else if event.key() == "Escape" {
                            event.prevent_default();
                            editing.set(false);
                        }
                    }
                    on:blur=move |_| commit()
                />
            }.into_any()
        } else {
            view! {
                <button
                    class=move || {
                        if value() < 0.0 {
                            "inline-display money-display negative"
                        } else {
                            "inline-display money-display positive"
                        }
                    }
                    type="button"
                    on:click=move |_| {
                        raw_value.set(money_input(value()));
                        editing.set(true);
                    }
                >
                    {move || signed_money(value())}
                </button>
            }.into_any()
        }}
    }
}

#[component]
fn InlineFrequencyCell(
    value: impl Fn() -> Frequency + Copy + Send + 'static,
    on_input: impl Fn(Frequency) + Copy + Send + 'static,
) -> impl IntoView {
    let editing = RwSignal::new(false);

    view! {
        {move || if editing.get() {
            view! {
                <select
                    class="inline-edit compact-select"
                    autofocus=true
                    prop:value=move || frequency_value(value())
                    on:change=move |event| on_input(parse_frequency(&event_target_value(&event)))
                    on:blur=move |_| editing.set(false)
                >
                    {Frequency::ALL.into_iter().map(|frequency| view! {
                        <option value=frequency_value(frequency)>{frequency_label(frequency)}</option>
                    }).collect_view()}
                </select>
            }.into_any()
        } else {
            view! {
                <button class="inline-display" type="button" on:click=move |_| editing.set(true)>
                    {move || frequency_label(value())}
                </button>
            }.into_any()
        }}
    }
}

#[component]
fn InlineNumberField(
    value: impl Fn() -> f64 + Copy + Send + 'static,
    on_input: impl Fn(f64) + Copy + Send + 'static,
) -> impl IntoView {
    view! {
        <input
            class="compact-number"
            type="number"
            step="0.01"
            prop:value=value
            on:input=move |event| on_input(event_target_value(&event).parse::<f64>().unwrap_or(0.0))
        />
    }
}

fn date_input_value(date: Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

fn relative_date_label(date: Date, today: Date) -> String {
    let delta = date_days(date) - date_days(today);

    match delta {
        0 => t("Today").to_string(),
        1 => t("Tomorrow").to_string(),
        2..=6 => match current_language() {
            Language::French => format!("Dans {delta} jours"),
            Language::English => format!("In {delta} days"),
        },
        7..=13 => t("Next week").to_string(),
        _ => date.label(),
    }
}

fn next_bill_due_date(bill: &Bill, mut date: Date) -> Date {
    for _ in 0..=370 {
        if bill_occurs_for_ui(bill, date) {
            return date;
        }
        date = date.next_day();
    }

    date
}

fn bill_occurs_for_ui(bill: &Bill, date: Date) -> bool {
    if bill.frequency == Frequency::Biweekly {
        return biweekly_bill_occurs_for_ui(bill, date);
    }

    if bill.frequency == Frequency::Semimonthly {
        return semimonthly_bill_occurs_for_ui(date);
    }

    if date.day != bill.due_day.min(days_in_month(date.year, date.month)) {
        return false;
    }

    match bill.frequency {
        Frequency::Biweekly => false,
        Frequency::Semimonthly => false,
        Frequency::Monthly => true,
        Frequency::Quarterly => date.month % 3 == bill.renewal_month % 3,
        Frequency::Yearly => date.month == bill.renewal_month,
    }
}

fn semimonthly_bill_occurs_for_ui(date: Date) -> bool {
    date.day == 15 || date.day == 30.min(days_in_month(date.year, date.month))
}

fn biweekly_bill_occurs_for_ui(bill: &Bill, date: Date) -> bool {
    let Some(anchor) = bill_anchor_date_for_ui(bill) else {
        return false;
    };
    let delta = date_days(date) - date_days(anchor);
    delta >= 0 && delta % 14 == 0
}

fn bill_anchor_date_for_ui(bill: &Bill) -> Option<Date> {
    bill.anchor_date
        .as_deref()
        .and_then(parse_iso_date)
        .map(simple_date_to_date)
        .or_else(|| {
            let month = bill.renewal_month.clamp(1, 12);
            let day = bill.due_day.min(days_in_month(2026, month));
            Some(Date {
                year: 2026,
                month,
                day,
            })
        })
}

fn simple_date_to_date(date: SimpleDate) -> Date {
    Date {
        year: date.year,
        month: date.month,
        day: date.day,
    }
}

#[component]
fn TrendsView(state: RwSignal<PlannerState>) -> impl IntoView {
    view! {
        <section class="view active">
            <div class="section-heading">
                <div>
                    <h3>{move || t("Increase analysis")}</h3>
                    <p>{move || t("Compare the latest increase against each bill's historical average.")}</p>
                </div>
            </div>
            <div class="insight-grid">
                {move || state.get().bills.into_iter().map(|bill| {
                    let (average, latest) = historical_increase_summary(&bill);
                    let delta = latest - average;
                    view! {
                        <article class="insight-card">
                            <div class="row-top"><span>{bill.name}</span><span>{format!("{:.1}% {}", bill.annual_increase, t("forecast"))}</span></div>
                            <div class="row-sub">{format!("{}: {:.1}%", t("Historical average"), average)}</div>
                            <div class={if delta > 1.0 { "row-sub negative" } else if delta < -1.0 { "row-sub positive" } else { "row-sub" }}>
                                {format!("{}: {:.1}%, {:.1} {} {} {}", t("Latest change"), latest, delta.abs(), t("points"), if delta >= 0.0 { t("above") } else { t("below") }, t("average"))}
                            </div>
                        </article>
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

#[component]
fn SettingsView(
    state: RwSignal<PlannerState>,
    is_importing: RwSignal<bool>,
    show_tutorial: RwSignal<bool>,
    tutorial_step: RwSignal<TutorialStep>,
) -> impl IntoView {
    let is_editing_token = RwSignal::new(false);
    let language_preference =
        use_context::<RwSignal<LanguagePreference>>().expect("language preference context");

    view! {
        <section class="view active">
            <div class="settings-grid">
                <section class="form-panel">
                    <h3>{move || t("Display")}</h3>
                    <label>
                        {move || t("Language")}
                        <select
                            prop:value=move || language_preference.get().code()
                            on:change=move |event| {
                                let next = match event_target_value(&event).as_str() {
                                    "en" => LanguagePreference::English,
                                    "fr" => LanguagePreference::French,
                                    _ => LanguagePreference::Browser,
                                };
                                language_preference.set(next);
                            }
                        >
                            <option value="browser">{move || t("Browser default")}</option>
                            <option value="en">{move || t("English")}</option>
                            <option value="fr">{move || t("French")}</option>
                        </select>
                    </label>
                    <p>{move || t("Browser default follows your browser language. Choosing a language saves that preference in a cookie.")}</p>
                </section>

                <section class="form-panel">
                    <h3>{move || t("Install button")}</h3>
                    <p>{move || t("Hide the install button on this device if Payflow is already on your home screen.")}</p>
                    <div class="settings-actions install-settings-actions">
                        <button class="secondary-button install-visible-only" type="button" data-install-hide>
                            {move || t("Hide install button")}
                        </button>
                        <button class="secondary-button install-hidden-only" type="button" data-install-show>
                            {move || t("Show install button")}
                        </button>
                    </div>
                </section>

                <section class="form-panel">
                    <h3>{move || t("Account setup")}</h3>
                    <SettingsMoney label="Starting balance" value=move || state.get().settings.starting_balance on_input=move |value| state.update(|state| state.settings.starting_balance = value) />
                    <SettingsMoney label="Minimum cash buffer" value=move || state.get().settings.minimum_buffer on_input=move |value| state.update(|state| state.settings.minimum_buffer = value) />
                </section>

                <section class="form-panel">
                    <h3>{move || t("Forecast controls")}</h3>
                    <SettingsSlider
                        label="Forecast years"
                        value=Signal::derive(move || state.get().settings.forecast_years as f64)
                        min=1.0
                        max=5.0
                        step=1.0
                        suffix=" years"
                        on_input=move |value| {
                            state.update(|state| state.settings.forecast_years = value.round() as u32);
                        }
                    />
                    <SettingsSlider
                        label="Safety margin"
                        value=Signal::derive(move || state.get().settings.margin_percent)
                        min=0.0
                        max=20.0
                        step=1.0
                        suffix="% per transfer"
                        on_input=move |value| {
                            state.update(|state| state.settings.margin_percent = value);
                        }
                    />
                </section>

                <section class="form-panel">
                    <h3>{move || t("Paycheck rules")}</h3>
                    <SettingsMoney label="Paycheck amount" value=move || state.get().settings.paycheck_amount on_input=move |value| state.update(|state| state.settings.paycheck_amount = value) />
                    <p>{move || t("Used to warn when the recommended transfer would take too much of one paycheck.")}</p>
                </section>

                <section class="form-panel">
                    <h3>{move || t("Data")}</h3>
                    <p>{move || t("Reset the local planner, load demo data, or reopen the introduction.")}</p>
                    <div class="settings-actions">
                        <button class="secondary-button" type="button" on:click=move |_| {
                            state.update(|planner| {
                                let onboarding = planner.onboarding.clone();
                                *planner = PlannerState::sample();
                                planner.onboarding = onboarding;
                            });
                        }>
                            {move || t("Sample")}
                        </button>
                        <button class="secondary-button" type="button" on:click=move |_| {
                            tutorial_step.set(TutorialStep::Welcome);
                            show_tutorial.set(true);
                        }>
                            {move || t("Show tutorial")}
                        </button>
                        <button class="secondary-button danger" type="button" on:click=move |_| {
                            state.update(|planner| {
                                let onboarding = planner.onboarding.clone();
                                *planner = PlannerState::default();
                                planner.onboarding = onboarding;
                            });
                        }>
                            {move || t("Clear")}
                        </button>
                    </div>
                </section>

                <section class="form-panel ynab-panel">
                    <h3>{move || t("YNAB import")}</h3>
                    <p>{move || t("Use a personal access token to import the dedicated recurring-payment account.")}</p>
                    {move || {
                        if state.get().ynab.access_token.is_empty() || is_editing_token.get() {
                            view! {
                                <label>
                                    {move || t("Personal access token")}
                                    <input
                                        type="password"
                                        placeholder=move || t("Paste YNAB token")
                                        prop:value=move || state.get().ynab.access_token
                                        on:focus=move |_| is_editing_token.set(true)
                                        on:input=move |event| {
                                            state.update(|state| state.ynab.access_token = event_target_value(&event));
                                        }
                                        on:blur=move |_| is_editing_token.set(false)
                                    />
                                </label>
                            }.into_any()
                        } else {
                            view! {
                                <div class="token-status">
                                    <div>
                                        <strong>{move || t("Token saved")}</strong>
                                        <span>{move || t("Stored locally in this browser.")}</span>
                                    </div>
                                    <button
                                        class="secondary-button"
                                        type="button"
                                        on:click=move |_| {
                                            state.update(|state| {
                                                state.ynab.access_token.clear();
                                                state.ynab.plan_id = None;
                                                state.ynab.account_id = None;
                                                state.ynab.available_plans.clear();
                                                state.ynab.available_accounts.clear();
                                                state.ynab.last_import_status = "Token removed".to_string();
                                            });
                                        }
                                    >
                                        {move || t("Forget token")}
                                    </button>
                                </div>
                            }.into_any()
                        }
                    }}
                    <label>
                        {move || t("Budget")}
                        <select
                            prop:value=move || state.get().ynab.plan_id.unwrap_or_default()
                            disabled=move || state.get().ynab.available_plans.is_empty()
                            on:change=move |event| {
                                let plan_id = event_target_value(&event);
                                state.update(|state| {
                                    state.ynab.plan_id = (!plan_id.is_empty()).then_some(plan_id.clone());
                                    state.ynab.account_id = None;
                                    state.ynab.available_accounts.clear();
                                });
                                run_ynab_account_load(state, is_importing);
                            }
                        >
                            <option value="">{move || t("Select budget")}</option>
                            {move || state.get().ynab.available_plans.into_iter().map(|plan| view! {
                                <option value=plan.id>{plan.name}</option>
                            }).collect_view()}
                        </select>
                    </label>
                    <label>
                        {move || t("Account")}
                        <select
                            prop:value=move || state.get().ynab.account_id.unwrap_or_default()
                            disabled=move || state.get().ynab.available_accounts.is_empty()
                            on:change=move |event| {
                                let account_id = event_target_value(&event);
                                state.update(|state| {
                                    state.ynab.account_id = (!account_id.is_empty()).then_some(account_id.clone());
                                    if let Some(account) = state.ynab.available_accounts.iter().find(|account| account.id == account_id) {
                                        state.ynab.account_name = account.name.clone();
                                    }
                                });
                            }
                        >
                            <option value="">{move || t("Select account")}</option>
                            {move || state.get().ynab.available_accounts.into_iter().map(|account| view! {
                                <option value=account.id>{account.name}</option>
                            }).collect_view()}
                        </select>
                    </label>
                    <div class="ynab-actions">
                        {move || {
                            let choices_loaded = {
                                let snapshot = state.get();
                                !snapshot.ynab.available_plans.is_empty()
                                    && !snapshot.ynab.available_accounts.is_empty()
                            };

                            if choices_loaded {
                                view! {
                                    <button
                                        class="primary-button"
                                        type="button"
                                        disabled=move || is_importing.get()
                                        on:click=move |_| run_ynab_import(state, is_importing)
                                    >
                                        {move || if is_importing.get() { t("Importing...") } else { t("Import from YNAB") }}
                                    </button>
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        class="primary-button"
                                        type="button"
                                        disabled=move || is_importing.get()
                                        on:click=move |_| run_ynab_choice_load(state, is_importing)
                                    >
                                        {move || if is_importing.get() { t("Loading...") } else { t("Load Accounts") }}
                                    </button>
                                }.into_any()
                            }
                        }}
                    </div>
                    <div class="import-status">
                        <strong>{move || state.get().ynab.last_import_status}</strong>
                        <span>{move || state.get().ynab.last_imported_at.unwrap_or_else(|| t("Never imported").to_string())}</span>
                    </div>
                </section>
            </div>

        </section>
    }
}

#[component]
fn TransactionsView(state: RwSignal<PlannerState>) -> impl IntoView {
    let group_by_bills = RwSignal::new(true);
    let sort_column = RwSignal::new(TransactionSortColumn::Date);
    let sort_ascending = RwSignal::new(false);
    let page = RwSignal::new(0usize);

    view! {
        <section class="view active">
            <div class="transaction-grid">
                <section class="table-panel full-span">
                    <div class="section-heading">
                        <div>
                    <h3>{move || t("Transactions")}</h3>
                    <p>{move || t("Assign imported transactions to bills, leave them unassigned, mark them non-recurring, or create a bill from the dropdown.")}</p>
                        </div>
                        <div class="view-controls">
                            <button class="primary-button" type="button" on:click=move |_| {
                                add_transaction(state);
                                page.set(0);
                            }>
                                {move || t("Add Transaction")}
                            </button>
                            <label class="group-select">
                                <span>{move || t("Group")}</span>
                                <select
                                    aria-label=move || t("Group transactions")
                                    prop:value=move || if group_by_bills.get() { "bills" } else { "none" }
                                    on:change=move |event| {
                                        group_by_bills.set(event_target_value(&event) == "bills");
                                        page.set(0);
                                    }
                                >
                                    <option value="none">{move || t("None")}</option>
                                    <option value="bills">{move || t("Bills")}</option>
                                </select>
                            </label>
                        </div>
                    </div>
                    {move || {
                        let snapshot = state.get();
                        let bills = snapshot.bills.clone();
                        let paychecks = snapshot.paychecks.clone();
                        let recurring_candidates = snapshot.recurring_candidates.clone();
                        let mut transactions = snapshot.transactions;
                        sort_transactions(
                            &mut transactions,
                            sort_column.get(),
                            sort_ascending.get(),
                            &bills,
                            &paychecks,
                            &recurring_candidates,
                        );

                        if group_by_bills.get() {
                            let groups = group_transactions_by_category(
                                transactions,
                                sort_column.get(),
                                sort_ascending.get(),
                                &bills,
                                &paychecks,
                                &recurring_candidates,
                            );
                            view! {
                                {transaction_grouped_table(state, groups, bills, paychecks, recurring_candidates, sort_column, sort_ascending, page)}
                            }.into_any()
                        } else {
                            let total = transactions.len();
                            let total_pages = page_count(total, TRANSACTIONS_PER_PAGE);
                            let current_page = page.get().min(total_pages.saturating_sub(1));
                            if current_page != page.get() {
                                page.set(current_page);
                            }
                            let start_index = current_page * TRANSACTIONS_PER_PAGE;
                            let page_transactions = transactions
                                .into_iter()
                                .skip(start_index)
                                .take(TRANSACTIONS_PER_PAGE)
                                .collect::<Vec<_>>();
                            view! {
                                <>
                                    <PaginationControls page total_items=total />
                                    {transaction_table(state, page_transactions, bills, paychecks, recurring_candidates, sort_column, sort_ascending, page)}
                                </>
                            }.into_any()
                        }
                    }}
                </section>

                <details class="table-panel full-span summary-panel">
                    <summary>{move || t("YNAB transaction summary")}</summary>
                    <div class="metrics-grid compact-metrics">
                        <Metric label="Recurring" value=move || money(class_total(&state.get(), TransactionClass::Recurring).abs()) note=move || format!("{} {}", class_count(&state.get(), TransactionClass::Recurring), t("matched")) />
                        <Metric label="Paycheck Transfers" value=move || money(class_total(&state.get(), TransactionClass::Paycheck).abs()) note=move || format!("{} {}", class_count(&state.get(), TransactionClass::Paycheck), t("matched")) />
                        <Metric label="Transfers" value=move || money(class_total(&state.get(), TransactionClass::Transfer)) note=move || format!("{} {}", class_count(&state.get(), TransactionClass::Transfer), t("imported")) />
                        <Metric label="No bill" value=move || money(class_total(&state.get(), TransactionClass::Misc).abs()) note=move || format!("{} {}", class_count(&state.get(), TransactionClass::Misc), t("transactions")) />
                        <Metric label="Imported" value=move || state.get().transactions.len().to_string() note=|| t("transactions").to_string() />
                    </div>
                </details>
            </div>
        </section>
    }
}

fn transaction_grouped_table(
    state: RwSignal<PlannerState>,
    groups: Vec<(String, Vec<TrackedTransaction>)>,
    bills: Vec<Bill>,
    paychecks: Vec<Bill>,
    recurring_candidates: Vec<RecurringCandidate>,
    sort_column: RwSignal<TransactionSortColumn>,
    sort_ascending: RwSignal<bool>,
    page: RwSignal<usize>,
) -> impl IntoView {
    let groups = groups
        .into_iter()
        .map(|(name, transactions)| {
            let count = transactions.len();
            let recurring_label = transaction_group_recurring_label(
                &transactions,
                &bills,
                &paychecks,
                &recurring_candidates,
            );
            let amount_label = transaction_group_amount_label(
                &transactions,
                &bills,
                &paychecks,
                &recurring_candidates,
            );
            let ids = transactions
                .into_iter()
                .map(|transaction| transaction.id)
                .collect::<Vec<_>>();
            (name, count, recurring_label, amount_label, ids)
        })
        .collect::<Vec<_>>();

    view! {
        <div class="table-wrap transaction-table-wrap" data-testid="transactions-table">
            <table class="transaction-table grouped-transaction-table">
                {transaction_table_head(sort_column, sort_ascending, page)}
                <tbody>
                    <For
                        each=move || groups.clone()
                        key=|(name, _, _, _, _)| name.clone()
                        children=move |(name, count, recurring_label, amount_label, ids)| view! {
                            <TransactionGroupRows
                                state=state
                                name=name
                                count=count
                                recurring_label=recurring_label
                                amount_label=amount_label
                                ids=ids
                                bills=bills.clone()
                                paychecks=paychecks.clone()
                                recurring_candidates=recurring_candidates.clone()
                            />
                        }
                    />
                </tbody>
            </table>
        </div>
    }
}

fn transaction_table(
    state: RwSignal<PlannerState>,
    transactions: Vec<TrackedTransaction>,
    bills: Vec<Bill>,
    paychecks: Vec<Bill>,
    recurring_candidates: Vec<RecurringCandidate>,
    sort_column: RwSignal<TransactionSortColumn>,
    sort_ascending: RwSignal<bool>,
    page: RwSignal<usize>,
) -> impl IntoView {
    let transaction_ids = transactions
        .into_iter()
        .map(|transaction| transaction.id)
        .collect::<Vec<_>>();

    view! {
        <div class="table-wrap transaction-table-wrap" data-testid="transactions-table">
            <table class="transaction-table">
                {transaction_table_head(sort_column, sort_ascending, page)}
                <tbody>
                    <For
                        each={move || transaction_ids.clone()}
                        key=|id| id.clone()
                        children={move |id| view! {
                            <TransactionRow state=state id=id bills=bills.clone() paychecks=paychecks.clone() recurring_candidates=recurring_candidates.clone() />
                        }}
                    />
                </tbody>
            </table>
        </div>
    }
}

fn transaction_table_head(
    sort_column: RwSignal<TransactionSortColumn>,
    sort_ascending: RwSignal<bool>,
    page: RwSignal<usize>,
) -> impl IntoView {
    view! {
        <thead>
            <tr>
                <th></th>
                <th>
                    <TransactionSortHeader label="Date" column=TransactionSortColumn::Date sort_column sort_ascending page />
                </th>
                <th>
                    <TransactionSortHeader label="Bills / Paycheck Transfers" column=TransactionSortColumn::Bills sort_column sort_ascending page />
                </th>
                <th>
                    <TransactionSortHeader label="Recurring" column=TransactionSortColumn::Recurring sort_column sort_ascending page />
                </th>
                <th>
                    <TransactionSortHeader label="Amount" column=TransactionSortColumn::Amount sort_column sort_ascending page />
                </th>
            </tr>
        </thead>
    }
}

#[component]
fn TransactionGroupRows(
    state: RwSignal<PlannerState>,
    name: String,
    count: usize,
    recurring_label: String,
    amount_label: String,
    ids: Vec<String>,
    bills: Vec<Bill>,
    paychecks: Vec<Bill>,
    recurring_candidates: Vec<RecurringCandidate>,
) -> impl IntoView {
    let expanded = RwSignal::new(true);
    let name = RwSignal::new(name);
    let recurring_label = RwSignal::new(recurring_label);
    let amount_label = RwSignal::new(amount_label);
    let ids = RwSignal::new(ids);
    let bills = RwSignal::new(bills);
    let paychecks = RwSignal::new(paychecks);
    let recurring_candidates = RwSignal::new(recurring_candidates);
    let is_unassigned_group = Memo::new(move |_| name.get() == t("Unassigned"));

    view! {
        <tr class=move || if is_unassigned_group.get() { "transaction-group-row is-pinned" } else { "transaction-group-row" } data-testid="transaction-group-row">
            <td colspan="5">
                <button
                    class="transaction-group-toggle"
                    type="button"
                    aria-expanded=move || expanded.get().to_string()
                    aria-label=move || {
                        if expanded.get() {
                            format!("{} {}", t("Collapse"), name.get())
                        } else {
                            format!("{} {}", t("Expand"), name.get())
                        }
                    }
                    on:click=move |_| expanded.update(|value| *value = !*value)
                >
                    <span class="group-caret" aria-hidden="true"></span>
                    <span class="group-name">{move || name.get()}</span>
                    <span class="group-count">{format!("{count} {}", t("transactions"))}</span>
                    <span class="group-recurring">{move || recurring_label.get()}</span>
                    <span class="group-amount">{move || amount_label.get()}</span>
                    <span class="group-pin-note">
                        {move || if is_unassigned_group.get() { t("Pinned last") } else { "" }}
                    </span>
                </button>
            </td>
        </tr>
        <Show when=move || expanded.get()>
            <For
                each=move || ids.get()
                key=|id| id.clone()
                children=move |id| view! {
                    <TransactionRow state=state id=id bills=bills.get() paychecks=paychecks.get() recurring_candidates=recurring_candidates.get() />
                }
            />
        </Show>
    }
}

#[component]
fn TransactionSortHeader(
    label: &'static str,
    column: TransactionSortColumn,
    sort_column: RwSignal<TransactionSortColumn>,
    sort_ascending: RwSignal<bool>,
    page: RwSignal<usize>,
) -> impl IntoView {
    view! {
        <button
            class=move || {
                if sort_column.get() == column {
                    "sort-header active"
                } else {
                    "sort-header"
                }
            }
            type="button"
            on:click=move |_| {
                if sort_column.get() == column {
                    sort_ascending.update(|ascending| *ascending = !*ascending);
                } else {
                    sort_column.set(column);
                    sort_ascending.set(column.default_ascending());
                }
                page.set(0);
            }
        >
            <span>{move || t(label)}</span>
            <span class="sort-indicator">
                {move || {
                    if sort_column.get() != column {
                        String::new()
                    } else if sort_ascending.get() {
                        t("Asc").to_string()
                    } else {
                        t("Desc").to_string()
                    }
                }}
            </span>
        </button>
    }
}

#[component]
fn TransactionRow(
    state: RwSignal<PlannerState>,
    id: String,
    bills: Vec<Bill>,
    paychecks: Vec<Bill>,
    recurring_candidates: Vec<RecurringCandidate>,
) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let row_id = id.clone();
    let transaction = Memo::new(move |_| {
        state
            .get()
            .transactions
            .into_iter()
            .find(|transaction| transaction.id == row_id)
    });
    let date_id = RwSignal::new(id.clone());
    let amount_id = RwSignal::new(id.clone());
    let select_id = RwSignal::new(id.clone());
    let payee_id = RwSignal::new(id.clone());
    let memo_id = RwSignal::new(id.clone());
    let delete_id = RwSignal::new(id.clone());
    let bills_for_select_value = bills.clone();
    let bills_for_recurring = bills.clone();
    let paychecks_for_select_value = paychecks.clone();
    let paychecks_for_recurring = paychecks.clone();
    let candidates_for_recurring = recurring_candidates.clone();

    view! {
        <tr class=move || if expanded.get() { "transaction-row is-expanded" } else { "transaction-row" } data-testid="transaction-row">
            <td class="transaction-expand-cell">
                <button
                    class="bill-expand-button"
                    type="button"
                    aria-label=move || if expanded.get() { t("Hide transaction details") } else { t("Show transaction details") }
                    aria-expanded=move || expanded.get().to_string()
                    on:click=move |_| expanded.update(|value| *value = !*value)
                ></button>
            </td>
            <td class="date-cell">
                <InlineIsoDateCell
                    value=move || transaction.get().map(|transaction| transaction.date).unwrap_or_default()
                    on_input=move |value| update_transaction(state, date_id.get_untracked(), |transaction| transaction.date = value)
                />
            </td>
            <td class="transaction-bill-cell">
                <select
                    class="ledger-select"
                    aria-label=move || t("Bills and paycheck transfers")
                    prop:value=move || {
                        transaction
                            .get()
                            .map(|transaction| {
                                transaction_bill_select_value(
                                    &transaction,
                                    &bills_for_select_value,
                                    &paychecks_for_select_value,
                                )
                            })
                            .unwrap_or_else(|| BILL_SELECT_UNASSIGNED.to_string())
                    }
                    on:change=move |event| {
                        update_transaction_bill_assignment(state, select_id.get_untracked(), event_target_value(&event));
                    }
                >
                    <option value=BILL_SELECT_UNASSIGNED>{move || t("Unassigned")}</option>
                    <option value=BILL_SELECT_NON_RECURRING>{move || t("Non-Recurring")}</option>
                    {bills.iter().filter(|bill| is_assignable_bill(bill)).map(|bill| view! {
                        <option value=format!("bill:{}", bill.id)>{bill.name.clone()}</option>
                    }).collect_view()}
                    {paychecks.iter().filter(|paycheck| is_assignable_bill(paycheck)).map(|paycheck| view! {
                        <option value=format!("{PAYCHECK_SELECT_PREFIX}{}", paycheck.id)>{format!("{}: {}", t("Paycheck Transfer"), paycheck.name)}</option>
                    }).collect_view()}
                    <option value=BILL_SELECT_CREATE>{move || t("Create Bill")}</option>
                </select>
            </td>
            <td class="recurring-cell">
                {move || {
                    transaction.get()
                        .map(|transaction| {
                            transaction_recurring_label(
                                &transaction,
                                &bills_for_recurring,
                                &paychecks_for_recurring,
                                &candidates_for_recurring,
                            )
                        })
                        .unwrap_or_default()
                }}
            </td>
            <td class="amount-cell">
                <InlineSignedMoneyCell
                    value=move || transaction.get().map(|transaction| transaction.amount).unwrap_or_default()
                    on_input=move |value| update_transaction(state, amount_id.get_untracked(), |transaction| transaction.amount = value)
                />
            </td>
        </tr>
        <tr class=move || if expanded.get() { "transaction-detail-row" } else { "transaction-detail-row is-hidden" }>
            <td></td>
            <td colspan="4">
                <div class="transaction-detail-fields">
                    <div>
                        <span>{move || t("Payee")}</span>
                        <InlineTextCell
                            value=move || transaction.get().map(|transaction| transaction.payee_name).unwrap_or_default()
                            on_input=move |value| update_transaction(state, payee_id.get_untracked(), |transaction| transaction.payee_name = value)
                        />
                    </div>
                    <div>
                        <span>{move || t("Memo")}</span>
                        <InlineTextCell
                            value=move || transaction.get().map(|transaction| transaction.memo).unwrap_or_default()
                            on_input=move |value| update_transaction(state, memo_id.get_untracked(), |transaction| transaction.memo = value)
                        />
                    </div>
                    <div class="transaction-danger-zone">
                        <button
                            class="small-button danger"
                            type="button"
                            on:click=move |_| delete_transaction(state, delete_id.get_untracked())
                        >
                            {move || t("Delete")}
                        </button>
                    </div>
                </div>
            </td>
        </tr>
    }
}

#[component]
fn PaginationControls(page: RwSignal<usize>, total_items: usize) -> impl IntoView {
    let total_pages = page_count(total_items, TRANSACTIONS_PER_PAGE);
    view! {
        <div class="pagination-bar">
            <span>
                {move || {
                    if total_items == 0 {
                        t("No transactions").to_string()
                    } else {
                        let current_page = page.get().min(total_pages.saturating_sub(1));
                        let start = current_page * TRANSACTIONS_PER_PAGE + 1;
                        let end = ((current_page + 1) * TRANSACTIONS_PER_PAGE).min(total_items);
                        format!("{start}-{end} {} {total_items}", t("of"))
                    }
                }}
            </span>
            <div class="pagination-actions">
                <button
                    class="icon-button"
                    type="button"
                    disabled=move || page.get() == 0
                    on:click=move |_| page.update(|page| *page = page.saturating_sub(1))
                >
                    {move || t("Previous")}
                </button>
                <button
                    class="icon-button"
                    type="button"
                    disabled=move || { page.get() + 1 >= total_pages }
                    on:click=move |_| page.update(|page| {
                        if *page + 1 < total_pages {
                            *page += 1;
                        }
                    })
                >
                    {move || t("Next")}
                </button>
            </div>
        </div>
    }
}

const TRANSACTIONS_PER_PAGE: usize = 50;

#[derive(Clone, Copy, Eq, PartialEq)]
enum TransactionSortColumn {
    Date,
    Amount,
    Bills,
    Recurring,
}

impl TransactionSortColumn {
    fn default_ascending(self) -> bool {
        !matches!(self, TransactionSortColumn::Date)
    }
}

fn page_count(total_items: usize, page_size: usize) -> usize {
    total_items.div_ceil(page_size).max(1)
}

#[cfg(test)]
fn sort_transactions_by_date(transactions: &mut [TrackedTransaction], newest_first: bool) {
    transactions.sort_by(|left, right| {
        let date_order = if newest_first {
            right.date.cmp(&left.date)
        } else {
            left.date.cmp(&right.date)
        };

        date_order
            .then_with(|| left.payee_name.cmp(&right.payee_name))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn sort_transactions(
    transactions: &mut [TrackedTransaction],
    column: TransactionSortColumn,
    ascending: bool,
    bills: &[Bill],
    paychecks: &[Bill],
    recurring_candidates: &[RecurringCandidate],
) {
    transactions.sort_by(|left, right| {
        let order = match column {
            TransactionSortColumn::Date => left.date.cmp(&right.date),
            TransactionSortColumn::Amount => left
                .amount
                .total_cmp(&right.amount)
                .then_with(|| left.date.cmp(&right.date)),
            TransactionSortColumn::Bills => transaction_bill_sort_label(left, bills, paychecks)
                .cmp(&transaction_bill_sort_label(right, bills, paychecks))
                .then_with(|| left.date.cmp(&right.date)),
            TransactionSortColumn::Recurring => {
                transaction_recurring_label(left, bills, paychecks, recurring_candidates)
                    .cmp(&transaction_recurring_label(
                        right,
                        bills,
                        paychecks,
                        recurring_candidates,
                    ))
                    .then_with(|| left.date.cmp(&right.date))
            }
        };

        let order = if ascending { order } else { order.reverse() };
        order
            .then_with(|| left.payee_name.cmp(&right.payee_name))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn group_transactions_by_category(
    transactions: Vec<TrackedTransaction>,
    sort_column: TransactionSortColumn,
    sort_ascending: bool,
    bills: &[Bill],
    paychecks: &[Bill],
    recurring_candidates: &[RecurringCandidate],
) -> Vec<(String, Vec<TrackedTransaction>)> {
    let mut groups: Vec<(String, Vec<TrackedTransaction>)> = Vec::new();

    for transaction in transactions {
        let category = transaction_bill_sort_label(&transaction, bills, paychecks);
        if let Some((_, items)) = groups
            .iter_mut()
            .find(|(existing_category, _)| *existing_category == category)
        {
            items.push(transaction);
        } else {
            groups.push((category, vec![transaction]));
        }
    }

    for (_, transactions) in &mut groups {
        sort_transactions(
            transactions,
            sort_column,
            sort_ascending,
            bills,
            paychecks,
            recurring_candidates,
        );
    }

    groups.sort_by(|(left, _), (right, _)| group_sort_key(left).cmp(&group_sort_key(right)));
    groups
}

fn group_sort_key(name: &str) -> (u8, String) {
    if name == t("Unassigned") {
        (1, String::new())
    } else {
        (0, name.to_lowercase())
    }
}

fn transaction_bill_select_value(
    transaction: &TrackedTransaction,
    bills: &[Bill],
    paychecks: &[Bill],
) -> String {
    if transaction.classification == TransactionClass::Paycheck {
        if let Some(paycheck_id) = transaction.matched_bill_id {
            if paychecks
                .iter()
                .any(|paycheck| paycheck.id == paycheck_id && is_assignable_bill(paycheck))
            {
                return format!("{PAYCHECK_SELECT_PREFIX}{paycheck_id}");
            }
        }
    }

    if let Some(bill_id) = transaction.matched_bill_id {
        if bills
            .iter()
            .any(|bill| bill.id == bill_id && is_assignable_bill(bill))
        {
            return format!("bill:{bill_id}");
        }
    }

    let category = normalize_category_name(&transaction.category_name);
    if category == NON_RECURRING_CATEGORY {
        return BILL_SELECT_NON_RECURRING.to_string();
    }

    if let Some(bill_id) = bill_id_for_category(bills, &category) {
        return format!("bill:{bill_id}");
    }

    BILL_SELECT_UNASSIGNED.to_string()
}

fn transaction_bill_sort_label(
    transaction: &TrackedTransaction,
    bills: &[Bill],
    paychecks: &[Bill],
) -> String {
    if transaction.classification == TransactionClass::Paycheck {
        if let Some(paycheck_id) = transaction.matched_bill_id {
            if paychecks
                .iter()
                .any(|paycheck| paycheck.id == paycheck_id && is_assignable_bill(paycheck))
            {
                return t("Paycheck Transfers").to_string();
            }
        }

        return t("Paycheck Transfers").to_string();
    }

    if let Some(bill_id) = transaction.matched_bill_id {
        if let Some(bill) = bills
            .iter()
            .find(|bill| bill.id == bill_id && is_assignable_bill(bill))
        {
            return bill.name.clone();
        }
    }

    let category = normalize_category_name(&transaction.category_name);
    if category == NON_RECURRING_CATEGORY {
        NON_RECURRING_CATEGORY.to_string()
    } else if bill_id_for_category(bills, &category).is_some() {
        category
    } else {
        t("Unassigned").to_string()
    }
}

fn transaction_recurring_label(
    transaction: &TrackedTransaction,
    bills: &[Bill],
    paychecks: &[Bill],
    recurring_candidates: &[RecurringCandidate],
) -> String {
    if normalize_category_name(&transaction.category_name) == NON_RECURRING_CATEGORY {
        return t("Non-recurring").to_string();
    }

    if let Some(bill_id) = transaction.matched_bill_id {
        if transaction.classification == TransactionClass::Paycheck {
            if let Some(paycheck) = paychecks
                .iter()
                .find(|paycheck| paycheck.id == bill_id && is_assignable_bill(paycheck))
            {
                return frequency_label(paycheck.frequency).to_string();
            }
        }

        if let Some(bill) = bills
            .iter()
            .find(|bill| bill.id == bill_id && is_assignable_bill(bill))
        {
            return frequency_label(bill.frequency).to_string();
        }
    }

    let category = normalize_category_name(&transaction.category_name);
    if let Some(bill) = bills.iter().find(|bill| {
        is_assignable_bill(bill)
            && normalize_name_for_ui(&bill.name) == normalize_name_for_ui(&category)
    }) {
        return frequency_label(bill.frequency).to_string();
    }

    if transaction.manual_classification == Some(TransactionClass::Misc) {
        return t("Unassigned").to_string();
    }

    if let Some(candidate) = recurring_candidates
        .iter()
        .find(|candidate| candidate_key(candidate) == recurring_key(transaction))
    {
        return cadence_label(candidate.cadence).to_string();
    }

    if transaction.classification == TransactionClass::Recurring {
        return t("Recurring").to_string();
    }

    if transaction.classification == TransactionClass::Paycheck {
        return t("Paycheck Transfer").to_string();
    }

    t("Unassigned").to_string()
}

fn transaction_group_recurring_label(
    transactions: &[TrackedTransaction],
    bills: &[Bill],
    paychecks: &[Bill],
    recurring_candidates: &[RecurringCandidate],
) -> String {
    transactions
        .iter()
        .max_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|transaction| {
            transaction_recurring_label(transaction, bills, paychecks, recurring_candidates)
        })
        .unwrap_or_else(|| t("Unassigned").to_string())
}

fn transaction_group_amount_label(
    transactions: &[TrackedTransaction],
    bills: &[Bill],
    paychecks: &[Bill],
    recurring_candidates: &[RecurringCandidate],
) -> String {
    transactions
        .iter()
        .filter(|transaction| {
            let label =
                transaction_recurring_label(transaction, bills, paychecks, recurring_candidates);
            label != t("Unassigned") && label != t("Non-recurring")
        })
        .max_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|transaction| money(transaction.amount.abs()))
        .unwrap_or_else(|| "-".to_string())
}

fn cadence_label(cadence: RecurringCadence) -> &'static str {
    t(match cadence {
        RecurringCadence::Weekly => "Weekly",
        RecurringCadence::Biweekly => "Biweekly",
        RecurringCadence::Semimonthly => "Twice monthly",
        RecurringCadence::Monthly => "Monthly",
        RecurringCadence::Quarterly => "Quarterly",
        RecurringCadence::Yearly => "Yearly",
        RecurringCadence::Irregular => "Irregular",
    })
}

fn frequency_label(frequency: Frequency) -> &'static str {
    t(match frequency {
        Frequency::Biweekly => "Biweekly",
        Frequency::Semimonthly => "Twice monthly",
        Frequency::Monthly => "Monthly",
        Frequency::Quarterly => "Quarterly",
        Frequency::Yearly => "Yearly",
    })
}

fn normalize_category_name(category_name: &str) -> String {
    let trimmed = category_name.trim();
    if is_unassigned_category_name(trimmed) {
        DEFAULT_CATEGORY_NAME.to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_unassigned_category_name(category_name: &str) -> bool {
    let trimmed = category_name.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case(DEFAULT_CATEGORY_NAME)
        || trimmed.eq_ignore_ascii_case(UNCATEGORIZED_CATEGORY)
}

fn is_assignable_bill(bill: &Bill) -> bool {
    !is_unassigned_category_name(&bill.name)
}

fn normalize_transaction_categories(transactions: &mut [TrackedTransaction]) {
    for transaction in transactions {
        transaction.category_name = normalize_category_name(&transaction.category_name);
    }
}

#[component]
fn SettingsMoney(
    label: &'static str,
    value: impl Fn() -> f64 + Copy + Send + 'static,
    on_input: impl Fn(f64) + Copy + 'static,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let raw_value = RwSignal::new(money_input(value()));
    let commit = move || {
        if let Some(value) = parse_money(&raw_value.get_untracked()) {
            on_input(value);
        }
        editing.set(false);
    };

    Effect::new(move |_| {
        if !editing.get() {
            raw_value.set(money_input(value()));
        }
    });

    view! {
        <label>
            {move || t(label)}
            <input
                type="text"
                inputmode="decimal"
                prop:value=move || raw_value.get()
                on:focus=move |_| {
                    editing.set(true);
                    raw_value.set(money_input(value()));
                }
                on:input=move |event| raw_value.set(event_target_value(&event))
                on:keydown=move |event| {
                    if event.key() == "Enter" {
                        event.prevent_default();
                        commit();
                    } else if event.key() == "Escape" {
                        event.prevent_default();
                        editing.set(false);
                        raw_value.set(money_input(value()));
                    }
                }
                on:blur=move |_| commit()
            />
        </label>
    }
}

#[component]
fn SettingsSlider(
    label: &'static str,
    value: Signal<f64>,
    min: f64,
    max: f64,
    step: f64,
    suffix: &'static str,
    on_input: impl Fn(f64) + Copy + 'static,
) -> impl IntoView {
    view! {
        <label class="settings-slider">
            <span>{move || t(label)}</span>
            <input
                type="range"
                prop:min=min
                prop:max=max
                prop:step=step
                prop:value=move || value.get()
                on:input=move |event| on_input(event_target_value(&event).parse::<f64>().unwrap_or(min))
            />
            <strong>{move || format!("{:.0}{}", value.get(), t(suffix))}</strong>
        </label>
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewName {
    Dashboard,
    Bills,
    Transactions,
    Trends,
    Settings,
}

impl ViewName {
    fn label(self, language: Language) -> &'static str {
        tr(
            language,
            match self {
                ViewName::Dashboard => "Dashboard",
                ViewName::Bills => "Bills",
                ViewName::Transactions => "Transactions",
                ViewName::Trends => "Trends",
                ViewName::Settings => "Settings",
            },
        )
    }

    fn icon(self) -> &'static str {
        match self {
            ViewName::Dashboard => "⌂",
            ViewName::Bills => "$",
            ViewName::Transactions => "⇄",
            ViewName::Trends => "↗",
            ViewName::Settings => "⚙",
        }
    }

    fn short_label(self, language: Language) -> &'static str {
        tr(
            language,
            match self {
                ViewName::Dashboard => "Home",
                ViewName::Bills => "Bills",
                ViewName::Transactions => "Txns",
                ViewName::Trends => "Trends",
                ViewName::Settings => "Settings",
            },
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TutorialStep {
    Welcome,
    Dashboard,
    Bills,
    Transactions,
    Trends,
    Settings,
    YnabImport,
    ManualSetup,
}

impl TutorialStep {
    const COUNT: usize = 8;

    fn index(self) -> usize {
        match self {
            TutorialStep::Welcome => 0,
            TutorialStep::Dashboard => 1,
            TutorialStep::Bills => 2,
            TutorialStep::Transactions => 3,
            TutorialStep::Trends => 4,
            TutorialStep::Settings => 5,
            TutorialStep::YnabImport => 6,
            TutorialStep::ManualSetup => 7,
        }
    }

    fn next(self) -> Self {
        match self {
            TutorialStep::Welcome => TutorialStep::Dashboard,
            TutorialStep::Dashboard => TutorialStep::Bills,
            TutorialStep::Bills => TutorialStep::Transactions,
            TutorialStep::Transactions => TutorialStep::Trends,
            TutorialStep::Trends => TutorialStep::Settings,
            TutorialStep::Settings => TutorialStep::YnabImport,
            TutorialStep::YnabImport => TutorialStep::ManualSetup,
            TutorialStep::ManualSetup => TutorialStep::ManualSetup,
        }
    }

    fn previous(self) -> Self {
        match self {
            TutorialStep::Welcome => TutorialStep::Welcome,
            TutorialStep::Dashboard => TutorialStep::Welcome,
            TutorialStep::Bills => TutorialStep::Dashboard,
            TutorialStep::Transactions => TutorialStep::Bills,
            TutorialStep::Trends => TutorialStep::Transactions,
            TutorialStep::Settings => TutorialStep::Trends,
            TutorialStep::YnabImport => TutorialStep::Settings,
            TutorialStep::ManualSetup => TutorialStep::YnabImport,
        }
    }

    fn is_last(self) -> bool {
        self == TutorialStep::ManualSetup
    }

    fn view(self) -> Option<ViewName> {
        match self {
            TutorialStep::Welcome => None,
            TutorialStep::Dashboard => Some(ViewName::Dashboard),
            TutorialStep::Bills | TutorialStep::ManualSetup => Some(ViewName::Bills),
            TutorialStep::Transactions => Some(ViewName::Transactions),
            TutorialStep::Trends => Some(ViewName::Trends),
            TutorialStep::Settings | TutorialStep::YnabImport => Some(ViewName::Settings),
        }
    }

    fn title(self) -> &'static str {
        match self {
            TutorialStep::Welcome => "Welcome to Payflow Forecast",
            TutorialStep::Dashboard => "Dashboard",
            TutorialStep::Bills => "Bills",
            TutorialStep::Transactions => "Transactions",
            TutorialStep::Trends => "Trends",
            TutorialStep::Settings => "Settings",
            TutorialStep::YnabImport => "Start with YNAB",
            TutorialStep::ManualSetup => "Start manually",
        }
    }

    fn body(self) -> &'static str {
        match self {
            TutorialStep::Welcome => "This short introduction explains the app and the two normal ways to start: import from YNAB, or enter bills and paycheck transfers manually.",
            TutorialStep::Dashboard => "The dashboard shows whether the recurring-payment account can stay above your minimum cash buffer. The chart combines real history with the five-year forecast so low points stand out visually.",
            TutorialStep::Bills => "The Bills page is where recurring outflows and Paycheck Transfers live. Review the detected list after import, or create rows here when entering the plan yourself.",
            TutorialStep::Transactions => "The Transactions page lets you review imported activity, assign a transaction to a bill, mark it non-recurring, or create a bill from a transaction when the detector missed something.",
            TutorialStep::Trends => "Trends helps compare historical bill changes against the app forecast, especially when yearly renewals or price increases start to drift from expectation.",
            TutorialStep::Settings => "Settings controls the starting balance, minimum buffer, safety margin, paycheck amount, data reset, and YNAB connection.",
            TutorialStep::YnabImport => "To import from YNAB, paste your personal access token in Settings, load accounts, choose the budget and target account, then import. After import, review Bills and Transactions.",
            TutorialStep::ManualSetup => "To start without YNAB, open Bills, add recurring bills and Paycheck Transfers, then confirm Settings. The dashboard will forecast from those entries.",
        }
    }

    fn callout_title(self) -> &'static str {
        match self {
            TutorialStep::Welcome => "Existing users",
            TutorialStep::Dashboard => "What to watch",
            TutorialStep::Bills => "Most important review",
            TutorialStep::Transactions => "Correction flow",
            TutorialStep::Trends => "Use later",
            TutorialStep::Settings => "Do this early",
            TutorialStep::YnabImport => "Recommended path",
            TutorialStep::ManualSetup => "No import needed",
        }
    }

    fn callout_body(self) -> &'static str {
        match self {
            TutorialStep::Welcome => "Use Skip introduction if you already know the app. You can reopen this tutorial from Settings.",
            TutorialStep::Dashboard => "Lowest projected balance and recommended transfer are the key numbers.",
            TutorialStep::Bills => "Every recurring payment should have a clean bill row with amount, next due date, and frequency.",
            TutorialStep::Transactions => "The Bills dropdown is the source of truth for whether a transaction belongs to a recurring bill.",
            TutorialStep::Trends => "This becomes more useful once you have imported or entered enough history.",
            TutorialStep::Settings => "Set the starting balance and minimum buffer before trusting the forecast.",
            TutorialStep::YnabImport => "The token stays in this browser's local storage. Review the imported bills before relying on the forecast.",
            TutorialStep::ManualSetup => "Manual setup is enough for forecasting, but you will not get transaction history until you import.",
        }
    }
}

fn complete_tutorial(state: RwSignal<PlannerState>) {
    state.update(|state| state.onboarding.introduction_done = true);
}

fn current_language() -> Language {
    use_context::<RwSignal<Language>>()
        .map(|language| language.get())
        .unwrap_or(Language::English)
}

fn t(key: &'static str) -> &'static str {
    tr(current_language(), key)
}

fn tr(language: Language, key: &'static str) -> &'static str {
    if language == Language::English {
        return key;
    }

    match key {
        "Account" => "Compte",
        "Account setup" => "Configuration du compte",
        "Actual balance" => "Solde réel",
        "Adaptive transfers keep the account near the required reserve instead of accumulating excess cash." => {
            "Les virements adaptatifs gardent le compte près de la réserve requise au lieu d'accumuler trop d'argent."
        }
        "Add Bill" => "Ajouter une facture",
        "Add a paycheck transfer to forecast" => "Ajoutez un virement de paie pour prévoir",
        "Add Paycheck" => "Ajouter une paie",
        "Add or review recurring incoming transfers for this account." => {
            "Ajoutez ou révisez les virements entrants récurrents pour ce compte."
        }
        "Add paycheck amount" => "Ajouter le montant de la paie",
        "Add Transaction" => "Ajouter une transaction",
        "Amount" => "Montant",
        "Annual increase" => "Augmentation annuelle",
        "Asc" => "Asc",
        "Back" => "Retour",
        "Balance and activity" => "Solde et activité",
        "Biweekly" => "Aux deux semaines",
        "Bills" => "Factures",
        "Bills and paycheck transfers" => "Factures et virements de paie",
        "Bills / Paycheck Transfers" => "Factures / virements de paie",
        "Browser default" => "Langue du navigateur",
        "Browser default follows your browser language. Choosing a language saves that preference in a cookie." => {
            "La langue du navigateur suit votre navigateur. Choisir une langue enregistre cette préférence dans un cookie."
        }
        "Budget" => "Budget",
        "buffer floor" => "plancher du coussin",
        "Clear" => "Effacer",
        "Click to edit" => "Cliquer pour modifier",
        "Click any value to edit it. Expand a row for schedule and increase details." => {
            "Cliquez sur une valeur pour la modifier. Ouvrez une ligne pour voir l'échéancier et les détails d'augmentation."
        }
        "Add this site to your home screen." => "Ajoutez ce site à votre écran d'accueil.",
        "Add it to your home screen for a full-screen shortcut on this device." => {
            "Ajoutez-le à votre écran d'accueil pour avoir un raccourci plein écran sur cet appareil."
        }
        "Add to Home Screen" => "Ajouter à l'écran d'accueil",
        "Android: open the browser menu, then tap Install app or Add to Home screen." => {
            "Android : ouvrez le menu du navigateur, puis touchez Installer l'appli ou Ajouter à l'écran d'accueil."
        }
        "Close" => "Fermer",
        "Compare the latest increase against each bill's historical average." => {
            "Comparez la dernière augmentation avec la moyenne historique de chaque facture."
        }
        "Collapse" => "Réduire",
        "Collapse sidebar" => "Réduire le menu",
        "Create Bill" => "Créer une facture",
        "Dashboard" => "Tableau de bord",
        "Data" => "Données",
        "Date" => "Date",
        "Delete" => "Supprimer",
        "Desc" => "Desc",
        "Display" => "Affichage",
        "English" => "Anglais",
        "Expand" => "Agrandir",
        "Expand sidebar" => "Agrandir le menu",
        "Expected increase" => "Augmentation prévue",
        "Finish" => "Terminer",
        "For yearly and quarterly bills, this month anchors the payment schedule. For monthly bills, it is when yearly increases begin in the projection." => {
            "Pour les factures annuelles et trimestrielles, ce mois ancre l'échéancier. Pour les factures mensuelles, il indique quand les augmentations annuelles commencent dans la projection."
        }
        "Forecast" => "Prévision",
        "Forecast controls" => "Paramètres de prévision",
        "Forecast years" => "Années de prévision",
        "forecast" => "prévu",
        "Forget token" => "Oublier le jeton",
        "Frequency" => "Fréquence",
        "French" => "Français",
        "Group" => "Grouper",
        "Group transactions" => "Grouper les transactions",
        "Hide advanced bill fields" => "Masquer les détails de la facture",
        "Hide install button" => "Masquer le bouton d'installation",
        "Hide the install button on this device if Payflow is already on your home screen." => {
            "Masquez le bouton d'installation sur cet appareil si Payflow est déjà sur votre écran d'accueil."
        }
        "Hide paycheck transfer details" => "Masquer les détails du virement de paie",
        "Hide transaction details" => "Masquer les détails de la transaction",
        "Historical average" => "Moyenne historique",
        "Home" => "Accueil",
        "Import from YNAB" => "Importer depuis YNAB",
        "Importing..." => "Importation...",
        "Imported" => "Importées",
        "imported" => "importées",
        "Install app" => "Installer l'appli",
        "Install button" => "Bouton d'installation",
        "iPhone or iPad: open Safari, tap Share, then Add to Home Screen." => {
            "iPhone ou iPad : ouvrez Safari, touchez Partager, puis Ajouter à l'écran d'accueil."
        }
        "Inflow" => "Entrées",
        "Increase analysis" => "Analyse des augmentations",
        "Irregular" => "Irrégulier",
        "Latest change" => "Dernier changement",
        "Language" => "Langue",
        "left after transfer" => "restants après virement",
        "Load Accounts" => "Charger les comptes",
        "Loading..." => "Chargement...",
        "Lowest projected balance" => "Solde projeté le plus bas",
        "matched" => "associées",
        "Memo" => "Mémo",
        "Minimum cash buffer" => "Coussin minimal",
        "Monthly" => "Mensuel",
        "Name" => "Nom",
        "Never imported" => "Jamais importé",
        "Next" => "Suivant",
        "Next date" => "Prochaine date",
        "Next due date" => "Prochaine échéance",
        "Next paycheck" => "Prochaine paie",
        "Next week" => "La semaine prochaine",
        "No bill" => "Aucune facture",
        "No imported transactions in the past year" => "Aucune transaction importée dans la dernière année",
        "No paycheck transfers yet." => "Aucun virement de paie pour l'instant.",
        "No recurring paycheck transfers detected yet." => "Aucun virement de paie récurrent détecté pour l'instant.",
        "No transactions" => "Aucune transaction",
        "Non-recurring" => "Non récurrent",
        "Non-Recurring" => "Non récurrent",
        "None" => "Aucun",
        "of" => "sur",
        "on" => "le",
        "Outflow" => "Sorties",
        "Open Payflow Forecast from your device home screen like a regular app." => {
            "Ouvrez Payflow Forecast depuis l'écran d'accueil de votre appareil comme une appli normale."
        }
        "Past 12 months" => "12 derniers mois",
        "Paycheck amount" => "Montant de la paie",
        "Paycheck pressure" => "Pression sur la paie",
        "Paycheck rules" => "Règles de paie",
        "Payee" => "Bénéficiaire",
        "Payflow needs at least one paycheck transfer before it can estimate funding dates, recommended transfers, and projected balances." => {
            "Payflow a besoin d'au moins un virement de paie avant d'estimer les dates de financement, les virements recommandés et les soldes projetés."
        }
        "Paycheck Transfer" => "Virement de paie",
        "Paycheck Transfers" => "Virements de paie",
        "Personal access token" => "Jeton d'accès personnel",
        "Pinned last" => "Épinglé à la fin",
        "points" => "points",
        "Previous" => "Précédent",
        "Projected balance" => "Solde projeté",
        "Quarterly" => "Trimestriel",
        "Recurring" => "Récurrent",
        "Recurring account planner" => "Planificateur de compte récurrent",
        "Recurring incoming transfers detected from transactions." => "Virements entrants récurrents détectés à partir des transactions.",
        "Recurring payments" => "Paiements récurrents",
        "Reset the local planner, load demo data, or reopen the introduction." => {
            "Réinitialisez le plan local, chargez des données démo ou rouvrez l'introduction."
        }
        "Safety margin" => "Marge de sécurité",
        "Sample" => "Exemple",
        "Schedule / increase month" => "Mois d'échéance / d'augmentation",
        "Select account" => "Choisir un compte",
        "Select budget" => "Choisir un budget",
        "Settings" => "Paramètres",
        "Short by" => "Manque de",
        "Show tutorial" => "Afficher le tutoriel",
        "Show advanced bill fields" => "Afficher les détails de la facture",
        "Show install button" => "Afficher le bouton d'installation",
        "Show paycheck transfer details" => "Afficher les détails du virement de paie",
        "Show transaction details" => "Afficher les détails de la transaction",
        "Skip introduction" => "Passer l'introduction",
        "Start manually" => "Commencer manuellement",
        "Start with YNAB" => "Commencer avec YNAB",
        "Starting balance" => "Solde de départ",
        "Step" => "Étape",
        "Stored locally in this browser." => "Enregistré localement dans ce navigateur.",
        "% per transfer" => "% par virement",
        " years" => " ans",
        "Token saved" => "Jeton enregistré",
        "Today" => "Aujourd'hui",
        "Tomorrow" => "Demain",
        "to stay afloat" => "pour rester à flot",
        "Transactions" => "Transactions",
        "transactions" => "transactions",
        "Transfers" => "Virements",
        "Trends" => "Tendances",
        "Txns" => "Txns",
        "Twice monthly" => "Deux fois par mois",
        "Twice monthly paycheck transfers are scheduled on the 15th and 30th." => {
            "Les virements de paie deux fois par mois sont planifiés le 15 et le 30."
        }
        "Unassigned" => "Non assigné",
        "Upcoming payments" => "Paiements à venir",
        "Use Payflow like an app" => "Utiliser Payflow comme une appli",
        "Use a personal access token to import the dedicated recurring-payment account." => {
            "Utilisez un jeton d'accès personnel pour importer le compte dédié aux paiements récurrents."
        }
        "Used to warn when the recommended transfer would take too much of one paycheck." => {
            "Utilisé pour avertir lorsque le virement recommandé prendrait trop d'une paie."
        }
        "YNAB import" => "Import YNAB",
        "YNAB transaction summary" => "Sommaire des transactions YNAB",
        "Very high transfer" => "Virement très élevé",
        "Weekly" => "Hebdomadaire",
        "Welcome to Payflow Forecast" => "Bienvenue dans Payflow Forecast",
        "Yearly" => "Annuel",
        "above" => "au-dessus de la",
        "after payment" => "après paiement",
        "average" => "moyenne",
        "below" => "sous la",
        "This short introduction explains the app and the two normal ways to start: import from YNAB, or enter bills and paycheck transfers manually." => {
            "Cette courte introduction explique l'application et les deux façons de commencer : importer depuis YNAB, ou saisir les factures et virements de paie manuellement."
        }
        "Assign imported transactions to bills, leave them unassigned, mark them non-recurring, or create a bill from the dropdown." => {
            "Assignez les transactions importées aux factures, laissez-les non assignées, marquez-les non récurrentes ou créez une facture depuis la liste."
        }
        "The dashboard shows whether the recurring-payment account can stay above your minimum cash buffer. The chart combines real history with the five-year forecast so low points stand out visually." => {
            "Le tableau de bord montre si le compte de paiements récurrents peut rester au-dessus du coussin minimal. Le graphique combine l'historique réel avec la prévision sur cinq ans pour faire ressortir les creux."
        }
        "The Bills page is where recurring outflows and Paycheck Transfers live. Review the detected list after import, or create rows here when entering the plan yourself." => {
            "La page Factures contient les sorties récurrentes et les virements de paie. Révisez la liste détectée après l'import, ou créez des lignes ici pour une saisie manuelle."
        }
        "The Transactions page lets you review imported activity, assign a transaction to a bill, mark it non-recurring, or create a bill from a transaction when the detector missed something." => {
            "La page Transactions permet de réviser l'activité importée, d'assigner une transaction à une facture, de la marquer non récurrente, ou de créer une facture lorsqu'une détection manque quelque chose."
        }
        "Trends helps compare historical bill changes against the app forecast, especially when yearly renewals or price increases start to drift from expectation." => {
            "Tendances aide à comparer les changements historiques des factures avec la prévision, surtout lorsque les renouvellements annuels ou les hausses s'écartent des attentes."
        }
        "Settings controls the starting balance, minimum buffer, safety margin, paycheck amount, data reset, and YNAB connection." => {
            "Les paramètres contrôlent le solde de départ, le coussin minimal, la marge de sécurité, le montant de la paie, les données et la connexion YNAB."
        }
        "To import from YNAB, paste your personal access token in Settings, load accounts, choose the budget and target account, then import. After import, review Bills and Transactions." => {
            "Pour importer depuis YNAB, collez votre jeton dans Paramètres, chargez les comptes, choisissez le budget et le compte cible, puis importez. Ensuite, révisez Factures et Transactions."
        }
        "To start without YNAB, open Bills, add recurring bills and Paycheck Transfers, then confirm Settings. The dashboard will forecast from those entries." => {
            "Pour commencer sans YNAB, ouvrez Factures, ajoutez les factures récurrentes et les virements de paie, puis confirmez les Paramètres. Le tableau de bord prévoira à partir de ces entrées."
        }
        "Existing users" => "Utilisateurs existants",
        "What to watch" => "À surveiller",
        "Most important review" => "Révision la plus importante",
        "Correction flow" => "Flux de correction",
        "Use later" => "À utiliser plus tard",
        "Do this early" => "À faire tôt",
        "Recommended path" => "Parcours recommandé",
        "No import needed" => "Aucun import requis",
        "Use Skip introduction if you already know the app. You can reopen this tutorial from Settings." => {
            "Utilisez Passer l'introduction si vous connaissez déjà l'application. Vous pourrez rouvrir ce tutoriel dans Paramètres."
        }
        "Lowest projected balance and recommended transfer are the key numbers." => "Le solde projeté le plus bas et le virement recommandé sont les chiffres clés.",
        "Every recurring payment should have a clean bill row with amount, next due date, and frequency." => {
            "Chaque paiement récurrent devrait avoir une ligne de facture claire avec montant, prochaine échéance et fréquence."
        }
        "The Bills dropdown is the source of truth for whether a transaction belongs to a recurring bill." => {
            "La liste déroulante Factures est la référence pour savoir si une transaction appartient à une facture récurrente."
        }
        "This becomes more useful once you have imported or entered enough history." => "Cela devient plus utile après avoir importé ou saisi assez d'historique.",
        "Set the starting balance and minimum buffer before trusting the forecast." => "Réglez le solde de départ et le coussin minimal avant de vous fier à la prévision.",
        "The token stays in this browser's local storage. Review the imported bills before relying on the forecast." => {
            "Le jeton reste dans le stockage local de ce navigateur. Révisez les factures importées avant de vous fier à la prévision."
        }
        "Manual setup is enough for forecasting, but you will not get transaction history until you import." => {
            "La configuration manuelle suffit pour prévoir, mais l'historique des transactions exige un import."
        }
        _ => key,
    }
}

#[cfg(target_arch = "wasm32")]
fn load_language_preference() -> LanguagePreference {
    match read_language_cookie().as_deref() {
        Some("en") => LanguagePreference::English,
        Some("fr") => LanguagePreference::French,
        _ => LanguagePreference::Browser,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_language_preference() -> LanguagePreference {
    LanguagePreference::Browser
}

#[cfg(target_arch = "wasm32")]
fn persist_language_preference(preference: LanguagePreference) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };

    let cookie = match preference {
        LanguagePreference::Browser => {
            format!("{LANGUAGE_COOKIE_NAME}=; Path=/; Max-Age=0; SameSite=Lax")
        }
        LanguagePreference::English | LanguagePreference::French => format!(
            "{LANGUAGE_COOKIE_NAME}={}; Path=/; Max-Age=31536000; SameSite=Lax",
            preference.code()
        ),
    };

    let _ = js_sys::Reflect::set(
        document.as_ref(),
        &wasm_bindgen::JsValue::from_str("cookie"),
        &wasm_bindgen::JsValue::from_str(&cookie),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_language_preference(preference: LanguagePreference) {
    let _ = preference;
}

#[cfg(target_arch = "wasm32")]
fn read_language_cookie() -> Option<String> {
    let cookies = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| {
            js_sys::Reflect::get(
                document.as_ref(),
                &wasm_bindgen::JsValue::from_str("cookie"),
            )
            .ok()
        })
        .and_then(|value| value.as_string())?;

    cookies
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find_map(|(name, value)| {
            (name == LANGUAGE_COOKIE_NAME).then(|| value.trim().to_lowercase())
        })
}

#[cfg(target_arch = "wasm32")]
fn detect_browser_language() -> Language {
    web_sys::window()
        .and_then(|window| window.navigator().language())
        .filter(|language| language.to_lowercase().starts_with("fr"))
        .map(|_| Language::French)
        .unwrap_or(Language::English)
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_browser_language() -> Language {
    Language::English
}

fn add_bill(state: RwSignal<PlannerState>) {
    state.update(|state| {
        let id = state.bills.iter().map(|bill| bill.id).max().unwrap_or(0) + 1;
        let today = Date::today();
        state.bills.push(Bill {
            id,
            name: "New bill".to_string(),
            amount: 100.0,
            due_day: today.day,
            frequency: Frequency::Monthly,
            annual_increase: 3.0,
            renewal_month: today.month,
            anchor_date: Some(date_input_value(today)),
            history: Vec::new(),
        });
    });
}

fn add_paycheck(state: RwSignal<PlannerState>) {
    state.update(|state| {
        let id = state
            .paychecks
            .iter()
            .map(|paycheck| paycheck.id)
            .max()
            .unwrap_or(0)
            + 1;
        let next_thursday = next_thursday_after(Date::today());
        state.paychecks.push(Bill {
            id,
            name: "Paycheck transfer".to_string(),
            amount: 1000.0,
            due_day: next_thursday.day,
            frequency: Frequency::Biweekly,
            annual_increase: 0.0,
            renewal_month: next_thursday.month,
            anchor_date: Some(date_input_value(next_thursday)),
            history: Vec::new(),
        });
    });
}

fn next_thursday_after(mut date: Date) -> Date {
    loop {
        date = date.next_day();
        if weekday_index(date) == 4 {
            return date;
        }
    }
}

fn weekday_index(date: Date) -> i64 {
    (date_days(date) + 4).rem_euclid(7)
}

fn add_transaction(state: RwSignal<PlannerState>) {
    state.update(|state| {
        let today = Date::today();
        let id = next_manual_transaction_id(&state.transactions);
        state.transactions.push(TrackedTransaction {
            id,
            date: date_input_value(today),
            payee_name: "New transaction".to_string(),
            category_name: DEFAULT_CATEGORY_NAME.to_string(),
            memo: String::new(),
            amount: 0.0,
            classification: TransactionClass::Misc,
            matched_bill_id: None,
            manual_classification: Some(TransactionClass::Misc),
        });
    });
}

fn next_manual_transaction_id(transactions: &[TrackedTransaction]) -> String {
    let mut index = transactions.len() + 1;
    loop {
        let id = format!("manual-{index}");
        if transactions.iter().all(|transaction| transaction.id != id) {
            return id;
        }
        index += 1;
    }
}

fn update_bill(state: RwSignal<PlannerState>, id: u32, update: impl FnOnce(&mut Bill)) {
    state.update(|state| {
        if let Some(bill) = state.bills.iter_mut().find(|bill| bill.id == id) {
            update(bill);
        }
    });
}

fn update_bill_frequency(state: RwSignal<PlannerState>, id: u32, frequency: Frequency) {
    update_bill(state, id, |bill| {
        let anchor = next_bill_due_date(bill, Date::today());
        bill.frequency = frequency;
        if frequency == Frequency::Biweekly && bill.anchor_date.is_none() {
            bill.anchor_date = Some(date_input_value(anchor));
        }
    });
}

fn update_paycheck(state: RwSignal<PlannerState>, id: u32, update: impl FnOnce(&mut Bill)) {
    state.update(|state| {
        if let Some(paycheck) = state
            .paychecks
            .iter_mut()
            .find(|paycheck| paycheck.id == id)
        {
            update(paycheck);
        }
    });
}

fn update_paycheck_frequency(state: RwSignal<PlannerState>, id: u32, frequency: Frequency) {
    update_paycheck(state, id, |paycheck| {
        let anchor = next_bill_due_date(paycheck, Date::today());
        paycheck.frequency = frequency;
        if frequency == Frequency::Biweekly && paycheck.anchor_date.is_none() {
            paycheck.anchor_date = Some(date_input_value(anchor));
        }
    });
}

fn update_transaction(
    state: RwSignal<PlannerState>,
    transaction_id: String,
    update: impl FnOnce(&mut TrackedTransaction),
) {
    state.update(|state| {
        if let Some(transaction) = state
            .transactions
            .iter_mut()
            .find(|transaction| transaction.id == transaction_id)
        {
            update(transaction);
        }
        refresh_recurring_detection(state);
    });
}

fn update_bill_next_due_date(state: RwSignal<PlannerState>, id: u32, value: String) {
    let Some(date) = parse_iso_date(&value) else {
        return;
    };

    update_bill(state, id, |bill| {
        bill.due_day = date.day.clamp(1, 31);
        bill.renewal_month = date.month.clamp(1, 12);
        bill.anchor_date = Some(value);
    });
}

fn update_paycheck_next_date(state: RwSignal<PlannerState>, id: u32, value: String) {
    let Some(date) = parse_iso_date(&value) else {
        return;
    };

    update_paycheck(state, id, |paycheck| {
        paycheck.due_day = date.day.clamp(1, 31);
        paycheck.renewal_month = date.month.clamp(1, 12);
        paycheck.anchor_date = Some(value);
    });
}

fn update_transaction_bill_assignment(
    state: RwSignal<PlannerState>,
    transaction_id: String,
    value: String,
) {
    if value == BILL_SELECT_CREATE {
        create_and_assign_bill(state, transaction_id);
        return;
    }

    state.update(|state| {
        if let Some(bill_id) = value
            .strip_prefix("bill:")
            .and_then(|id| id.parse::<u32>().ok())
        {
            let bill_name = state
                .bills
                .iter()
                .find(|bill| bill.id == bill_id)
                .map(|bill| bill.name.clone());
            if let Some(transaction) = state
                .transactions
                .iter_mut()
                .find(|transaction| transaction.id == transaction_id)
            {
                transaction.category_name =
                    bill_name.unwrap_or_else(|| DEFAULT_CATEGORY_NAME.to_string());
                transaction.classification = TransactionClass::Recurring;
                transaction.matched_bill_id = Some(bill_id);
                transaction.manual_classification = Some(TransactionClass::Recurring);
            }
        } else if let Some(paycheck_id) = value
            .strip_prefix(PAYCHECK_SELECT_PREFIX)
            .and_then(|id| id.parse::<u32>().ok())
        {
            let paycheck_name = state
                .paychecks
                .iter()
                .find(|paycheck| paycheck.id == paycheck_id)
                .map(|paycheck| paycheck.name.clone());
            if let Some(transaction) = state
                .transactions
                .iter_mut()
                .find(|transaction| transaction.id == transaction_id)
            {
                transaction.category_name =
                    paycheck_name.unwrap_or_else(|| "Paycheck Transfer".to_string());
                transaction.classification = TransactionClass::Paycheck;
                transaction.matched_bill_id = Some(paycheck_id);
                transaction.manual_classification = Some(TransactionClass::Paycheck);
            }
        } else if value == BILL_SELECT_NON_RECURRING {
            if let Some(transaction) = state
                .transactions
                .iter_mut()
                .find(|transaction| transaction.id == transaction_id)
            {
                transaction.category_name = NON_RECURRING_CATEGORY.to_string();
                transaction.classification = TransactionClass::Misc;
                transaction.matched_bill_id = None;
                transaction.manual_classification = Some(TransactionClass::Misc);
            }
        } else if let Some(transaction) = state
            .transactions
            .iter_mut()
            .find(|transaction| transaction.id == transaction_id)
        {
            transaction.category_name = DEFAULT_CATEGORY_NAME.to_string();
            transaction.classification = if transaction.amount >= 0.0 {
                TransactionClass::Transfer
            } else {
                TransactionClass::Misc
            };
            transaction.matched_bill_id = None;
            transaction.manual_classification = None;
        }

        refresh_recurring_detection(state);
    });
}

fn delete_transaction(state: RwSignal<PlannerState>, transaction_id: String) {
    state.update(|state| {
        state
            .transactions
            .retain(|transaction| transaction.id != transaction_id);
        refresh_recurring_detection(state);
    });
}

fn create_and_assign_bill(state: RwSignal<PlannerState>, transaction_id: String) {
    state.update(|state| {
        let bill_id = create_bill_from_transaction(state, &transaction_id);
        let bill_name = bill_id.and_then(|id| {
            state
                .bills
                .iter()
                .find(|bill| bill.id == id)
                .map(|bill| bill.name.clone())
        });

        if let Some(transaction) = state
            .transactions
            .iter_mut()
            .find(|transaction| transaction.id == transaction_id)
        {
            transaction.classification = TransactionClass::Recurring;
            transaction.matched_bill_id = bill_id;
            transaction.manual_classification = Some(TransactionClass::Recurring);
            if let Some(name) = bill_name {
                transaction.category_name = name;
            }
        }

        refresh_recurring_detection(state);
    });
}

fn bill_id_for_category(bills: &[Bill], category_name: &str) -> Option<u32> {
    bills
        .iter()
        .find(|bill| {
            is_assignable_bill(bill)
                && normalize_name_for_ui(&bill.name) == normalize_name_for_ui(category_name)
        })
        .map(|bill| bill.id)
}

fn create_bill_from_transaction(state: &mut PlannerState, transaction_id: &str) -> Option<u32> {
    let transaction = state
        .transactions
        .iter()
        .find(|transaction| transaction.id == transaction_id)?;
    let parsed_date = parse_iso_date(&transaction.date);
    let category = normalize_category_name(&transaction.category_name);
    let name = if category == DEFAULT_CATEGORY_NAME {
        transaction.payee_name.trim()
    } else {
        category.trim()
    };
    let id = state.bills.iter().map(|bill| bill.id).max().unwrap_or(0) + 1;
    let cadence = state
        .recurring_candidates
        .iter()
        .find(|candidate| candidate_key(candidate) == recurring_key(transaction))
        .map(|candidate| candidate.cadence);

    state.bills.push(Bill {
        id,
        name: if name.is_empty() {
            "New bill".to_string()
        } else {
            name.to_string()
        },
        amount: transaction.amount.abs(),
        due_day: parsed_date.map(|date| date.day.clamp(1, 31)).unwrap_or(1),
        frequency: cadence
            .and_then(frequency_from_cadence)
            .unwrap_or(Frequency::Monthly),
        annual_increase: 3.0,
        renewal_month: parsed_date.map(|date| date.month.clamp(1, 12)).unwrap_or(1),
        anchor_date: parsed_date.map(simple_date_to_date).map(date_input_value),
        history: Vec::new(),
    });
    Some(id)
}

fn sync_detected_bills(state: &mut PlannerState) {
    let mut next_id = state.bills.iter().map(|bill| bill.id).max().unwrap_or(0) + 1;
    let mut next_paycheck_id = state
        .paychecks
        .iter()
        .map(|paycheck| paycheck.id)
        .max()
        .unwrap_or(0)
        + 1;
    for candidate in &state.recurring_candidates {
        let Some(frequency) = frequency_from_cadence(candidate.cadence) else {
            continue;
        };

        if candidate.average_amount > 0.0 {
            let name = paycheck_name_from_candidate(candidate);
            let parsed_date = parse_iso_date(&candidate.last_date);
            if let Some(paycheck) = state.paychecks.iter_mut().find(|paycheck| {
                normalize_name_for_ui(&paycheck.name) == normalize_name_for_ui(&name)
            }) {
                paycheck.amount = candidate.last_amount.abs();
                paycheck.due_day = parsed_date.map(|date| date.day.clamp(1, 31)).unwrap_or(1);
                paycheck.frequency = frequency;
                paycheck.renewal_month =
                    parsed_date.map(|date| date.month.clamp(1, 12)).unwrap_or(1);
                paycheck.anchor_date = parsed_date.map(simple_date_to_date).map(date_input_value);
                continue;
            }
            state.paychecks.push(Bill {
                id: next_paycheck_id,
                name,
                amount: candidate.last_amount.abs(),
                due_day: parsed_date.map(|date| date.day.clamp(1, 31)).unwrap_or(1),
                frequency,
                annual_increase: 0.0,
                renewal_month: parsed_date.map(|date| date.month.clamp(1, 12)).unwrap_or(1),
                anchor_date: parsed_date.map(simple_date_to_date).map(date_input_value),
                history: Vec::new(),
            });
            next_paycheck_id += 1;
            continue;
        }

        let name = bill_name_from_candidate(candidate);
        if state
            .bills
            .iter()
            .any(|bill| normalize_name_for_ui(&bill.name) == normalize_name_for_ui(&name))
        {
            continue;
        }
        let parsed_date = parse_iso_date(&candidate.last_date);
        state.bills.push(Bill {
            id: next_id,
            name,
            amount: candidate.average_amount.abs(),
            due_day: parsed_date.map(|date| date.day.clamp(1, 31)).unwrap_or(1),
            frequency,
            annual_increase: 3.0,
            renewal_month: parsed_date.map(|date| date.month.clamp(1, 12)).unwrap_or(1),
            anchor_date: parsed_date.map(simple_date_to_date).map(date_input_value),
            history: Vec::new(),
        });
        next_id += 1;
    }
}

fn paycheck_name_from_candidate(candidate: &RecurringCandidate) -> String {
    let payee = candidate.payee_name.trim();
    if !payee.is_empty() {
        return payee.to_string();
    }

    let category = normalize_category_name(&candidate.category_name);
    if category != DEFAULT_CATEGORY_NAME {
        category
    } else {
        "Paycheck Transfer".to_string()
    }
}

fn bill_name_from_candidate(candidate: &RecurringCandidate) -> String {
    let category = normalize_category_name(&candidate.category_name);
    if category != DEFAULT_CATEGORY_NAME {
        category
    } else {
        candidate.payee_name.trim().to_string()
    }
}

fn refresh_recurring_detection(state: &mut PlannerState) {
    normalize_transaction_categories(&mut state.transactions);
    state.recurring_candidates = detect_recurring_candidates(&state.transactions);
    sync_detected_bills(state);
    let transactions = std::mem::take(&mut state.transactions);
    state.transactions = apply_recurring_candidates(
        transactions,
        &state.recurring_candidates,
        &state.bills,
        &state.paychecks,
    );
}

fn frequency_from_cadence(cadence: RecurringCadence) -> Option<Frequency> {
    match cadence {
        RecurringCadence::Biweekly => Some(Frequency::Biweekly),
        RecurringCadence::Semimonthly => Some(Frequency::Semimonthly),
        RecurringCadence::Monthly => Some(Frequency::Monthly),
        RecurringCadence::Quarterly => Some(Frequency::Quarterly),
        RecurringCadence::Yearly => Some(Frequency::Yearly),
        RecurringCadence::Weekly | RecurringCadence::Irregular => None,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ActualBalancePoint {
    day: i64,
    balance: f64,
    inflow: f64,
    outflow: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct ActualDayTotals {
    net: f64,
    inflow: f64,
    outflow: f64,
}

fn actual_balance_points(
    transactions: &[TrackedTransaction],
    current_balance: f64,
    current_day: i64,
) -> Vec<ActualBalancePoint> {
    let mut daily_totals = std::collections::BTreeMap::<i64, ActualDayTotals>::new();

    for transaction in transactions {
        let Some(day) = parse_iso_date_days(&transaction.date) else {
            continue;
        };
        let totals = daily_totals.entry(day).or_default();
        totals.net += transaction.amount;
        if transaction.amount >= 0.0 {
            totals.inflow += transaction.amount;
        } else {
            totals.outflow += transaction.amount.abs();
        }
    }

    let mut running_balance = current_balance;
    let mut points = daily_totals
        .iter()
        .rev()
        .map(|(day, totals)| {
            let balance_after_day = running_balance;
            running_balance -= totals.net;
            ActualBalancePoint {
                day: *day,
                balance: balance_after_day,
                inflow: totals.inflow,
                outflow: totals.outflow,
            }
        })
        .collect::<Vec<_>>();
    points.reverse();

    let earliest_visible_day = current_day - 365;
    points.retain(|point| point.day >= earliest_visible_day && point.day <= current_day);

    if !points.is_empty() && points.last().map(|point| point.day) != Some(current_day) {
        points.push(ActualBalancePoint {
            day: current_day,
            balance: current_balance,
            inflow: 0.0,
            outflow: 0.0,
        });
    }

    points
}

fn chart_svg(
    forecast: &Forecast,
    floor: f64,
    current_balance: f64,
    transactions: &[TrackedTransaction],
) -> impl IntoView {
    let width = 1120.0;
    let height = 420.0;
    let chart_left = 64.0;
    let chart_right = width - 24.0;
    let top = 34.0;
    let bottom = 56.0;
    let baseline = height - bottom;
    let forecast_start = forecast
        .daily
        .first()
        .map(|point| point.date)
        .unwrap_or(Date {
            year: 0,
            month: 1,
            day: 1,
        });
    let forecast_end = forecast
        .daily
        .last()
        .map(|point| point.date)
        .unwrap_or(forecast_start);
    let today_day = date_days(forecast_start);
    let chart_start_day = today_day - 365;
    let chart_end_day = date_days(forecast_end).max(today_day + 1);
    let chart_day_range = (chart_end_day - chart_start_day).max(1);
    let x_for_day = |day: i64| {
        chart_left
            + (day - chart_start_day) as f64 / chart_day_range as f64 * (chart_right - chart_left)
    };
    let today_x = x_for_day(today_day);
    let actual_points = actual_balance_points(transactions, current_balance, today_day);

    let min_actual_balance = actual_points
        .iter()
        .map(|point| point.balance)
        .fold(current_balance, f64::min);
    let max_actual_balance = actual_points
        .iter()
        .map(|point| point.balance)
        .fold(current_balance, f64::max);
    let min_forecast_balance = forecast
        .daily
        .iter()
        .map(|point| point.balance)
        .fold(0.0, f64::min);
    let max_forecast_balance = forecast
        .daily
        .iter()
        .map(|point| point.balance)
        .fold(floor, f64::max);
    let min_balance = min_actual_balance.min(min_forecast_balance).min(0.0);
    let max_balance = max_actual_balance.max(max_forecast_balance).max(floor);
    let balance_range = (max_balance - min_balance).max(1.0);

    let y = |value: f64| top + (max_balance - value) / balance_range * (height - top - bottom);
    let actual_path = actual_points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            format!(
                "{} {:.1} {:.1}",
                if index == 0 { "M" } else { "L" },
                x_for_day(point.day),
                y(point.balance)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut forecast_path_parts = vec![format!(
        "M {:.1} {:.1}",
        x_for_day(today_day),
        y(current_balance)
    )];
    forecast_path_parts.extend(forecast.daily.iter().map(|point| {
        format!(
            "L {:.1} {:.1}",
            x_for_day(date_days(point.date)),
            y(point.balance)
        )
    }));
    let forecast_path = forecast_path_parts.join(" ");
    let max_activity = actual_points
        .iter()
        .map(|point| point.inflow.max(point.outflow))
        .chain(
            forecast
                .daily
                .iter()
                .map(|point| point.inflow.max(point.outflow)),
        )
        .fold(1.0, f64::max);
    let activity_height = |value: f64| (value / max_activity * 74.0).min(74.0);
    let actual_bar_width = ((today_x - chart_left) / actual_points.len().max(1) as f64)
        .abs()
        .clamp(2.0, 8.0);
    let forecast_bar_width =
        ((chart_right - today_x) / forecast.daily.len().max(1) as f64).clamp(1.0, 4.0);
    let past_year_label = (forecast_start.year - 1).to_string();
    let today_label = format!("{} {}", t("Today"), forecast_start.year);
    let forecast_year_label = forecast_end.year.to_string();
    let low_point_day = date_days(forecast.low_point.date);
    let low_point_x = x_for_day(low_point_day);
    let low_point_y = y(forecast.low_point.balance);
    let low_point_label = format!(
        "{}: {} {} {}",
        t("Lowest projected balance"),
        money(forecast.low_point.balance),
        t("on"),
        forecast.low_point.date.label()
    );
    let tooltip_width = 236.0;
    let tooltip_x = (low_point_x - tooltip_width / 2.0)
        .max(chart_left)
        .min(chart_right - tooltip_width);
    let tooltip_y = if low_point_y < top + 76.0 {
        low_point_y + 18.0
    } else {
        low_point_y - 66.0
    };

    view! {
        <div class="chart-frame" data-testid="balance-chart">
            <svg viewBox="0 0 1120 420" preserveAspectRatio="none">
                <rect width="1120" height="420" fill="#fbfcfc"></rect>
                <rect x=chart_left y=top width={today_x - chart_left} height={height - top - bottom} fill="#ffffff" opacity="0.62"></rect>
                <rect x=today_x y=top width={chart_right - today_x} height={height - top - bottom} fill="#ffffff" opacity="0.42"></rect>
                <line x1=today_x x2=today_x y1=top y2=baseline stroke="#9aa7a3" stroke-width="1.5" stroke-dasharray="5 6"></line>
                <line x1=chart_left x2=chart_right y1={y(0.0)} y2={y(0.0)} stroke="#bd3d2a" stroke-dasharray="5 5" opacity="0.45"></line>
                <line x1=chart_left x2=chart_right y1={y(floor)} y2={y(floor)} stroke="#087f7a" stroke-dasharray="4 6" opacity="0.55"></line>
                <line x1=chart_left x2=chart_right y1=baseline y2=baseline stroke="#cfd8d5"></line>
                {actual_points.iter().map(|point| {
                    let x = x_for_day(point.day);
                    let inflow_height = activity_height(point.inflow);
                    let outflow_height = activity_height(point.outflow);
                    view! {
                        <>
                            <rect x={x - actual_bar_width / 2.0} y={baseline - inflow_height} width=actual_bar_width height=inflow_height rx="2" fill="#3066be" opacity="0.32"></rect>
                            <rect x={x - actual_bar_width / 2.0} y={baseline - outflow_height} width=actual_bar_width height=outflow_height rx="2" fill="#c78022" opacity="0.42"></rect>
                        </>
                    }
                }).collect_view()}
                {forecast.daily.iter().filter(|point| point.inflow > 0.0 || point.outflow > 0.0).map(|point| {
                    let x = x_for_day(date_days(point.date));
                    let inflow_height = activity_height(point.inflow);
                    let outflow_height = activity_height(point.outflow);
                    view! {
                        <>
                            <rect x={x - forecast_bar_width} y={baseline - inflow_height} width=forecast_bar_width height=inflow_height rx="1.5" fill="#3066be" opacity="0.22"></rect>
                            <rect x=x y={baseline - outflow_height} width=forecast_bar_width height=outflow_height rx="1.5" fill="#c78022" opacity="0.28"></rect>
                        </>
                    }
                }).collect_view()}
                {if actual_path.is_empty() {
                    view! { <text x={chart_left + 14.0} y={top + 32.0} fill="#68717a" font-size="13">{t("No imported transactions in the past year")}</text> }.into_any()
                } else {
                    view! { <path d=actual_path fill="none" stroke="#4f6f52" stroke-width="3" vector-effect="non-scaling-stroke"></path> }.into_any()
                }}
                <path d=forecast_path fill="none" stroke="#087f7a" stroke-width="3" vector-effect="non-scaling-stroke"></path>
                <g class="low-point-marker" tabindex="0" role="img" aria-label=low_point_label.clone()>
                    <circle class="low-point-hit" cx=low_point_x cy=low_point_y r="18"></circle>
                    <circle cx=low_point_x cy=low_point_y r="6" fill="#bd3d2a">
                        <title>{low_point_label.clone()}</title>
                    </circle>
                    <g class="chart-tooltip">
                        <rect x=tooltip_x y=tooltip_y width=tooltip_width height="48" rx="7"></rect>
                        <text x={tooltip_x + 12.0} y={tooltip_y + 19.0}>{t("Lowest projected balance")}</text>
                        <text class="chart-tooltip-value" x={tooltip_x + 12.0} y={tooltip_y + 37.0}>
                            {format!("{} {} {}", money(forecast.low_point.balance), t("on"), forecast.low_point.date.label())}
                        </text>
                    </g>
                </g>
                <text x=chart_left y="20" fill="#1b1f23" font-size="13" font-weight="700">{t("Past 12 months")}</text>
                <text x={today_x + 12.0} y="20" fill="#1b1f23" font-size="13" font-weight="700">{t("Forecast")}</text>
                <text x="14" y={y(max_balance)} fill="#68717a" font-size="12">{money(max_balance)}</text>
                <text x="14" y={y(min_balance)} fill="#68717a" font-size="12">{money(min_balance)}</text>
                <text x={chart_right - 8.0} y={y(floor) - 7.0} text-anchor="end" fill="#087f7a" font-size="12">{t("buffer floor")}</text>
                <text x=chart_left y={height - 20.0} fill="#68717a" font-size="12" font-weight="700">{past_year_label}</text>
                <text x=today_x y={height - 20.0} text-anchor="middle" fill="#1b1f23" font-size="12" font-weight="800">{today_label}</text>
                <text x=chart_right y={height - 20.0} text-anchor="end" fill="#68717a" font-size="12" font-weight="700">{forecast_year_label}</text>
            </svg>
        </div>
    }
}

fn date_days(date: Date) -> i64 {
    days_from_civil(date.year, date.month, date.day)
}

fn money(value: f64) -> String {
    format!("${:.2}", round_cents(value))
}

fn signed_money(value: f64) -> String {
    if value < 0.0 {
        format!("-{}", money(value.abs()))
    } else {
        money(value)
    }
}

fn money_input(value: f64) -> String {
    format!("{:.2}", round_cents(value))
}

fn parse_money(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let compact = trimmed
        .chars()
        .filter(|character| !matches!(character, '$' | ' ' | '\u{00a0}' | '_'))
        .collect::<String>();

    let last_comma = compact.rfind(',');
    let last_dot = compact.rfind('.');
    let normalized = match (last_comma, last_dot) {
        (Some(comma), Some(dot)) if comma > dot => compact.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => compact.replace(',', ""),
        (Some(_), None) => compact.replace(',', "."),
        (None, Some(_)) => compact,
        (None, None) => compact,
    };

    normalized.parse::<f64>().ok().map(round_cents)
}

fn round_cents(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn frequency_value(frequency: Frequency) -> &'static str {
    match frequency {
        Frequency::Biweekly => "biweekly",
        Frequency::Semimonthly => "semimonthly",
        Frequency::Monthly => "monthly",
        Frequency::Quarterly => "quarterly",
        Frequency::Yearly => "yearly",
    }
}

fn parse_frequency(value: &str) -> Frequency {
    match value {
        "biweekly" => Frequency::Biweekly,
        "semimonthly" => Frequency::Semimonthly,
        "quarterly" => Frequency::Quarterly,
        "yearly" => Frequency::Yearly,
        _ => Frequency::Monthly,
    }
}

fn paycheck_pressure_value(state: &PlannerState, transfer: f64) -> String {
    let paycheck = state.settings.paycheck_amount;
    if paycheck <= 0.0 {
        return t("Set paycheck").to_string();
    }

    format!("{:.1}%", (transfer / paycheck) * 100.0)
}

fn paycheck_pressure_note(state: &PlannerState, transfer: f64) -> String {
    let paycheck = state.settings.paycheck_amount;
    if paycheck <= 0.0 {
        return t("Add paycheck amount").to_string();
    }

    if transfer > paycheck {
        format!("{} {}", t("Short by"), money(transfer - paycheck))
    } else if transfer > paycheck * 0.8 {
        t("Very high transfer").to_string()
    } else {
        format!(
            "{} {}",
            money(paycheck - transfer),
            t("left after transfer")
        )
    }
}

fn recommended_transfer_value(forecast: &Forecast, state: &PlannerState, transfer: f64) -> String {
    money(transfer + shortfall_add_on_per_paycheck(forecast, state).unwrap_or(0.0))
}

fn recommended_transfer_note(forecast: &Forecast, state: &PlannerState) -> String {
    let Some(add_on) = shortfall_add_on_per_paycheck(forecast, state) else {
        return t("Next paycheck").to_string();
    };

    format!("+{} {}", money(add_on), t("to stay afloat"))
}

fn shortfall_add_on_per_paycheck(forecast: &Forecast, state: &PlannerState) -> Option<f64> {
    if forecast.low_point.balance >= 0.0 {
        return None;
    }

    let start = forecast.daily.first()?.date;
    let low_date = forecast.low_point.date;
    let paychecks = paydays_before(start, low_date);
    if paychecks == 0 {
        return None;
    }

    let deficit = forecast.low_point.balance.abs();
    let required = state.settings.minimum_buffer + deficit;
    let with_margin = required * (1.0 + state.settings.margin_percent / 100.0);
    Some(round_cents(with_margin / paychecks as f64))
}

fn paydays_before(mut date: Date, end: Date) -> usize {
    if date >= end {
        return 0;
    }

    let mut count = 0;
    while date < end {
        if is_payday(date) {
            count += 1;
        }
        date = date.next_day();
    }
    count
}

fn is_payday(date: Date) -> bool {
    date.day == 15 || date.day == days_in_month(date.year, date.month).min(30)
}

fn account_matches_for_ui(found: &str, target: &str) -> bool {
    let found = normalize_name_for_ui(found);
    let target = normalize_name_for_ui(target);
    found == target || found.contains(&target) || target.contains(&found)
}

fn normalize_name_for_ui(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn run_ynab_import(state: RwSignal<PlannerState>, is_importing: RwSignal<bool>) {
    if state.get().ynab.access_token.trim().is_empty() {
        state.update(|state| {
            state.ynab.last_import_status = "Add a YNAB personal access token first".to_string();
        });
        return;
    }

    is_importing.set(true);

    leptos::task::spawn_local(async move {
        let snapshot = state.get_untracked();
        match import_ynab(snapshot).await {
            Ok(imported) => {
                state.update(|state| {
                    state.settings.starting_balance = imported.account_balance;
                    state.ynab.plan_id = Some(imported.plan_id);
                    state.ynab.account_id = Some(imported.account_id);
                    state.ynab.last_import_status = format!(
                        "Imported {} transactions from {}",
                        imported.transactions.len(),
                        imported.account_name
                    );
                    state.ynab.last_imported_at = Some(now_label());
                    let imported_transactions =
                        merge_transaction_corrections(imported.transactions, &state.transactions);
                    state.recurring_candidates =
                        detect_recurring_candidates(&imported_transactions);
                    sync_detected_bills(state);
                    state.transactions = apply_recurring_candidates(
                        imported_transactions,
                        &state.recurring_candidates,
                        &state.bills,
                        &state.paychecks,
                    );
                });
            }
            Err(error) => {
                state.update(|state| {
                    state.ynab.last_import_status = error;
                });
            }
        }
        is_importing.set(false);
    });
}

fn run_ynab_choice_load(state: RwSignal<PlannerState>, is_importing: RwSignal<bool>) {
    if state.get().ynab.access_token.trim().is_empty() {
        state.update(|state| {
            state.ynab.last_import_status = "Add a YNAB personal access token first".to_string();
        });
        return;
    }

    is_importing.set(true);

    leptos::task::spawn_local(async move {
        let snapshot = state.get_untracked();
        match load_ynab_choices(snapshot).await {
            Ok(choices) => {
                state.update(|state| {
                    state.ynab.available_plans = choices.plans;
                    state.ynab.available_accounts = choices.accounts;
                    state.ynab.plan_id = choices.plan_id;
                    state.ynab.account_id = choices.account_id;
                    if let Some(account_name) = choices.account_name {
                        state.ynab.account_name = account_name;
                    }
                    state.ynab.last_import_status = format!(
                        "Loaded {} budgets and {} accounts",
                        state.ynab.available_plans.len(),
                        state.ynab.available_accounts.len()
                    );
                });
            }
            Err(error) => {
                state.update(|state| {
                    state.ynab.last_import_status = error;
                });
            }
        }
        is_importing.set(false);
    });
}

fn run_ynab_account_load(state: RwSignal<PlannerState>, is_importing: RwSignal<bool>) {
    if state.get().ynab.access_token.trim().is_empty() || state.get().ynab.plan_id.is_none() {
        return;
    }

    is_importing.set(true);

    leptos::task::spawn_local(async move {
        let snapshot = state.get_untracked();
        match load_ynab_accounts(snapshot).await {
            Ok(accounts) => {
                state.update(|state| {
                    state.ynab.available_accounts = accounts;
                    let selected_account = state
                        .ynab
                        .account_id
                        .as_ref()
                        .and_then(|id| {
                            state
                                .ynab
                                .available_accounts
                                .iter()
                                .find(|account| account.id == *id)
                        })
                        .or_else(|| {
                            state.ynab.available_accounts.iter().find(|account| {
                                account_matches_for_ui(&account.name, &state.ynab.account_name)
                            })
                        });
                    if let Some(account) = selected_account {
                        state.ynab.account_id = Some(account.id.clone());
                        state.ynab.account_name = account.name.clone();
                    }
                    state.ynab.last_import_status =
                        format!("Loaded {} accounts", state.ynab.available_accounts.len());
                });
            }
            Err(error) => {
                state.update(|state| {
                    state.ynab.last_import_status = error;
                });
            }
        }
        is_importing.set(false);
    });
}

#[cfg(target_arch = "wasm32")]
async fn import_ynab(state: PlannerState) -> Result<YnabImport, String> {
    let token = state.ynab.access_token.trim().to_string();
    let plans: PlansResponse = ynab_get(&token, "https://api.ynab.com/v1/plans").await?;
    if plans.data.plans.is_empty() {
        return Err("YNAB returned no plans for this token".to_string());
    }

    let mut available_accounts = Vec::new();
    let preferred_plan_id = state.ynab.plan_id.as_deref();
    let mut plans_to_search = plans.data.plans;
    plans_to_search.sort_by_key(|plan| {
        if preferred_plan_id.is_some_and(|id| id == plan.id) {
            0
        } else {
            1
        }
    });

    for plan in plans_to_search {
        let accounts_url = format!("https://api.ynab.com/v1/plans/{}/accounts", plan.id);
        let accounts: AccountsResponse = ynab_get(&token, &accounts_url).await?;
        available_accounts.extend(
            accounts
                .data
                .accounts
                .iter()
                .map(|account| format!("{} / {}", plan.name, account.name)),
        );

        if let Some(account) = accounts.data.accounts.into_iter().find(|account| {
            state
                .ynab
                .account_id
                .as_deref()
                .is_some_and(|id| id == account.id)
                || account_matches(&account.name, &state.ynab.account_name)
        }) {
            let transactions_url = format!(
                "https://api.ynab.com/v1/plans/{}/accounts/{}/transactions",
                plan.id, account.id
            );
            let transactions: TransactionsResponse = ynab_get(&token, &transactions_url).await?;
            let tracked = transactions
                .data
                .transactions
                .into_iter()
                .filter(|transaction| !transaction.deleted)
                .map(|transaction| classify_transaction(transaction, &state.bills))
                .collect();

            return Ok(YnabImport {
                plan_id: plan.id,
                account_id: account.id,
                account_name: account.name,
                account_balance: milliunits_to_units(account.balance),
                transactions: tracked,
            });
        }
    }

    let examples = available_accounts
        .into_iter()
        .take(8)
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!(
        "Could not find '{}'. Found accounts: {}",
        state.ynab.account_name,
        if examples.is_empty() {
            "none".to_string()
        } else {
            examples
        }
    ))
}

#[cfg(not(target_arch = "wasm32"))]
async fn import_ynab(_state: PlannerState) -> Result<YnabImport, String> {
    Err("YNAB import is available in the browser build".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn load_ynab_choices(state: PlannerState) -> Result<YnabChoices, String> {
    let token = state.ynab.access_token.trim().to_string();
    let plans: PlansResponse = ynab_get(&token, "https://api.ynab.com/v1/plans").await?;
    let plan_choices = plans
        .data
        .plans
        .iter()
        .map(|plan| YnabChoice {
            id: plan.id.clone(),
            name: plan.name.clone(),
        })
        .collect::<Vec<_>>();
    let selected_plan_id = state
        .ynab
        .plan_id
        .filter(|id| plan_choices.iter().any(|plan| plan.id == *id))
        .or_else(|| plan_choices.first().map(|plan| plan.id.clone()));

    let accounts = if selected_plan_id.is_some() {
        load_ynab_accounts_for_plan(&token, selected_plan_id.as_deref().unwrap()).await?
    } else {
        Vec::new()
    };
    let selected_account = state
        .ynab
        .account_id
        .and_then(|id| accounts.iter().find(|account| account.id == id).cloned())
        .or_else(|| {
            accounts
                .iter()
                .find(|account| account_matches(&account.name, &state.ynab.account_name))
                .cloned()
        });

    Ok(YnabChoices {
        plans: plan_choices,
        accounts,
        plan_id: selected_plan_id,
        account_id: selected_account.as_ref().map(|account| account.id.clone()),
        account_name: selected_account.map(|account| account.name),
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_ynab_choices(_state: PlannerState) -> Result<YnabChoices, String> {
    Err("YNAB import is available in the browser build".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn load_ynab_accounts(state: PlannerState) -> Result<Vec<YnabChoice>, String> {
    let token = state.ynab.access_token.trim().to_string();
    let plan_id = state
        .ynab
        .plan_id
        .ok_or_else(|| "Select a YNAB budget first".to_string())?;
    load_ynab_accounts_for_plan(&token, &plan_id).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_ynab_accounts(_state: PlannerState) -> Result<Vec<YnabChoice>, String> {
    Err("YNAB import is available in the browser build".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn load_ynab_accounts_for_plan(
    token: &str,
    plan_id: &str,
) -> Result<Vec<YnabChoice>, String> {
    let accounts_url = format!("https://api.ynab.com/v1/plans/{plan_id}/accounts");
    let accounts: AccountsResponse = ynab_get(token, &accounts_url).await?;
    Ok(accounts
        .data
        .accounts
        .into_iter()
        .map(|account| YnabChoice {
            id: account.id,
            name: account.name,
        })
        .collect())
}

#[cfg(target_arch = "wasm32")]
async fn ynab_get<T>(token: &str, url: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = gloo_net::http::Request::get(url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|error| format!("YNAB request failed: {error}"))?;

    if !response.ok() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("YNAB returned HTTP {status}: {text}"));
    }

    response
        .json::<T>()
        .await
        .map_err(|error| format!("Could not read YNAB response: {error}"))
}

#[cfg(any(target_arch = "wasm32", test))]
fn classify_transaction(transaction: YnabTransaction, bills: &[Bill]) -> TrackedTransaction {
    let amount = milliunits_to_units(transaction.amount);
    let payee_name = transaction
        .payee_name
        .unwrap_or_else(|| "Unknown payee".to_string());
    let category_name = normalize_category_name(&transaction.category_name.unwrap_or_default());
    let memo = transaction.memo.unwrap_or_default();

    if amount > 0.0 {
        return TrackedTransaction {
            id: transaction.id,
            date: transaction.date,
            payee_name,
            category_name,
            memo,
            amount,
            classification: TransactionClass::Transfer,
            matched_bill_id: None,
            manual_classification: None,
        };
    }

    let matched_bill = bills.iter().find(|bill| {
        if !is_assignable_bill(bill) {
            return false;
        }
        let payee = payee_name.to_lowercase();
        let name = bill.name.to_lowercase();
        let amount_delta = (amount.abs() - bill.amount).abs();
        payee.contains(&name) || amount_delta <= (bill.amount * 0.08).max(5.0)
    });

    TrackedTransaction {
        id: transaction.id,
        date: transaction.date,
        payee_name,
        category_name,
        memo,
        amount,
        classification: if matched_bill.is_some() {
            TransactionClass::Recurring
        } else {
            TransactionClass::Misc
        },
        matched_bill_id: matched_bill.map(|bill| bill.id),
        manual_classification: None,
    }
}

fn merge_transaction_corrections(
    mut imported: Vec<TrackedTransaction>,
    existing: &[TrackedTransaction],
) -> Vec<TrackedTransaction> {
    for transaction in &mut imported {
        if let Some(previous) = existing
            .iter()
            .find(|previous| previous.id == transaction.id)
        {
            transaction.category_name = previous.category_name.clone();
            transaction.classification = previous.classification;
            transaction.matched_bill_id = previous.matched_bill_id;
            transaction.manual_classification = previous.manual_classification;
        }
        transaction.category_name = normalize_category_name(&transaction.category_name);
    }
    imported
}

#[cfg(target_arch = "wasm32")]
fn account_matches(found: &str, target: &str) -> bool {
    account_matches_for_ui(found, target)
}

#[cfg(any(target_arch = "wasm32", test))]
fn milliunits_to_units(amount: i64) -> f64 {
    amount as f64 / 1000.0
}

fn class_total(state: &PlannerState, class: TransactionClass) -> f64 {
    state
        .transactions
        .iter()
        .filter(|transaction| transaction.classification == class)
        .map(|transaction| transaction.amount)
        .sum()
}

fn class_count(state: &PlannerState, class: TransactionClass) -> usize {
    state
        .transactions
        .iter()
        .filter(|transaction| transaction.classification == class)
        .count()
}

fn detect_recurring_candidates(transactions: &[TrackedTransaction]) -> Vec<RecurringCandidate> {
    let mut groups: Vec<(String, Vec<&TrackedTransaction>)> = Vec::new();

    for transaction in transactions
        .iter()
        .filter(|transaction| transaction.amount != 0.0)
        .filter(|transaction| {
            normalize_category_name(&transaction.category_name) != NON_RECURRING_CATEGORY
        })
    {
        let key = recurring_key(transaction);
        if let Some((_, items)) = groups.iter_mut().find(|(existing, _)| existing == &key) {
            items.push(transaction);
        } else {
            groups.push((key, vec![transaction]));
        }
    }

    let mut candidates = groups
        .into_iter()
        .filter_map(|(_, mut items)| {
            items.sort_by(|left, right| left.date.cmp(&right.date));
            build_recurring_candidate(&items)
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .confidence
            .total_cmp(&left.confidence)
            .then_with(|| right.occurrence_count.cmp(&left.occurrence_count))
            .then_with(|| right.last_date.cmp(&left.last_date))
    });
    candidates
}

fn build_recurring_candidate(items: &[&TrackedTransaction]) -> Option<RecurringCandidate> {
    if items.len() < 2 {
        return None;
    }

    let mut dates = Vec::new();
    let mut gaps = Vec::new();
    for window in items.windows(2) {
        let Some(left) = parse_iso_date_days(&window[0].date) else {
            continue;
        };
        let Some(right) = parse_iso_date_days(&window[1].date) else {
            continue;
        };
        let gap = right - left;
        if gap > 0 {
            gaps.push(gap);
        }
    }

    for item in items {
        if let Some(date) = parse_iso_date(&item.date) {
            dates.push(date);
        }
    }

    let cadence = infer_cadence(&gaps, &dates)?;
    let amount_sum = items
        .iter()
        .map(|transaction| transaction.amount)
        .sum::<f64>();
    let average_amount = amount_sum / items.len() as f64;
    let last = items.last()?;
    let amount_variance = items
        .iter()
        .map(|transaction| (transaction.amount - average_amount).abs())
        .sum::<f64>()
        / items.len() as f64;
    let amount_stability = 1.0 - (amount_variance / average_amount.abs().max(1.0)).min(1.0);
    let cadence_confidence = cadence_confidence(cadence, &gaps);
    let has_real_category =
        normalize_category_name(&items[0].category_name) != DEFAULT_CATEGORY_NAME;
    let confidence = if has_real_category {
        ((cadence_confidence * 0.9) + (amount_stability * 0.1)).clamp(0.0, 1.0)
    } else {
        ((cadence_confidence * 0.7) + (amount_stability * 0.3)).clamp(0.0, 1.0)
    };

    let minimum_confidence = if has_real_category { 0.35 } else { 0.45 };
    if confidence < minimum_confidence {
        return None;
    }

    Some(RecurringCandidate {
        payee_name: items[0].payee_name.clone(),
        category_name: normalize_category_name(&items[0].category_name),
        cadence,
        average_amount,
        last_amount: last.amount,
        occurrence_count: items.len(),
        last_date: last.date.clone(),
        confidence,
    })
}

fn apply_recurring_candidates(
    mut transactions: Vec<TrackedTransaction>,
    candidates: &[RecurringCandidate],
    bills: &[Bill],
    paychecks: &[Bill],
) -> Vec<TrackedTransaction> {
    for transaction in &mut transactions {
        transaction.category_name = normalize_category_name(&transaction.category_name);
        if transaction.manual_classification.is_some() {
            continue;
        }

        if let Some(bill_id) = transaction.matched_bill_id {
            if transaction.classification == TransactionClass::Paycheck {
                if let Some(paycheck) = paychecks.iter().find(|paycheck| paycheck.id == bill_id) {
                    if is_assignable_bill(paycheck) {
                        transaction.category_name = paycheck.name.clone();
                    } else {
                        transaction.matched_bill_id = None;
                    }
                }
            } else if let Some(bill) = bills.iter().find(|bill| bill.id == bill_id) {
                if is_assignable_bill(bill) {
                    transaction.classification = TransactionClass::Recurring;
                    transaction.category_name = bill.name.clone();
                } else {
                    transaction.matched_bill_id = None;
                }
            }
        }

        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate_key(candidate) == recurring_key(transaction))
        {
            if transaction.amount > 0.0 {
                transaction.classification = TransactionClass::Paycheck;
                transaction.matched_bill_id = paycheck_id_for_candidate(candidate, paychecks);
                if let Some(paycheck_id) = transaction.matched_bill_id {
                    if let Some(paycheck) =
                        paychecks.iter().find(|paycheck| paycheck.id == paycheck_id)
                    {
                        if is_assignable_bill(paycheck) {
                            transaction.category_name = paycheck.name.clone();
                        } else {
                            transaction.matched_bill_id = None;
                        }
                    }
                }
            } else if transaction.amount < 0.0 {
                transaction.classification = TransactionClass::Recurring;
                transaction.matched_bill_id = bill_id_for_candidate(candidate, bills);
                if let Some(bill_id) = transaction.matched_bill_id {
                    if let Some(bill) = bills.iter().find(|bill| bill.id == bill_id) {
                        if is_assignable_bill(bill) {
                            transaction.category_name = bill.name.clone();
                        } else {
                            transaction.matched_bill_id = None;
                        }
                    } else {
                        transaction.matched_bill_id = None;
                    }
                }
            }
        }
    }

    transactions
}

fn paycheck_id_for_candidate(candidate: &RecurringCandidate, paychecks: &[Bill]) -> Option<u32> {
    let name = paycheck_name_from_candidate(candidate);
    paychecks
        .iter()
        .find(|paycheck| {
            is_assignable_bill(paycheck)
                && normalize_name_for_ui(&paycheck.name) == normalize_name_for_ui(&name)
        })
        .map(|paycheck| paycheck.id)
}

fn bill_id_for_candidate(candidate: &RecurringCandidate, bills: &[Bill]) -> Option<u32> {
    let name = bill_name_from_candidate(candidate);
    bills
        .iter()
        .find(|bill| {
            is_assignable_bill(bill)
                && normalize_name_for_ui(&bill.name) == normalize_name_for_ui(&name)
        })
        .map(|bill| bill.id)
}

fn recurring_key(transaction: &TrackedTransaction) -> String {
    let category = normalize_category_name(&transaction.category_name);
    if category != DEFAULT_CATEGORY_NAME {
        return normalize_recurring_key(&format!("category:{category}"));
    }

    normalize_recurring_key(&format!("payee:{}", transaction.payee_name))
}

fn candidate_key(candidate: &RecurringCandidate) -> String {
    let category = normalize_category_name(&candidate.category_name);
    if category != DEFAULT_CATEGORY_NAME {
        return normalize_recurring_key(&format!("category:{category}"));
    }

    normalize_recurring_key(&format!("payee:{}", candidate.payee_name))
}

fn normalize_recurring_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn infer_cadence(gaps: &[i64], dates: &[SimpleDate]) -> Option<RecurringCadence> {
    if gaps.is_empty() {
        return None;
    }

    let average = gaps.iter().sum::<i64>() as f64 / gaps.len() as f64;
    let cadence = if is_semimonthly_pattern(dates) {
        RecurringCadence::Semimonthly
    } else if within(average, 7.0, 2.0) {
        RecurringCadence::Weekly
    } else if within(average, 14.0, 3.0) {
        RecurringCadence::Biweekly
    } else if let Some(calendar_cadence) = infer_calendar_cadence(dates) {
        calendar_cadence
    } else if within(average, 30.5, 20.0) {
        RecurringCadence::Monthly
    } else if within(average, 91.0, 35.0) {
        RecurringCadence::Quarterly
    } else if within(average, 365.0, 95.0) {
        RecurringCadence::Yearly
    } else {
        RecurringCadence::Irregular
    };

    (cadence != RecurringCadence::Irregular || gaps.len() >= 3).then_some(cadence)
}

fn is_semimonthly_pattern(dates: &[SimpleDate]) -> bool {
    if dates.len() < 4 {
        return false;
    }

    let matching_days = dates
        .iter()
        .filter(|date| {
            let month_end_day = 30.min(days_in_month(date.year, date.month));
            (date.day as i32 - 15).abs() <= 2 || (date.day as i32 - month_end_day as i32).abs() <= 2
        })
        .count();
    let mut month_counts = std::collections::BTreeMap::<(i32, u32), usize>::new();
    for date in dates {
        *month_counts.entry((date.year, date.month)).or_default() += 1;
    }
    let months_with_two_paychecks = month_counts.values().filter(|count| **count >= 2).count();

    matching_days as f64 / dates.len() as f64 >= 0.8 && months_with_two_paychecks >= 2
}

fn infer_calendar_cadence(dates: &[SimpleDate]) -> Option<RecurringCadence> {
    if dates.len() < 2 {
        return None;
    }

    let mut month_indices = dates
        .iter()
        .map(|date| date.year as i64 * 12 + date.month as i64)
        .collect::<Vec<_>>();
    month_indices.sort_unstable();
    month_indices.dedup();

    if month_indices.len() < 2 {
        return None;
    }

    let month_gaps = month_indices
        .windows(2)
        .map(|window| window[1] - window[0])
        .collect::<Vec<_>>();
    let average_month_gap = month_gaps.iter().sum::<i64>() as f64 / month_gaps.len().max(1) as f64;

    if within(average_month_gap, 1.0, 0.25) && mostly_within_month_gap(&month_gaps, 1, 0) {
        Some(RecurringCadence::Monthly)
    } else if within(average_month_gap, 3.0, 1.0) && mostly_within_month_gap(&month_gaps, 3, 1) {
        Some(RecurringCadence::Quarterly)
    } else if within(average_month_gap, 12.0, 4.0) && mostly_within_month_gap(&month_gaps, 12, 4) {
        Some(RecurringCadence::Yearly)
    } else {
        None
    }
}

fn mostly_within_month_gap(gaps: &[i64], target: i64, tolerance: i64) -> bool {
    let matching = gaps
        .iter()
        .filter(|gap| (**gap - target).abs() <= tolerance)
        .count();
    matching as f64 / gaps.len().max(1) as f64 >= 0.75
}

fn cadence_confidence(cadence: RecurringCadence, gaps: &[i64]) -> f64 {
    let Some(target) = cadence_target(cadence) else {
        return 0.45;
    };
    let tolerance = cadence_tolerance(cadence);
    let matching = gaps
        .iter()
        .filter(|gap| within(**gap as f64, target, tolerance))
        .count();
    matching as f64 / gaps.len().max(1) as f64
}

fn cadence_target(cadence: RecurringCadence) -> Option<f64> {
    match cadence {
        RecurringCadence::Weekly => Some(7.0),
        RecurringCadence::Biweekly => Some(14.0),
        RecurringCadence::Semimonthly => Some(15.2),
        RecurringCadence::Monthly => Some(30.5),
        RecurringCadence::Quarterly => Some(91.0),
        RecurringCadence::Yearly => Some(365.0),
        RecurringCadence::Irregular => None,
    }
}

fn cadence_tolerance(cadence: RecurringCadence) -> f64 {
    match cadence {
        RecurringCadence::Weekly => 2.0,
        RecurringCadence::Biweekly => 3.0,
        RecurringCadence::Semimonthly => 2.0,
        RecurringCadence::Monthly => 30.0,
        RecurringCadence::Quarterly => 35.0,
        RecurringCadence::Yearly => 95.0,
        RecurringCadence::Irregular => 0.0,
    }
}

fn within(value: f64, target: f64, tolerance: f64) -> bool {
    (value - target).abs() <= tolerance
}

fn parse_iso_date_days(value: &str) -> Option<i64> {
    let date = parse_iso_date(value)?;
    Some(days_from_civil(date.year, date.month, date.day))
}

fn parse_iso_date(value: &str) -> Option<SimpleDate> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    Some(SimpleDate { year, month, day })
}

#[derive(Clone, Copy)]
struct SimpleDate {
    year: i32,
    month: u32,
    day: u32,
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - (month <= 2) as i32;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i64
}

fn now_label() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::new_0()
            .to_locale_string("en-CA", &wasm_bindgen::JsValue::UNDEFINED)
            .into()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        "Imported".to_string()
    }
}

struct YnabImport {
    plan_id: String,
    account_id: String,
    account_name: String,
    account_balance: f64,
    transactions: Vec<TrackedTransaction>,
}

struct YnabChoices {
    plans: Vec<YnabChoice>,
    accounts: Vec<YnabChoice>,
    plan_id: Option<String>,
    account_id: Option<String>,
    account_name: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct PlansResponse {
    data: PlansData,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct PlansData {
    plans: Vec<YnabPlan>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize, Clone)]
struct YnabPlan {
    id: String,
    name: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct AccountsResponse {
    data: AccountsData,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct AccountsData {
    accounts: Vec<YnabAccount>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct YnabAccount {
    id: String,
    name: String,
    balance: i64,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct TransactionsResponse {
    data: TransactionsData,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct TransactionsData {
    transactions: Vec<YnabTransaction>,
}

#[derive(Deserialize)]
struct YnabTransaction {
    id: String,
    date: String,
    amount: i64,
    payee_name: Option<String>,
    category_name: Option<String>,
    memo: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    deleted: bool,
}

fn load_planner_state() -> Option<PlannerState> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()?.local_storage().ok().flatten()?;
        let raw = storage.get_item(STORAGE_KEY).ok().flatten()?;
        let mut state: PlannerState = serde_json::from_str(&raw).ok()?;
        normalize_transaction_categories(&mut state.transactions);
        refresh_recurring_detection(&mut state);
        Some(state)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

fn persist_planner_state(state: &PlannerState) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(Some(storage)) = window.local_storage() else {
            return;
        };
        let Ok(serialized) = serde_json::to_string(state) else {
            return;
        };

        let _ = storage.set_item(STORAGE_KEY, &serialized);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = state;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        actual_balance_points, apply_recurring_candidates, classify_transaction, days_from_civil,
        detect_recurring_candidates, group_transactions_by_category, merge_transaction_corrections,
        next_bill_due_date, next_thursday_after, normalize_category_name, parse_money,
        paydays_before, recommended_transfer_note, recommended_transfer_value,
        refresh_recurring_detection, shortfall_add_on_per_paycheck, sort_transactions_by_date,
        sync_detected_bills, transaction_bill_select_value, transaction_group_amount_label,
        transaction_group_recurring_label, transaction_recurring_label, TransactionSortColumn,
        YnabTransaction,
    };
    use crate::forecast::{DailyPoint, Date, Forecast};
    use crate::models::{
        Bill, Frequency, PlannerState, RecurringCadence, TrackedTransaction, TransactionClass,
        DEFAULT_CATEGORY_NAME,
    };

    #[test]
    fn parses_money_with_cents_and_localized_separators() {
        assert_eq!(parse_money("1072.52"), Some(1072.52));
        assert_eq!(parse_money("1 072,52$"), Some(1072.52));
        assert_eq!(parse_money("1,072.52"), Some(1072.52));
        assert_eq!(parse_money("$1.072,52"), Some(1072.52));
    }

    #[test]
    fn positive_ynab_transaction_is_a_transfer() {
        let transaction = classify_transaction(
            YnabTransaction {
                id: "transfer".to_string(),
                date: "2026-05-02".to_string(),
                amount: 200_000,
                payee_name: Some("Transfer from chequing".to_string()),
                category_name: Some("Inflow: Ready to Assign".to_string()),
                memo: None,
                deleted: false,
            },
            &PlannerState::sample().bills,
        );

        assert_eq!(transaction.amount, 200.0);
        assert_eq!(transaction.classification, TransactionClass::Transfer);
    }

    #[test]
    fn unmatched_outflow_is_misc() {
        let transaction = classify_transaction(
            YnabTransaction {
                id: "misc".to_string(),
                date: "2026-05-02".to_string(),
                amount: -42_520,
                payee_name: Some("Unexpected fee".to_string()),
                category_name: Some("Fees".to_string()),
                memo: None,
                deleted: false,
            },
            &PlannerState::sample().bills,
        );

        assert_eq!(transaction.amount, -42.52);
        assert_eq!(transaction.classification, TransactionClass::Misc);
    }

    #[test]
    fn missing_or_blank_categories_normalize_to_misc() {
        let missing = classify_transaction(
            YnabTransaction {
                id: "missing-category".to_string(),
                date: "2026-05-02".to_string(),
                amount: -10_000,
                payee_name: Some("Unknown".to_string()),
                category_name: None,
                memo: None,
                deleted: false,
            },
            &[],
        );

        assert_eq!(missing.category_name, DEFAULT_CATEGORY_NAME);
        assert_eq!(normalize_category_name("   "), DEFAULT_CATEGORY_NAME);
        assert_eq!(
            normalize_category_name("Uncategorized"),
            DEFAULT_CATEGORY_NAME
        );
    }

    #[test]
    fn similar_amount_outflow_matches_recurring_bill() {
        let transaction = classify_transaction(
            YnabTransaction {
                id: "internet".to_string(),
                date: "2026-05-02".to_string(),
                amount: -91_000,
                payee_name: Some("Telecom provider".to_string()),
                category_name: Some("Internet".to_string()),
                memo: None,
                deleted: false,
            },
            &PlannerState::sample().bills,
        );

        assert_eq!(transaction.classification, TransactionClass::Recurring);
        assert_eq!(transaction.matched_bill_id, Some(1));
    }

    #[test]
    fn detects_monthly_recurring_transactions_from_history() {
        let transactions = tracked_series(
            "Streaming Service",
            &["2026-01-05", "2026-02-05", "2026-03-06", "2026-04-05"],
            -19.99,
        );

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].cadence, RecurringCadence::Monthly);
        assert!(candidates[0].confidence > 0.8);
    }

    #[test]
    fn detects_monthly_recurring_transactions_with_large_day_drift() {
        let transactions = tracked_with_category_series(
            "Variable Utility",
            "Utilities",
            &["2026-01-31", "2026-02-02", "2026-03-28", "2026-04-03"],
            -110.0,
        );

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].category_name, "Utilities");
        assert_eq!(candidates[0].cadence, RecurringCadence::Monthly);
    }

    #[test]
    fn detects_monthly_utility_with_large_amount_variance_by_category() {
        let transactions = vec![
            tracked_with_category("Hydro-Quebec", "Hydro-Quebec", "2026-01-15", -55.0),
            tracked_with_category("Hydro-Quebec", "Hydro-Quebec", "2026-02-16", -180.0),
            tracked_with_category("Hydro-Quebec", "Hydro-Quebec", "2026-03-15", -92.0),
        ];

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].category_name, "Hydro-Quebec");
        assert_eq!(candidates[0].cadence, RecurringCadence::Monthly);
    }

    #[test]
    fn detects_weekly_recurring_transactions_from_history() {
        let transactions = tracked_series(
            "Weekly Cleaner",
            &["2026-01-02", "2026-01-09", "2026-01-16", "2026-01-23"],
            -85.0,
        );

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].cadence, RecurringCadence::Weekly);
    }

    #[test]
    fn detects_biweekly_royal_bank_transactions_from_history() {
        let transactions = tracked_series(
            "Royal Bank",
            &["2026-01-03", "2026-01-17", "2026-01-31", "2026-02-14"],
            -1200.0,
        );

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].payee_name, "Royal Bank");
        assert_eq!(candidates[0].cadence, RecurringCadence::Biweekly);
    }

    #[test]
    fn detects_semimonthly_paychecks_from_history() {
        let transactions = tracked_series(
            "Employer Payroll",
            &[
                "2026-01-15",
                "2026-01-30",
                "2026-02-15",
                "2026-02-28",
                "2026-03-15",
                "2026-03-30",
            ],
            1620.25,
        );

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].cadence, RecurringCadence::Semimonthly);
        assert_eq!(candidates[0].last_amount, 1620.25);
    }

    #[test]
    fn detects_yearly_recurring_transactions_from_history() {
        let transactions = tracked_series(
            "Annual Insurance",
            &["2024-04-15", "2025-04-15", "2026-04-14"],
            -325.0,
        );

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].cadence, RecurringCadence::Yearly);
    }

    #[test]
    fn detects_yearly_recurring_transactions_with_large_month_drift() {
        let transactions = tracked_with_category_series(
            "Home Insurance",
            "Insurance",
            &["2024-04-15", "2025-06-10"],
            -925.0,
        );

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].category_name, "Insurance");
        assert_eq!(candidates[0].cadence, RecurringCadence::Yearly);
    }

    #[test]
    fn recurring_candidates_reclassify_matching_misc_outflows() {
        let transactions = tracked_series(
            "Gym Membership",
            &["2026-01-10", "2026-02-10", "2026-03-10"],
            -42.0,
        );
        let candidates = detect_recurring_candidates(&transactions);
        let classified = apply_recurring_candidates(transactions, &candidates, &[], &[]);

        assert!(classified
            .iter()
            .all(|transaction| transaction.classification == TransactionClass::Recurring));
    }

    #[test]
    fn transaction_recurring_label_shows_detected_cadence() {
        let transactions = tracked_series(
            "Weekly Cleaner",
            &["2026-01-02", "2026-01-09", "2026-01-16", "2026-01-23"],
            -85.0,
        );
        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(
            transaction_recurring_label(&transactions[0], &[], &[], &candidates),
            "Weekly"
        );
    }

    #[test]
    fn transaction_recurring_label_shows_bill_frequency() {
        let mut transaction =
            tracked_with_category("Insurer", "Home Insurance", "2026-04-15", -925.0);
        transaction.matched_bill_id = Some(12);
        let bills = vec![Bill {
            id: 12,
            name: "Home Insurance".to_string(),
            amount: 925.0,
            due_day: 15,
            frequency: Frequency::Yearly,
            annual_increase: 4.0,
            renewal_month: 4,
            anchor_date: None,
            history: Vec::new(),
        }];

        assert_eq!(
            transaction_recurring_label(&transaction, &bills, &[], &[]),
            "Yearly"
        );
    }

    #[test]
    fn manual_misc_correction_is_not_reclassified_by_candidates() {
        let mut transactions = tracked_series(
            "Gym Membership",
            &["2026-01-10", "2026-02-10", "2026-03-10"],
            -42.0,
        );
        transactions[0].manual_classification = Some(TransactionClass::Misc);
        let candidates = detect_recurring_candidates(&transactions);
        let classified = apply_recurring_candidates(transactions, &candidates, &[], &[]);

        assert_eq!(classified[0].classification, TransactionClass::Misc);
        assert!(classified[0].manual_classification.is_some());
        assert!(classified[1..]
            .iter()
            .all(|transaction| transaction.classification == TransactionClass::Recurring));
    }

    #[test]
    fn import_preserves_manual_transaction_corrections() {
        let mut previous = tracked("Insurance Provider", "2026-04-15", -925.0);
        previous.category_name = "Home Insurance".to_string();
        previous.classification = TransactionClass::Recurring;
        previous.matched_bill_id = Some(7);
        previous.manual_classification = Some(TransactionClass::Recurring);
        let imported = vec![tracked("Insurance Provider", "2026-04-15", -925.0)];

        let merged = merge_transaction_corrections(imported, &[previous]);

        assert_eq!(merged[0].category_name, "Home Insurance");
        assert_eq!(merged[0].classification, TransactionClass::Recurring);
        assert_eq!(merged[0].matched_bill_id, Some(7));
        assert_eq!(
            merged[0].manual_classification,
            Some(TransactionClass::Recurring)
        );
    }

    #[test]
    fn detected_monthly_recurring_category_creates_bill_entry() {
        let mut state = PlannerState::default();
        state.transactions = tracked_with_category_series(
            "Hydro Provider",
            "Utilities",
            &["2026-01-12", "2026-02-12", "2026-03-12"],
            -90.0,
        );
        state.recurring_candidates = detect_recurring_candidates(&state.transactions);

        sync_detected_bills(&mut state);
        let classified = apply_recurring_candidates(
            state.transactions.clone(),
            &state.recurring_candidates,
            &state.bills,
            &state.paychecks,
        );

        assert_eq!(state.bills.len(), 1);
        assert_eq!(state.bills[0].name, "Utilities");
        assert_eq!(state.bills[0].frequency, Frequency::Monthly);
        assert!(classified
            .iter()
            .all(|transaction| transaction.matched_bill_id == Some(state.bills[0].id)));
        assert!(classified
            .iter()
            .all(|transaction| transaction.category_name == "Utilities"));
    }

    #[test]
    fn detected_biweekly_recurring_payee_creates_bill_entry() {
        let mut state = PlannerState::default();
        state.transactions = tracked_series(
            "Royal Bank",
            &["2026-01-03", "2026-01-17", "2026-01-31", "2026-02-14"],
            -1200.0,
        );
        state.recurring_candidates = detect_recurring_candidates(&state.transactions);

        sync_detected_bills(&mut state);
        let classified = apply_recurring_candidates(
            state.transactions.clone(),
            &state.recurring_candidates,
            &state.bills,
            &state.paychecks,
        );

        assert_eq!(state.bills.len(), 1);
        assert_eq!(state.bills[0].name, "Royal Bank");
        assert_eq!(state.bills[0].frequency, Frequency::Biweekly);
        assert_eq!(state.bills[0].anchor_date.as_deref(), Some("2026-02-14"));
        assert!(classified
            .iter()
            .all(|transaction| transaction.matched_bill_id == Some(state.bills[0].id)));
    }

    #[test]
    fn detected_semimonthly_income_creates_paycheck_entry() {
        let mut state = PlannerState::default();
        state.settings.paycheck_amount = 3818.0;
        state.transactions = tracked_series(
            "Employer Payroll",
            &[
                "2026-01-15",
                "2026-01-30",
                "2026-02-15",
                "2026-02-28",
                "2026-03-15",
                "2026-03-30",
            ],
            1620.25,
        );
        state.recurring_candidates = detect_recurring_candidates(&state.transactions);

        sync_detected_bills(&mut state);
        let classified = apply_recurring_candidates(
            state.transactions.clone(),
            &state.recurring_candidates,
            &state.bills,
            &state.paychecks,
        );

        assert!(state.bills.is_empty());
        assert_eq!(state.paychecks.len(), 1);
        assert_eq!(state.paychecks[0].name, "Employer Payroll");
        assert_eq!(state.paychecks[0].amount, 1620.25);
        assert_eq!(state.paychecks[0].frequency, Frequency::Semimonthly);
        assert_eq!(state.settings.paycheck_amount, 3818.0);
        assert!(classified
            .iter()
            .all(|transaction| transaction.classification == TransactionClass::Paycheck));
        assert!(classified
            .iter()
            .all(|transaction| transaction.matched_bill_id == Some(state.paychecks[0].id)));
    }

    #[test]
    fn detected_paycheck_prefers_payee_over_ynab_inflow_category() {
        let mut state = PlannerState::default();
        state.transactions = tracked_with_category_series(
            "Employer Payroll",
            "Inflow: Ready to Assign",
            &[
                "2026-01-15",
                "2026-01-30",
                "2026-02-15",
                "2026-02-28",
                "2026-03-15",
                "2026-03-30",
            ],
            1620.25,
        );
        state.recurring_candidates = detect_recurring_candidates(&state.transactions);

        sync_detected_bills(&mut state);

        assert_eq!(state.paychecks.len(), 1);
        assert_eq!(state.paychecks[0].name, "Employer Payroll");
    }

    #[test]
    fn recurring_refresh_builds_bills_and_paycheck_transfers_from_imported_transactions() {
        let mut state = PlannerState::default();
        state.settings.paycheck_amount = 3818.0;
        state.transactions = vec![
            tracked("Employer Payroll", "2026-01-15", 1620.25),
            tracked("Employer Payroll", "2026-01-30", 1620.25),
            tracked("Employer Payroll", "2026-02-15", 1620.25),
            tracked("Employer Payroll", "2026-02-28", 1620.25),
            tracked("Employer Payroll", "2026-03-15", 1620.25),
            tracked("Employer Payroll", "2026-03-30", 1620.25),
            tracked("Royal Bank", "2026-01-03", -1200.0),
            tracked("Royal Bank", "2026-01-17", -1200.0),
            tracked("Royal Bank", "2026-01-31", -1200.0),
            tracked("Royal Bank", "2026-02-14", -1200.0),
            tracked_with_category("Hydro-Quebec", "Hydro-Quebec", "2026-01-15", -55.0),
            tracked_with_category("Hydro-Quebec", "Hydro-Quebec", "2026-02-16", -180.0),
            tracked_with_category("Hydro-Quebec", "Hydro-Quebec", "2026-03-15", -92.0),
            tracked_with_category("Gaspesie Trip", "Gaspesie", "2026-03-20", -40.0),
        ];

        refresh_recurring_detection(&mut state);

        assert_eq!(state.settings.paycheck_amount, 3818.0);
        assert_eq!(state.paychecks.len(), 1);
        assert_eq!(state.paychecks[0].name, "Employer Payroll");
        assert_eq!(state.paychecks[0].frequency, Frequency::Semimonthly);
        assert!(state
            .bills
            .iter()
            .any(|bill| bill.name == "Royal Bank" && bill.frequency == Frequency::Biweekly));
        assert!(state
            .bills
            .iter()
            .any(|bill| bill.name == "Hydro-Quebec" && bill.frequency == Frequency::Monthly));

        assert_eq!(
            state
                .transactions
                .iter()
                .filter(|transaction| transaction.classification == TransactionClass::Paycheck)
                .count(),
            6
        );
        assert_eq!(
            state
                .transactions
                .iter()
                .filter(|transaction| transaction.classification == TransactionClass::Recurring)
                .count(),
            7
        );
        assert_eq!(
            state
                .transactions
                .iter()
                .filter(|transaction| transaction.classification == TransactionClass::Misc)
                .count(),
            1
        );
    }

    #[test]
    fn paycheck_transaction_select_value_uses_paycheck_transfer_namespace() {
        let paychecks = vec![Bill {
            id: 8,
            name: "Employer Payroll".to_string(),
            amount: 1620.25,
            due_day: 30,
            frequency: Frequency::Semimonthly,
            annual_increase: 0.0,
            renewal_month: 3,
            anchor_date: Some("2026-03-30".to_string()),
            history: Vec::new(),
        }];
        let mut transaction = tracked("Employer Payroll", "2026-03-30", 1620.25);
        transaction.classification = TransactionClass::Paycheck;
        transaction.matched_bill_id = Some(8);

        assert_eq!(
            transaction_bill_select_value(&transaction, &[], &paychecks),
            "paycheck:8"
        );
    }

    #[test]
    fn grouping_places_detected_paychecks_in_paycheck_transfer_group() {
        let paychecks = vec![Bill {
            id: 8,
            name: "Employer Payroll".to_string(),
            amount: 1620.25,
            due_day: 30,
            frequency: Frequency::Semimonthly,
            annual_increase: 0.0,
            renewal_month: 3,
            anchor_date: Some("2026-03-30".to_string()),
            history: Vec::new(),
        }];
        let mut transaction = tracked("Employer Payroll", "2026-03-30", 1620.25);
        transaction.classification = TransactionClass::Paycheck;
        transaction.matched_bill_id = Some(8);

        let groups = group_transactions_by_category(
            vec![transaction],
            TransactionSortColumn::Date,
            false,
            &[],
            &paychecks,
            &[],
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "Paycheck Transfers");
        assert_eq!(
            transaction_group_recurring_label(&groups[0].1, &[], &paychecks, &[]),
            "Twice monthly"
        );
        assert_eq!(
            transaction_group_amount_label(&groups[0].1, &[], &paychecks, &[]),
            "$1620.25"
        );
    }

    #[test]
    fn existing_auto_recurring_transaction_gets_bill_category_repaired() {
        let mut state = PlannerState::default();
        state.transactions = tracked_series(
            "Hydro-Quebec",
            &["2026-01-12", "2026-02-12", "2026-03-12"],
            -120.0,
        );
        for transaction in &mut state.transactions {
            transaction.classification = TransactionClass::Recurring;
        }
        state.recurring_candidates = detect_recurring_candidates(&state.transactions);
        sync_detected_bills(&mut state);

        let classified = apply_recurring_candidates(
            state.transactions.clone(),
            &state.recurring_candidates,
            &state.bills,
            &state.paychecks,
        );

        assert_eq!(state.bills[0].name, "Hydro-Quebec");
        assert!(classified
            .iter()
            .all(|transaction| transaction.category_name == "Hydro-Quebec"));
    }

    #[test]
    fn existing_bill_match_gets_bill_category_repaired() {
        let mut transaction = tracked("Videotron", "2026-01-12", -160.0);
        transaction.classification = TransactionClass::Recurring;
        transaction.matched_bill_id = Some(9);
        let bills = vec![Bill {
            id: 9,
            name: "Internet".to_string(),
            amount: 160.0,
            due_day: 12,
            frequency: Frequency::Monthly,
            annual_increase: 3.0,
            renewal_month: 1,
            anchor_date: None,
            history: Vec::new(),
        }];

        let classified = apply_recurring_candidates(vec![transaction], &[], &bills, &[]);

        assert_eq!(classified[0].category_name, "Internet");
        assert_eq!(classified[0].classification, TransactionClass::Recurring);
    }

    #[test]
    fn transactions_sort_newest_first_by_default() {
        let mut transactions = vec![
            tracked("Old", "2024-01-01", -10.0),
            tracked("New", "2024-03-01", -10.0),
            tracked("Middle", "2024-02-01", -10.0),
        ];

        sort_transactions_by_date(&mut transactions, true);

        assert_eq!(transactions[0].date, "2024-03-01");
        assert_eq!(transactions[2].date, "2024-01-01");
    }

    #[test]
    fn transactions_sort_oldest_first_when_requested() {
        let mut transactions = vec![
            tracked("Old", "2024-01-01", -10.0),
            tracked("New", "2024-03-01", -10.0),
            tracked("Middle", "2024-02-01", -10.0),
        ];

        sort_transactions_by_date(&mut transactions, false);

        assert_eq!(transactions[0].date, "2024-01-01");
        assert_eq!(transactions[2].date, "2024-03-01");
    }

    #[test]
    fn actual_balance_points_reconstruct_history_from_current_balance() {
        let transactions = vec![
            tracked("Paycheck Transfer", "2026-05-15", 500.0),
            tracked("Internet", "2026-05-18", -91.0),
            tracked("Hydro", "2026-05-18", -109.0),
        ];

        let points = actual_balance_points(&transactions, 1300.0, days_from_civil(2026, 5, 18));

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].balance, 1500.0);
        assert_eq!(points[0].inflow, 500.0);
        assert_eq!(points[0].outflow, 0.0);
        assert_eq!(points[1].balance, 1300.0);
        assert_eq!(points[1].inflow, 0.0);
        assert_eq!(points[1].outflow, 200.0);
    }

    #[test]
    fn actual_balance_points_end_at_today_for_continuous_chart() {
        let transactions = vec![tracked("Internet", "2026-05-18", -100.0)];

        let points = actual_balance_points(&transactions, 900.0, days_from_civil(2026, 5, 20));

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].day, days_from_civil(2026, 5, 18));
        assert_eq!(points[0].balance, 900.0);
        assert_eq!(points[1].day, days_from_civil(2026, 5, 20));
        assert_eq!(points[1].balance, 900.0);
    }

    #[test]
    fn shortfall_add_on_spreads_deficit_over_remaining_paydays() {
        let mut state = PlannerState::default();
        state.settings.minimum_buffer = 250.0;
        state.settings.margin_percent = 8.0;
        let forecast = Forecast {
            daily: vec![DailyPoint {
                date: Date {
                    year: 2026,
                    month: 5,
                    day: 3,
                },
                balance: 900.0,
                inflow: 0.0,
                outflow: 0.0,
            }],
            events: Vec::new(),
            low_point: DailyPoint {
                date: Date {
                    year: 2026,
                    month: 11,
                    day: 14,
                },
                balance: -370.80,
                inflow: 0.0,
                outflow: 0.0,
            },
            current_year_outflow: 0.0,
        };

        assert_eq!(
            paydays_before(
                Date {
                    year: 2026,
                    month: 5,
                    day: 3,
                },
                forecast.low_point.date,
            ),
            12
        );
        assert_eq!(
            shortfall_add_on_per_paycheck(&forecast, &state),
            Some(55.87)
        );
        assert_eq!(
            recommended_transfer_note(&forecast, &state),
            "+$55.87 to stay afloat"
        );
        assert_eq!(
            recommended_transfer_value(&forecast, &state, 200.0),
            "$255.87"
        );
    }

    #[test]
    fn next_bill_due_date_uses_frequency_month_and_day() {
        let mut bill = Bill {
            id: 1,
            name: "Insurance".to_string(),
            amount: 100.0,
            due_day: 12,
            frequency: Frequency::Yearly,
            annual_increase: 3.0,
            renewal_month: 4,
            anchor_date: None,
            history: Vec::new(),
        };

        assert_eq!(
            next_bill_due_date(
                &bill,
                Date {
                    year: 2026,
                    month: 5,
                    day: 3,
                },
            ),
            Date {
                year: 2027,
                month: 4,
                day: 12,
            }
        );

        bill.frequency = Frequency::Monthly;
        assert_eq!(
            next_bill_due_date(
                &bill,
                Date {
                    year: 2026,
                    month: 5,
                    day: 3,
                },
            ),
            Date {
                year: 2026,
                month: 5,
                day: 12,
            }
        );
    }

    #[test]
    fn next_bill_due_date_supports_biweekly_anchor_dates() {
        let bill = Bill {
            id: 1,
            name: "Mortgage".to_string(),
            amount: 1200.0,
            due_day: 3,
            frequency: Frequency::Biweekly,
            annual_increase: 3.0,
            renewal_month: 5,
            anchor_date: Some("2026-05-03".to_string()),
            history: Vec::new(),
        };

        assert_eq!(
            next_bill_due_date(
                &bill,
                Date {
                    year: 2026,
                    month: 5,
                    day: 4,
                },
            ),
            Date {
                year: 2026,
                month: 5,
                day: 17,
            }
        );
    }

    #[test]
    fn next_thursday_after_picks_the_following_thursday() {
        assert_eq!(
            next_thursday_after(Date {
                year: 2026,
                month: 5,
                day: 10,
            }),
            Date {
                year: 2026,
                month: 5,
                day: 14,
            }
        );
        assert_eq!(
            next_thursday_after(Date {
                year: 2026,
                month: 5,
                day: 14,
            }),
            Date {
                year: 2026,
                month: 5,
                day: 21,
            }
        );
    }

    #[test]
    fn one_off_transactions_do_not_create_candidates() {
        let transactions = vec![
            tracked("Coffee Shop", "2026-01-10", -6.25),
            tracked("Hardware Store", "2026-02-18", -48.0),
        ];

        let candidates = detect_recurring_candidates(&transactions);

        assert!(candidates.is_empty());
    }

    #[test]
    fn category_groups_recurring_transactions_when_payees_vary() {
        let transactions = vec![
            tracked_with_category("Hydro Provider A", "Utilities", "2026-01-12", -88.0),
            tracked_with_category("Hydro Provider B", "Utilities", "2026-02-12", -91.0),
            tracked_with_category("Hydro Provider C", "Utilities", "2026-03-12", -89.5),
        ];

        let candidates = detect_recurring_candidates(&transactions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].category_name, "Utilities");
        assert_eq!(candidates[0].cadence, RecurringCadence::Monthly);
    }

    #[test]
    fn bill_grouping_uses_bill_assignment_not_raw_transaction_category() {
        let transactions = vec![
            tracked_with_category("Gaspesie Trip", "Gaspesie", "2026-04-01", -120.0),
            tracked_with_category("Cable Provider", "Internet", "2026-04-02", -91.0),
            tracked_with_category("Unknown Import", "Uncategorized", "2026-04-03", -42.0),
        ];
        let bills = vec![
            Bill {
                id: 1,
                name: "Internet".to_string(),
                amount: 91.0,
                due_day: 2,
                frequency: Frequency::Monthly,
                annual_increase: 3.0,
                renewal_month: 1,
                anchor_date: None,
                history: Vec::new(),
            },
            Bill {
                id: 2,
                name: "Uncategorized".to_string(),
                amount: 42.0,
                due_day: 3,
                frequency: Frequency::Monthly,
                annual_increase: 3.0,
                renewal_month: 1,
                anchor_date: None,
                history: Vec::new(),
            },
        ];

        let groups = group_transactions_by_category(
            transactions,
            TransactionSortColumn::Date,
            false,
            &bills,
            &[],
            &[],
        );
        let unassigned = groups
            .iter()
            .find(|(name, _)| name == "Unassigned")
            .expect("unassigned bill group should exist");
        let internet = groups
            .iter()
            .find(|(name, _)| name == "Internet")
            .expect("internet bill group should exist");

        assert_eq!(unassigned.1.len(), 2);
        assert!(unassigned
            .1
            .iter()
            .any(|transaction| transaction.category_name == "Gaspesie"));
        assert!(unassigned
            .1
            .iter()
            .any(
                |transaction| normalize_category_name(&transaction.category_name)
                    == DEFAULT_CATEGORY_NAME
            ));
        assert_eq!(internet.1[0].category_name, "Internet");
        assert!(groups.iter().all(|(name, _)| name != "Gaspesie"));
        assert!(groups.iter().all(|(name, _)| name != "Uncategorized"));
        assert_eq!(
            groups.last().map(|(name, _)| name.as_str()),
            Some("Unassigned")
        );
    }

    #[test]
    fn group_summary_uses_latest_regular_amount() {
        let transactions = vec![
            tracked_with_category("Hydro-Quebec", "Electricite", "2026-01-15", -88.0),
            tracked_with_category("Hydro-Quebec", "Electricite", "2026-02-15", -88.0),
            tracked_with_category("Hydro-Quebec", "Electricite", "2026-03-15", -94.0),
        ];
        let bills = vec![Bill {
            id: 3,
            name: "Electricite".to_string(),
            amount: 88.0,
            due_day: 15,
            frequency: Frequency::Monthly,
            annual_increase: 3.0,
            renewal_month: 3,
            anchor_date: None,
            history: Vec::new(),
        }];

        assert_eq!(
            transaction_group_recurring_label(&transactions, &bills, &[], &[]),
            "Monthly"
        );
        assert_eq!(
            transaction_group_amount_label(&transactions, &bills, &[], &[]),
            "$94.00"
        );
    }

    fn tracked_series(payee: &str, dates: &[&str], amount: f64) -> Vec<TrackedTransaction> {
        dates
            .iter()
            .map(|date| tracked(payee, date, amount))
            .collect()
    }

    fn tracked_with_category_series(
        payee: &str,
        category: &str,
        dates: &[&str],
        amount: f64,
    ) -> Vec<TrackedTransaction> {
        dates
            .iter()
            .map(|date| tracked_with_category(payee, category, date, amount))
            .collect()
    }

    fn tracked(payee: &str, date: &str, amount: f64) -> TrackedTransaction {
        TrackedTransaction {
            id: format!("{payee}-{date}"),
            date: date.to_string(),
            payee_name: payee.to_string(),
            category_name: DEFAULT_CATEGORY_NAME.to_string(),
            memo: String::new(),
            amount,
            classification: TransactionClass::Misc,
            matched_bill_id: None,
            manual_classification: None,
        }
    }

    fn tracked_with_category(
        payee: &str,
        category: &str,
        date: &str,
        amount: f64,
    ) -> TrackedTransaction {
        TrackedTransaction {
            category_name: category.to_string(),
            ..tracked(payee, date, amount)
        }
    }
}
