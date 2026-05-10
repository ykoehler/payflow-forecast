use crate::models::{Bill, Frequency, PlannerState, Settings};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DailyPoint {
    pub date: Date,
    pub balance: f64,
    pub inflow: f64,
    pub outflow: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForecastEvent {
    pub date: Date,
    pub name: String,
    pub amount: f64,
    pub balance: f64,
    pub event_type: EventType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventType {
    Transfer,
    Payment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Forecast {
    pub daily: Vec<DailyPoint>,
    pub events: Vec<ForecastEvent>,
    pub low_point: DailyPoint,
    pub current_year_outflow: f64,
}

pub fn optimize_transfer(state: &PlannerState, start: Date) -> f64 {
    if state.bills.is_empty() || state.paychecks.is_empty() {
        return 0.0;
    }

    simulate(state, start)
        .events
        .iter()
        .find(|event| event.event_type == EventType::Transfer)
        .map(|event| event.amount)
        .unwrap_or(0.0)
}

pub fn simulate(state: &PlannerState, start: Date) -> Forecast {
    let mut date = start;
    let end = Date {
        year: start.year + state.settings.forecast_years as i32,
        month: start.month,
        day: start.day,
    };
    let mut balance = state.settings.starting_balance;
    let mut daily = Vec::new();
    let mut events = Vec::new();
    let mut current_year_outflow = 0.0;

    while date <= end {
        let inflow = transfer_for_date(state, date, start, balance);
        let mut outflow = 0.0;

        if inflow > 0.0 {
            balance += inflow;
            events.push(ForecastEvent {
                date,
                name: "Paycheck transfer".to_string(),
                amount: inflow,
                balance,
                event_type: EventType::Transfer,
            });
        }

        for bill in &state.bills {
            if bill_occurs(bill, date) {
                let amount = projected_bill_amount(bill, date, start);
                balance -= amount;
                outflow += amount;
                if date.year == start.year {
                    current_year_outflow += amount;
                }
                events.push(ForecastEvent {
                    date,
                    name: bill.name.clone(),
                    amount: -amount,
                    balance,
                    event_type: EventType::Payment,
                });
            }
        }

        daily.push(DailyPoint {
            date,
            balance,
            inflow,
            outflow,
        });
        date = date.next_day();
    }

    let low_point = daily
        .iter()
        .min_by(|left, right| left.balance.total_cmp(&right.balance))
        .cloned()
        .unwrap_or(DailyPoint {
            date: start,
            balance,
            inflow: 0.0,
            outflow: 0.0,
        });

    Forecast {
        daily,
        events,
        low_point,
        current_year_outflow,
    }
}

pub fn required_floor(settings: &Settings, bills: &[Bill]) -> f64 {
    let _ = bills;
    settings.minimum_buffer
}

pub fn historical_increase_summary(bill: &Bill) -> (f64, f64) {
    let increases: Vec<f64> = bill
        .history
        .windows(2)
        .filter_map(|window| {
            let previous = window[0].amount;
            let current = window[1].amount;
            (previous > 0.0).then_some(((current - previous) / previous) * 100.0)
        })
        .collect();

    if increases.is_empty() {
        return (0.0, 0.0);
    }

    let latest = *increases.last().unwrap_or(&0.0);
    let average = increases.iter().sum::<f64>() / increases.len() as f64;
    (average, latest)
}

fn transfer_for_date(state: &PlannerState, date: Date, start: Date, balance: f64) -> f64 {
    if state.bills.is_empty() || state.paychecks.is_empty() {
        return 0.0;
    }

    let last_payday = date.day == days_in_month(date.year, date.month).min(30);
    if date.day == 15 || last_payday {
        let required_reserve = required_reserve_for_date(state, date, start)
            .max(next_paycheck_payment_reserve(state, date, start));
        let base_transfer = (required_reserve - balance).max(0.0);
        ceil_cents(base_transfer * (1.0 + state.settings.margin_percent / 100.0))
    } else {
        0.0
    }
}

fn required_reserve_for_date(state: &PlannerState, date: Date, start: Date) -> f64 {
    state.settings.minimum_buffer
        + state
            .bills
            .iter()
            .map(|bill| accrued_bill_reserve(bill, date, start))
            .sum::<f64>()
}

fn next_paycheck_payment_reserve(state: &PlannerState, date: Date, start: Date) -> f64 {
    state.settings.minimum_buffer
        + state
            .bills
            .iter()
            .filter_map(|bill| {
                let next_due = next_bill_date_on_or_after(bill, date);
                (next_due < next_payday_after(date))
                    .then_some(projected_bill_amount(bill, next_due, start))
            })
            .sum::<f64>()
}

fn accrued_bill_reserve(bill: &Bill, date: Date, start: Date) -> f64 {
    let next_due = next_bill_date_on_or_after(bill, date);
    let previous_due = previous_bill_date_before(bill, next_due);
    let total_days = days_between(previous_due, next_due).max(1) as f64;
    let elapsed_days = days_between(previous_due, date).clamp(0, total_days as i64) as f64;
    let amount = projected_bill_amount(bill, next_due, start);

    amount * (elapsed_days / total_days)
}

fn next_bill_date_on_or_after(bill: &Bill, mut date: Date) -> Date {
    for _ in 0..=370 {
        if bill_occurs(bill, date) {
            return date;
        }
        date = date.next_day();
    }

    date
}

fn previous_bill_date_before(bill: &Bill, mut date: Date) -> Date {
    date = date.previous_day();
    for _ in 0..=370 {
        if bill_occurs(bill, date) {
            return date;
        }
        date = date.previous_day();
    }

    date
}

fn next_payday_after(date: Date) -> Date {
    let last_payday = days_in_month(date.year, date.month).min(30);
    if date.day < 15 {
        Date { day: 15, ..date }
    } else if date.day < last_payday {
        Date {
            day: last_payday,
            ..date
        }
    } else if date.month < 12 {
        Date {
            year: date.year,
            month: date.month + 1,
            day: 15,
        }
    } else {
        Date {
            year: date.year + 1,
            month: 1,
            day: 15,
        }
    }
}

fn bill_occurs(bill: &Bill, date: Date) -> bool {
    if bill.frequency == Frequency::Biweekly {
        return biweekly_bill_occurs(bill, date);
    }

    if bill.frequency == Frequency::Semimonthly {
        return semimonthly_bill_occurs(date);
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

fn semimonthly_bill_occurs(date: Date) -> bool {
    date.day == 15 || date.day == 30.min(days_in_month(date.year, date.month))
}

fn biweekly_bill_occurs(bill: &Bill, date: Date) -> bool {
    let Some(anchor) = bill_anchor_date(bill) else {
        return false;
    };
    let delta = days_between(anchor, date);
    delta >= 0 && delta % 14 == 0
}

fn bill_anchor_date(bill: &Bill) -> Option<Date> {
    bill.anchor_date
        .as_deref()
        .and_then(parse_anchor_date)
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

fn parse_anchor_date(value: &str) -> Option<Date> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?.clamp(1, 12);
    let day = parts
        .next()?
        .parse::<u32>()
        .ok()?
        .clamp(1, days_in_month(year, month));
    Some(Date { year, month, day })
}

fn projected_bill_amount(bill: &Bill, date: Date, start: Date) -> f64 {
    let mut years = date.year - start.year;
    let renewal_passed = date.month >= bill.renewal_month;
    let start_renewal_passed = start.month >= bill.renewal_month;

    if renewal_passed && !start_renewal_passed {
        years += 1;
    } else if !renewal_passed && start_renewal_passed {
        years -= 1;
    }

    let years = years.max(0) as i32;
    bill.amount * (1.0 + bill.annual_increase / 100.0).powi(years)
}

fn ceil_cents(value: f64) -> f64 {
    (value * 100.0).ceil() / 100.0
}

fn days_between(left: Date, right: Date) -> i64 {
    days_from_civil(right.year, right.month, right.day)
        - days_from_civil(left.year, left.month, left.day)
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

impl Date {
    pub fn today() -> Self {
        let now = js_sys::Date::new_0();
        Self {
            year: now.get_full_year() as i32,
            month: now.get_month() + 1,
            day: now.get_date(),
        }
    }

    pub fn label(self) -> String {
        format!("{} {}, {}", month_label(self.month), self.day, self.year)
    }

    pub fn next_day(self) -> Self {
        let max_day = days_in_month(self.year, self.month);
        if self.day < max_day {
            return Self {
                day: self.day + 1,
                ..self
            };
        }

        if self.month < 12 {
            Self {
                year: self.year,
                month: self.month + 1,
                day: 1,
            }
        } else {
            Self {
                year: self.year + 1,
                month: 1,
                day: 1,
            }
        }
    }

    pub fn previous_day(self) -> Self {
        if self.day > 1 {
            return Self {
                day: self.day - 1,
                ..self
            };
        }

        if self.month > 1 {
            let month = self.month - 1;
            Self {
                year: self.year,
                month,
                day: days_in_month(self.year, month),
            }
        } else {
            Self {
                year: self.year - 1,
                month: 12,
                day: 31,
            }
        }
    }
}

impl PartialOrd for Date {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Date {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.year, self.month, self.day).cmp(&(other.year, other.month, other.day))
    }
}

pub fn month_label(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        _ => "Dec",
    }
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Bill, Frequency, PlannerState};

    #[test]
    fn optimized_transfer_keeps_balance_above_floor() {
        let state = PlannerState::sample();
        let start = Date {
            year: 2026,
            month: 5,
            day: 2,
        };
        let transfer = optimize_transfer(&state, start);
        let forecast = simulate(&state, start);

        assert!(transfer > 0.0);
        assert!(forecast.low_point.balance >= required_floor(&state.settings, &state.bills));
    }

    #[test]
    fn transfer_margin_applies_to_payday_top_up_not_account_floor() {
        let mut state = PlannerState::default();
        state.settings.starting_balance = 50.0;
        state.settings.minimum_buffer = 250.0;
        state.settings.margin_percent = 8.0;
        state.settings.forecast_years = 1;
        add_test_paycheck(&mut state);
        state.bills.push(Bill {
            id: 1,
            name: "Placeholder".to_string(),
            amount: 0.0,
            due_day: 1,
            frequency: Frequency::Monthly,
            annual_increase: 0.0,
            renewal_month: 1,
            anchor_date: None,
            history: Vec::new(),
        });

        let transfer = optimize_transfer(
            &state,
            Date {
                year: 2026,
                month: 5,
                day: 15,
            },
        );

        assert_eq!(transfer, 216.0);
        assert_eq!(required_floor(&state.settings, &state.bills), 250.0);
    }

    #[test]
    fn adaptive_transfers_do_not_accumulate_unbounded_cash() {
        let mut state = PlannerState::default();
        state.settings.starting_balance = 250.0;
        state.settings.minimum_buffer = 250.0;
        state.settings.margin_percent = 0.0;
        state.settings.forecast_years = 5;
        add_test_paycheck(&mut state);
        state.bills.push(Bill {
            id: 1,
            name: "Monthly bill".to_string(),
            amount: 400.0,
            due_day: 20,
            frequency: Frequency::Monthly,
            annual_increase: 0.0,
            renewal_month: 1,
            anchor_date: None,
            history: Vec::new(),
        });

        let forecast = simulate(
            &state,
            Date {
                year: 2026,
                month: 5,
                day: 2,
            },
        );
        let max_balance = forecast
            .daily
            .iter()
            .map(|point| point.balance)
            .fold(f64::NEG_INFINITY, f64::max);

        assert!(forecast.low_point.balance >= state.settings.minimum_buffer);
        assert!(max_balance < 800.0);
    }

    #[test]
    fn yearly_bill_occurs_only_in_renewal_month() {
        let bill = PlannerState::sample().bills[3].clone();

        assert!(bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 4,
                day: 12
            }
        ));
        assert!(!bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 5,
                day: 12
            }
        ));
    }

    #[test]
    fn biweekly_bill_occurs_every_fourteen_days_from_anchor() {
        let bill = Bill {
            id: 1,
            name: "Mortgage".to_string(),
            amount: 1200.0,
            due_day: 3,
            frequency: Frequency::Biweekly,
            annual_increase: 0.0,
            renewal_month: 5,
            anchor_date: Some("2026-05-03".to_string()),
            history: Vec::new(),
        };

        assert!(bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 5,
                day: 3,
            },
        ));
        assert!(bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 5,
                day: 17,
            },
        ));
        assert!(!bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 5,
                day: 10,
            },
        ));
    }

    #[test]
    fn semimonthly_bill_occurs_on_fifteenth_and_month_end_payday() {
        let bill = Bill {
            id: 1,
            name: "Paycheck".to_string(),
            amount: 1620.25,
            due_day: 15,
            frequency: Frequency::Semimonthly,
            annual_increase: 0.0,
            renewal_month: 1,
            anchor_date: None,
            history: Vec::new(),
        };

        assert!(bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 2,
                day: 15,
            },
        ));
        assert!(bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 2,
                day: 28,
            },
        ));
        assert!(!bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 2,
                day: 27,
            },
        ));
    }

    #[test]
    fn quarterly_bill_occurs_from_renewal_month_every_three_months() {
        let bill = Bill {
            id: 1,
            name: "Water tax".to_string(),
            amount: 150.0,
            due_day: 10,
            frequency: Frequency::Quarterly,
            annual_increase: 0.0,
            renewal_month: 2,
            anchor_date: None,
            history: Vec::new(),
        };

        for month in [2, 5, 8, 11] {
            assert!(bill_occurs(
                &bill,
                Date {
                    year: 2026,
                    month,
                    day: 10,
                },
            ));
        }
        assert!(!bill_occurs(
            &bill,
            Date {
                year: 2026,
                month: 3,
                day: 10,
            },
        ));
    }

    #[test]
    fn annual_increase_applies_after_renewal_month() {
        let bill = Bill {
            id: 1,
            name: "Insurance".to_string(),
            amount: 100.0,
            due_day: 1,
            frequency: Frequency::Yearly,
            annual_increase: 10.0,
            renewal_month: 7,
            anchor_date: None,
            history: Vec::new(),
        };
        let start = Date {
            year: 2026,
            month: 5,
            day: 1,
        };

        let before_renewal = projected_bill_amount(
            &bill,
            Date {
                year: 2026,
                month: 6,
                day: 1,
            },
            start,
        );
        let first_renewal = projected_bill_amount(
            &bill,
            Date {
                year: 2026,
                month: 7,
                day: 1,
            },
            start,
        );
        let second_renewal = projected_bill_amount(
            &bill,
            Date {
                year: 2027,
                month: 7,
                day: 1,
            },
            start,
        );

        assert!((before_renewal - 100.0).abs() < 0.001);
        assert!((first_renewal - 110.0).abs() < 0.001);
        assert!((second_renewal - 121.0).abs() < 0.001);
    }

    #[test]
    fn same_day_transfer_happens_before_bill_payment() {
        let mut state = PlannerState::default();
        state.settings.starting_balance = 0.0;
        state.settings.minimum_buffer = 250.0;
        state.settings.margin_percent = 0.0;
        state.settings.forecast_years = 1;
        add_test_paycheck(&mut state);
        state.bills.push(Bill {
            id: 1,
            name: "Internet".to_string(),
            amount: 100.0,
            due_day: 15,
            frequency: Frequency::Monthly,
            annual_increase: 0.0,
            renewal_month: 1,
            anchor_date: None,
            history: Vec::new(),
        });

        let forecast = simulate(
            &state,
            Date {
                year: 2026,
                month: 5,
                day: 15,
            },
        );

        assert_eq!(forecast.events[0].event_type, EventType::Transfer);
        assert_eq!(forecast.events[0].amount, 350.0);
        assert_eq!(forecast.events[1].event_type, EventType::Payment);
        assert_eq!(forecast.events[1].balance, 250.0);
        assert!(forecast.low_point.balance >= state.settings.minimum_buffer);
    }

    #[test]
    fn empty_default_has_no_seed_bills() {
        let state = PlannerState::default();

        assert!(state.bills.is_empty());
        assert!(state.transactions.is_empty());
        assert_eq!(state.settings.starting_balance, 0.0);
    }

    #[test]
    fn empty_default_recommends_no_transfer_instead_of_looping() {
        let state = PlannerState::default();
        let transfer = optimize_transfer(
            &state,
            Date {
                year: 2026,
                month: 5,
                day: 2,
            },
        );

        assert_eq!(transfer, 0.0);
    }

    #[test]
    fn bills_without_paycheck_transfers_do_not_generate_funding_forecast() {
        let mut state = PlannerState::default();
        state.settings.starting_balance = 0.0;
        state.bills.push(Bill {
            id: 1,
            name: "Rent".to_string(),
            amount: 1300.0,
            due_day: 1,
            frequency: Frequency::Monthly,
            annual_increase: 0.0,
            renewal_month: 1,
            anchor_date: None,
            history: Vec::new(),
        });

        let start = Date {
            year: 2026,
            month: 5,
            day: 1,
        };
        let transfer = optimize_transfer(&state, start);
        let forecast = simulate(&state, start);

        assert_eq!(transfer, 0.0);
        assert!(forecast
            .events
            .iter()
            .all(|event| event.event_type != EventType::Transfer));
    }

    #[test]
    fn pre_payday_deficit_does_not_explode_transfer_recommendation() {
        let mut state = PlannerState::sample();
        state.settings.starting_balance = -454.49;
        state.settings.minimum_buffer = 250.0;

        let transfer = optimize_transfer(
            &state,
            Date {
                year: 2026,
                month: 5,
                day: 2,
            },
        );

        assert!(transfer > 0.0);
        assert!(transfer < 2_000.0);
    }

    #[test]
    fn sample_fixture_keeps_development_data_available() {
        let state = PlannerState::sample();

        assert_eq!(state.bills.len(), 4);
        assert!(state.bills.iter().any(|bill| bill.name == "Internet"));
    }

    #[test]
    fn planner_state_round_trips_through_json() {
        let state = PlannerState::sample();
        let serialized = serde_json::to_string(&state).expect("serialize planner state");
        let restored: PlannerState =
            serde_json::from_str(&serialized).expect("deserialize planner state");

        assert_eq!(restored, state);
    }

    fn add_test_paycheck(state: &mut PlannerState) {
        state.paychecks.push(Bill {
            id: 1,
            name: "Paycheck transfer".to_string(),
            amount: 1000.0,
            due_day: 15,
            frequency: Frequency::Semimonthly,
            annual_increase: 0.0,
            renewal_month: 1,
            anchor_date: Some("2026-05-15".to_string()),
            history: Vec::new(),
        });
    }
}
