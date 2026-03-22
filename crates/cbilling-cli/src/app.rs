use std::collections::BTreeMap;
use std::time::Instant;

use ratatui::widgets::TableState;
use tokio::sync::mpsc::UnboundedSender;

use cbilling::service::{BillingData, CloudBillingService, ProductCost};

use crate::event::AppEvent;

#[derive(Clone)]
pub enum ProviderDataState {
    Loading,
    Loaded(BillingData),
    Error(String),
}

#[derive(PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Cost,
    MoM,
    Product,
    Code,
    Account,
}

impl SortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Cost => Self::MoM,
            Self::MoM => Self::Product,
            Self::Product => Self::Code,
            Self::Code => Self::Account,
            Self::Account => Self::Cost,
        }
    }

}

pub struct App {
    pub should_quit: bool,
    pub active_tab: usize,
    pub providers: Vec<String>,
    pub billing_cycle: String,
    pub data: BTreeMap<String, ProviderDataState>,
    /// Previous month data for MoM comparison
    pub prev_data: BTreeMap<String, ProviderDataState>,
    pub table_state: TableState,
    pub show_help: bool,
    pub last_refresh: Option<Instant>,
    pub spinner_tick: usize,
    pub tx: UnboundedSender<AppEvent>,
    // Search / filter
    pub input_mode: InputMode,
    pub search_query: String,
    // Sort
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    // Expand: product_codes that are expanded to show region detail
    pub expanded: std::collections::HashSet<String>,
}

impl App {
    pub fn new(tx: UnboundedSender<AppEvent>) -> Self {
        let configured = CloudBillingService::get_configured_providers();
        let mut providers = vec!["All".to_string()];
        providers.extend(configured);

        let billing_cycle = chrono::Local::now().format("%Y-%m").to_string();

        Self {
            should_quit: false,
            active_tab: 0,
            providers,
            billing_cycle,
            data: BTreeMap::new(),
            prev_data: BTreeMap::new(),
            table_state: TableState::default(),
            show_help: false,
            last_refresh: None,
            spinner_tick: 0,
            tx,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            sort_column: SortColumn::Cost,
            sort_ascending: false,
            expanded: std::collections::HashSet::new(),
        }
    }

    pub fn start_loading_all(&mut self) {
        for provider in &self.providers {
            if provider == "All" {
                continue;
            }
            self.data
                .insert(provider.clone(), ProviderDataState::Loading);
            let tx = self.tx.clone();
            let provider = provider.clone();
            let cycle = self.billing_cycle.clone();
            tokio::spawn(async move {
                let result = CloudBillingService::query_provider(&provider, &cycle)
                    .await
                    .map_err(|e| e.to_string());
                let _ = tx.send(AppEvent::DataResult {
                    provider,
                    result,
                    is_prev: false,
                });
            });
        }
        // Also load previous month for MoM
        self.load_prev_month();
        self.last_refresh = Some(Instant::now());
    }

    fn load_prev_month(&mut self) {
        let prev_cycle = Self::offset_month(&self.billing_cycle, -1);
        for provider in &self.providers {
            if provider == "All" {
                continue;
            }
            self.prev_data
                .insert(provider.clone(), ProviderDataState::Loading);
            let tx = self.tx.clone();
            let provider = provider.clone();
            let cycle = prev_cycle.clone();
            tokio::spawn(async move {
                let result = CloudBillingService::query_provider(&provider, &cycle)
                    .await
                    .map_err(|e| e.to_string());
                let _ = tx.send(AppEvent::DataResult {
                    provider,
                    result,
                    is_prev: true,
                });
            });
        }
    }

    fn offset_month(cycle: &str, delta: i32) -> String {
        let parts: Vec<&str> = cycle.split('-').collect();
        let year: i32 = parts[0].parse().unwrap_or(2026);
        let month: i32 = parts[1].parse().unwrap_or(1);
        let total = (year * 12 + month - 1) + delta;
        let y = total / 12;
        let m = total % 12 + 1;
        format!("{}-{:02}", y, m)
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::ScrollUp(n) => self.scroll_up(n),
            AppEvent::ScrollDown(n) => self.scroll_down(n),
            AppEvent::Tick => {
                self.spinner_tick = self.spinner_tick.wrapping_add(1);
            }
            AppEvent::DataResult {
                provider,
                result,
                is_prev,
            } => {
                let state = match result {
                    Ok(data) => ProviderDataState::Loaded(data),
                    Err(e) => ProviderDataState::Error(e),
                };
                if is_prev {
                    self.prev_data.insert(provider, state);
                } else {
                    self.data.insert(provider, state);
                }
            }
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Search input mode
        if self.input_mode == InputMode::Search {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                    self.table_state.select(Some(0));
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.table_state.select(Some(0));
                }
                _ => {}
            }
            return;
        }

        if self.show_help {
            self.show_help = false;
            return;
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.should_quit = true,
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => self.page_down(),
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => self.page_up(),
            (_, KeyCode::Char('q')) => self.should_quit = true,
            (_, KeyCode::Char('?')) => self.show_help = true,
            (_, KeyCode::Char('r')) => self.start_loading_all(),
            (_, KeyCode::Left) | (_, KeyCode::Char('m')) => self.prev_month(),
            (_, KeyCode::Right) | (_, KeyCode::Char('M')) => self.next_month(),
            (_, KeyCode::Tab) | (_, KeyCode::Char('l')) => self.next_tab(),
            (_, KeyCode::BackTab) | (_, KeyCode::Char('h')) => self.prev_tab(),
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => self.next_row(),
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => self.prev_row(),
            (_, KeyCode::Char('g')) => self.table_state.select_first(),
            (_, KeyCode::Char('G')) => {
                let len = self.current_products().len();
                if len > 0 {
                    self.table_state.select(Some(len - 1));
                }
            }
            (_, KeyCode::Char('s')) => {
                self.sort_column = self.sort_column.next();
                self.table_state.select(Some(0));
            }
            (_, KeyCode::Char('S')) => {
                self.sort_ascending = !self.sort_ascending;
                self.table_state.select(Some(0));
            }
            (_, KeyCode::Char('1')) => self.set_sort(SortColumn::Product),
            (_, KeyCode::Char('2')) => self.set_sort(SortColumn::Code),
            (_, KeyCode::Char('3')) => self.set_sort(SortColumn::Cost),
            (_, KeyCode::Char('4')) => self.set_sort(SortColumn::MoM),
            (_, KeyCode::Char('5')) => self.set_sort(SortColumn::Account),
            (_, KeyCode::Enter) => self.toggle_expand(),
            (_, KeyCode::Char('/')) => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }
            (_, KeyCode::Esc) => {
                if !self.search_query.is_empty() {
                    self.search_query.clear();
                    self.table_state.select(Some(0));
                }
            }
            _ => {}
        }
    }

    fn next_tab(&mut self) {
        if !self.providers.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.providers.len();
            self.table_state.select(Some(0));
        }
    }

    fn prev_tab(&mut self) {
        if !self.providers.is_empty() {
            self.active_tab = self
                .active_tab
                .checked_sub(1)
                .unwrap_or(self.providers.len() - 1);
            self.table_state.select(Some(0));
        }
    }

    fn next_row(&mut self) {
        let len = self.current_products().len();
        if len == 0 {
            return;
        }
        let i = self
            .table_state
            .selected()
            .map(|i| (i + 1).min(len - 1))
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }

    fn prev_row(&mut self) {
        let i = self
            .table_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }

    fn page_down(&mut self) {
        let len = self.current_products().len();
        if len == 0 {
            return;
        }
        let half = 20;
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some((i + half).min(len - 1)));
    }

    fn page_up(&mut self) {
        let i = self.table_state.selected().unwrap_or(0);
        let half = 20;
        self.table_state.select(Some(i.saturating_sub(half)));
    }

    fn scroll_up(&mut self, n: u16) {
        let i = self
            .table_state
            .selected()
            .unwrap_or(0)
            .saturating_sub(n as usize);
        self.table_state.select(Some(i));
    }

    fn scroll_down(&mut self, n: u16) {
        let len = self.current_products().len();
        if len == 0 {
            return;
        }
        let i = self
            .table_state
            .selected()
            .unwrap_or(0)
            .saturating_add(n as usize)
            .min(len - 1);
        self.table_state.select(Some(i));
    }

    fn toggle_expand(&mut self) {
        let products = self.current_products();
        if let Some(idx) = self.table_state.selected() {
            if let Some(row) = products.get(idx) {
                if row.is_sub_row {
                    return; // can't expand a sub-row
                }
                let key = row.product_code.clone();
                if self.expanded.contains(&key) {
                    self.expanded.remove(&key);
                } else {
                    self.expanded.insert(key);
                }
            }
        }
    }

    fn set_sort(&mut self, col: SortColumn) {
        if self.sort_column == col {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = col;
            self.sort_ascending = matches!(col, SortColumn::Product | SortColumn::Code | SortColumn::Account);
        }
        self.table_state.select(Some(0));
    }

    fn prev_month(&mut self) {
        self.billing_cycle = Self::offset_month(&self.billing_cycle, -1);
        self.start_loading_all();
    }

    fn next_month(&mut self) {
        self.billing_cycle = Self::offset_month(&self.billing_cycle, 1);
        self.start_loading_all();
    }

    /// Build previous month cost lookup: product_code -> (cost, count)
    fn prev_cost_map(&self) -> BTreeMap<String, (f64, Option<u32>)> {
        let mut map = BTreeMap::new();
        let tab_name = &self.providers[self.active_tab];
        let iter: Box<dyn Iterator<Item = &ProviderDataState>> = if tab_name == "All" {
            Box::new(self.prev_data.values())
        } else if let Some(state) = self.prev_data.get(tab_name) {
            Box::new(std::iter::once(state))
        } else {
            return map;
        };
        for state in iter {
            if let ProviderDataState::Loaded(data) = state {
                for p in &data.products {
                    let entry = map.entry(p.product_code.clone()).or_insert((0.0, None));
                    entry.0 += p.cost;
                    if let Some(c) = p.count {
                        entry.1 = Some(entry.1.unwrap_or(0) + c);
                    }
                }
            }
        }
        map
    }

    /// Get currently visible products with MoM change, applying search filter
    pub fn current_products(&self) -> Vec<ProductRow> {
        let raw = self.raw_products();
        let prev_map = self.prev_cost_map();

        let query = self.search_query.to_lowercase();

        let mut rows: Vec<ProductRow> = raw.into_iter()
            .filter(|p| {
                if query.is_empty() {
                    return true;
                }
                p.product_name.to_lowercase().contains(&query)
                    || p.product_code.to_lowercase().contains(&query)
                    || p.account_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .map(|p| {
                let prev = prev_map.get(&p.product_code);
                let prev_cost = prev.map(|(c, _)| *c);
                let prev_count = prev.and_then(|(_, c)| *c);
                let mom_change = prev_cost.map(|pv| {
                    if pv == 0.0 {
                        if p.cost > 0.0 { f64::INFINITY } else { 0.0 }
                    } else {
                        (p.cost - pv) / pv * 100.0
                    }
                });
                let count_change = match (p.count, prev_count) {
                    (Some(cur), Some(prev)) => Some(cur as i32 - prev as i32),
                    _ => None,
                };
                let regions_display = if p.regions.is_empty() {
                    "-".to_string()
                } else if p.regions.len() <= 2 {
                    p.regions.join(",")
                } else {
                    format!("{}+{}", p.regions[0], p.regions.len() - 1)
                };
                ProductRow {
                    product_name: p.product_name.clone(),
                    product_code: p.product_code.clone(),
                    cost: p.cost,
                    mom_change,
                    account_name: p.account_name.clone(),
                    count: p.count,
                    count_change,
                    regions_display,
                    region_details: p.region_details.clone(),
                    is_sub_row: false,
                }
            })
            .collect::<Vec<_>>();

        // Apply sort
        let asc = self.sort_ascending;
        rows.sort_by(|a, b| {
            let ord = match self.sort_column {
                SortColumn::Cost => a.cost.partial_cmp(&b.cost).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::MoM => {
                    let am = a.mom_change.unwrap_or(f64::NEG_INFINITY);
                    let bm = b.mom_change.unwrap_or(f64::NEG_INFINITY);
                    am.partial_cmp(&bm).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortColumn::Product => a.product_name.to_lowercase().cmp(&b.product_name.to_lowercase()),
                SortColumn::Code => a.product_code.to_lowercase().cmp(&b.product_code.to_lowercase()),
                SortColumn::Account => {
                    let aa = a.account_name.as_deref().unwrap_or("");
                    let ba = b.account_name.as_deref().unwrap_or("");
                    aa.to_lowercase().cmp(&ba.to_lowercase())
                }
            };
            if asc { ord } else { ord.reverse() }
        });

        // Insert expanded region sub-rows
        let mut expanded_rows = Vec::with_capacity(rows.len() * 2);
        for row in rows {
            let is_expanded = self.expanded.contains(&row.product_code);
            let details = row.region_details.clone();
            expanded_rows.push(row);
            if is_expanded {
                for rd in &details {
                    expanded_rows.push(ProductRow {
                        product_name: format!("  {} {}", "└", rd.region),
                        product_code: String::new(),
                        cost: rd.cost,
                        mom_change: None,
                        account_name: None,
                        count: Some(rd.count),
                        count_change: None,
                        regions_display: rd.region.clone(),
                        region_details: vec![],
                        is_sub_row: true,
                    });
                }
            }
        }
        expanded_rows
    }

    fn raw_products(&self) -> Vec<ProductCost> {
        let tab_name = &self.providers[self.active_tab];
        if tab_name == "All" {
            let mut all = Vec::new();
            for state in self.data.values() {
                if let ProviderDataState::Loaded(data) = state {
                    all.extend(data.products.clone());
                }
            }
            all.sort_by(|a, b| {
                b.cost
                    .partial_cmp(&a.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            all
        } else if let Some(ProviderDataState::Loaded(data)) = self.data.get(tab_name) {
            data.products.clone()
        } else {
            vec![]
        }
    }

    pub fn cost_summary(&self) -> Vec<(String, f64, String)> {
        let tab_name = &self.providers[self.active_tab];
        if tab_name == "All" {
            let mut summaries = Vec::new();
            for (provider, state) in &self.data {
                if let ProviderDataState::Loaded(data) = state {
                    summaries.push((provider.clone(), data.total_cost, data.currency.clone()));
                }
            }
            summaries.sort_by(|a, b| {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            summaries
        } else if let Some(ProviderDataState::Loaded(data)) = self.data.get(tab_name) {
            vec![(tab_name.clone(), data.total_cost, data.currency.clone())]
        } else {
            vec![]
        }
    }

    /// Stable-order totals by currency
    pub fn totals_by_currency(&self) -> Vec<(String, f64)> {
        let mut totals = BTreeMap::new();
        let tab_name = &self.providers[self.active_tab];
        if tab_name == "All" {
            for state in self.data.values() {
                if let ProviderDataState::Loaded(data) = state {
                    *totals.entry(data.currency.clone()).or_insert(0.0) += data.total_cost;
                }
            }
        } else if let Some(ProviderDataState::Loaded(data)) = self.data.get(tab_name) {
            *totals.entry(data.currency.clone()).or_insert(0.0) += data.total_cost;
        }
        totals.into_iter().collect()
    }

    /// Previous month totals for MoM on summary
    pub fn prev_totals_by_currency(&self) -> Vec<(String, f64)> {
        let mut totals = BTreeMap::new();
        let tab_name = &self.providers[self.active_tab];
        if tab_name == "All" {
            for state in self.prev_data.values() {
                if let ProviderDataState::Loaded(data) = state {
                    *totals.entry(data.currency.clone()).or_insert(0.0) += data.total_cost;
                }
            }
        } else if let Some(ProviderDataState::Loaded(data)) = self.prev_data.get(tab_name) {
            *totals.entry(data.currency.clone()).or_insert(0.0) += data.total_cost;
        }
        totals.into_iter().collect()
    }

    pub fn is_any_loading(&self) -> bool {
        self.data
            .values()
            .any(|s| matches!(s, ProviderDataState::Loading))
    }

    pub fn spinner_char(&self) -> char {
        const SPINNER: &[char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];
        SPINNER[self.spinner_tick % SPINNER.len()]
    }

    pub fn loading_errors(&self) -> Vec<(String, String)> {
        let mut errs: Vec<_> = self
            .data
            .iter()
            .filter_map(|(k, v)| {
                if let ProviderDataState::Error(e) = v {
                    Some((k.clone(), e.clone()))
                } else {
                    None
                }
            })
            .collect();
        errs.sort_by(|a, b| a.0.cmp(&b.0));
        errs
    }

    pub fn loaded_count(&self) -> usize {
        self.data
            .values()
            .filter(|s| matches!(s, ProviderDataState::Loaded(_)))
            .count()
    }

    pub fn total_providers(&self) -> usize {
        self.providers.len() - 1
    }
}

/// Product row with MoM comparison
#[derive(Clone)]
pub struct ProductRow {
    pub product_name: String,
    pub product_code: String,
    pub cost: f64,
    pub mom_change: Option<f64>,
    pub account_name: Option<String>,
    pub count: Option<u32>,
    pub count_change: Option<i32>,
    pub regions_display: String,
    pub region_details: Vec<cbilling::service::RegionDetail>,
    /// Is this a sub-row (region detail)?
    pub is_sub_row: bool,
}
