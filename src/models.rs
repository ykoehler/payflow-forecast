use serde::{Deserialize, Serialize};

pub const DEFAULT_CATEGORY_NAME: &str = "Misc";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Frequency {
    Biweekly,
    Semimonthly,
    Monthly,
    Quarterly,
    Yearly,
}

impl Frequency {
    pub const ALL: [Frequency; 5] = [
        Frequency::Biweekly,
        Frequency::Semimonthly,
        Frequency::Monthly,
        Frequency::Quarterly,
        Frequency::Yearly,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Frequency::Biweekly => "Biweekly",
            Frequency::Semimonthly => "Twice monthly",
            Frequency::Monthly => "Monthly",
            Frequency::Quarterly => "Quarterly",
            Frequency::Yearly => "Yearly",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bill {
    pub id: u32,
    pub name: String,
    pub amount: f64,
    pub due_day: u32,
    pub frequency: Frequency,
    pub annual_increase: f64,
    pub renewal_month: u32,
    #[serde(default)]
    pub anchor_date: Option<String>,
    pub history: Vec<BillHistory>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BillHistory {
    pub year: i32,
    pub amount: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Settings {
    pub starting_balance: f64,
    pub minimum_buffer: f64,
    pub margin_percent: f64,
    pub forecast_years: u32,
    #[serde(default = "default_paycheck_amount")]
    pub paycheck_amount: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlannerState {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub bills: Vec<Bill>,
    #[serde(default)]
    pub paychecks: Vec<Bill>,
    #[serde(default)]
    pub ynab: YnabSettings,
    #[serde(default)]
    pub transactions: Vec<TrackedTransaction>,
    #[serde(default)]
    pub recurring_candidates: Vec<RecurringCandidate>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct YnabSettings {
    pub access_token: String,
    pub plan_id: Option<String>,
    pub account_id: Option<String>,
    pub account_name: String,
    #[serde(default)]
    pub available_plans: Vec<YnabChoice>,
    #[serde(default)]
    pub available_accounts: Vec<YnabChoice>,
    pub last_import_status: String,
    pub last_imported_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct YnabChoice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TrackedTransaction {
    pub id: String,
    pub date: String,
    pub payee_name: String,
    #[serde(default = "default_category_name")]
    pub category_name: String,
    pub memo: String,
    pub amount: f64,
    pub classification: TransactionClass,
    pub matched_bill_id: Option<u32>,
    #[serde(default)]
    pub manual_classification: Option<TransactionClass>,
}

fn default_category_name() -> String {
    DEFAULT_CATEGORY_NAME.to_string()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RecurringCandidate {
    pub payee_name: String,
    #[serde(default)]
    pub category_name: String,
    pub cadence: RecurringCadence,
    pub average_amount: f64,
    pub last_amount: f64,
    pub occurrence_count: usize,
    pub last_date: String,
    pub confidence: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RecurringCadence {
    Weekly,
    Biweekly,
    Semimonthly,
    Monthly,
    Quarterly,
    Yearly,
    Irregular,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionClass {
    Recurring,
    Paycheck,
    Transfer,
    Misc,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            starting_balance: 0.0,
            minimum_buffer: 250.0,
            margin_percent: 8.0,
            forecast_years: 5,
            paycheck_amount: default_paycheck_amount(),
        }
    }
}

fn default_paycheck_amount() -> f64 {
    2600.0
}

impl Default for YnabSettings {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            plan_id: None,
            account_id: None,
            account_name: "Desjardins Conjoint (LaSalle)".to_string(),
            available_plans: Vec::new(),
            available_accounts: Vec::new(),
            last_import_status: "Not connected".to_string(),
            last_imported_at: None,
        }
    }
}

impl Default for PlannerState {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            bills: Vec::new(),
            paychecks: Vec::new(),
            ynab: YnabSettings::default(),
            transactions: Vec::new(),
            recurring_candidates: Vec::new(),
        }
    }
}

impl PlannerState {
    pub fn sample() -> Self {
        Self {
            settings: Settings {
                starting_balance: 950.0,
                ..Settings::default()
            },
            bills: vec![
                Bill {
                    id: 1,
                    name: "Internet".to_string(),
                    amount: 91.0,
                    due_day: 18,
                    frequency: Frequency::Monthly,
                    annual_increase: 6.0,
                    renewal_month: 7,
                    anchor_date: None,
                    history: vec![
                        BillHistory {
                            year: 2023,
                            amount: 75.0,
                        },
                        BillHistory {
                            year: 2024,
                            amount: 82.0,
                        },
                        BillHistory {
                            year: 2025,
                            amount: 91.0,
                        },
                    ],
                },
                Bill {
                    id: 2,
                    name: "Home insurance".to_string(),
                    amount: 178.0,
                    due_day: 3,
                    frequency: Frequency::Monthly,
                    annual_increase: 7.0,
                    renewal_month: 2,
                    anchor_date: None,
                    history: vec![
                        BillHistory {
                            year: 2023,
                            amount: 150.0,
                        },
                        BillHistory {
                            year: 2024,
                            amount: 164.0,
                        },
                        BillHistory {
                            year: 2025,
                            amount: 178.0,
                        },
                    ],
                },
                Bill {
                    id: 3,
                    name: "Mobile plan".to_string(),
                    amount: 64.0,
                    due_day: 26,
                    frequency: Frequency::Monthly,
                    annual_increase: 3.0,
                    renewal_month: 10,
                    anchor_date: None,
                    history: vec![
                        BillHistory {
                            year: 2023,
                            amount: 59.0,
                        },
                        BillHistory {
                            year: 2024,
                            amount: 62.0,
                        },
                        BillHistory {
                            year: 2025,
                            amount: 64.0,
                        },
                    ],
                },
                Bill {
                    id: 4,
                    name: "Vehicle registration".to_string(),
                    amount: 325.0,
                    due_day: 12,
                    frequency: Frequency::Yearly,
                    annual_increase: 4.0,
                    renewal_month: 4,
                    anchor_date: None,
                    history: vec![
                        BillHistory {
                            year: 2023,
                            amount: 294.0,
                        },
                        BillHistory {
                            year: 2024,
                            amount: 310.0,
                        },
                        BillHistory {
                            year: 2025,
                            amount: 325.0,
                        },
                    ],
                },
            ],
            paychecks: Vec::new(),
            ynab: YnabSettings::default(),
            transactions: Vec::new(),
            recurring_candidates: Vec::new(),
        }
    }
}
