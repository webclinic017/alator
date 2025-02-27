use itertools::Itertools;

use super::{BrokerRecordedEvent, DividendPayment, Trade, TradeType};
use crate::types::{CashValue, DateTime, PortfolioQty, Price};

///Records certain events executed by the broker.
///
///This is required for some internal calculations, such as the cost basis of positions, but also
///should be public to clients for tax calculations.
#[derive(Clone, Debug)]
pub struct BrokerLog {
    log: Vec<BrokerRecordedEvent>,
}

impl BrokerLog {
    pub fn record<E: Into<BrokerRecordedEvent>>(&mut self, event: E) {
        let brokerevent: BrokerRecordedEvent = event.into();
        self.log.push(brokerevent);
    }

    pub fn trades(&self) -> Vec<Trade> {
        let mut trades = Vec::new();
        for event in &self.log {
            if let BrokerRecordedEvent::TradeCompleted(trade) = event {
                trades.push(trade.clone());
            }
        }
        trades
    }

    pub fn dividends(&self) -> Vec<DividendPayment> {
        let mut dividends = Vec::new();
        for event in &self.log {
            if let BrokerRecordedEvent::DividendPaid(dividend) = event {
                dividends.push(dividend.clone());
            }
        }
        dividends
    }

    pub fn dividends_between(&self, start: &i64, stop: &i64) -> Vec<DividendPayment> {
        let dividends = self.dividends();
        dividends
            .iter()
            .filter(|v| v.date >= DateTime::from(*start) && v.date <= DateTime::from(*stop))
            .cloned()
            .collect_vec()
    }

    pub fn trades_between(&self, start: &i64, stop: &i64) -> Vec<Trade> {
        let trades = self.trades();
        trades
            .iter()
            .filter(|v| v.date >= DateTime::from(*start) && v.date <= DateTime::from(*stop))
            .cloned()
            .collect_vec()
    }

    pub fn cost_basis(&self, symbol: &str) -> Option<Price> {
        let mut cum_qty = PortfolioQty::default();
        let mut cum_val = CashValue::default();
        for event in &self.log {
            if let BrokerRecordedEvent::TradeCompleted(trade) = event {
                if trade.symbol.eq(symbol) {
                    match trade.typ {
                        TradeType::Buy => {
                            cum_qty = PortfolioQty::from(*cum_qty + *trade.quantity.clone());
                            cum_val = CashValue::from(*cum_val + *trade.value.clone());
                        }
                        TradeType::Sell => {
                            cum_qty = PortfolioQty::from(*cum_qty - *trade.quantity.clone());
                            cum_val = CashValue::from(*cum_val - *trade.value.clone());
                        }
                    }
                    //reset the value if we are back to zero
                    if (*cum_qty).eq(&0.0) {
                        cum_val = CashValue::default();
                    }
                }
            }
        }
        if (*cum_qty).eq(&0.0) {
            return None;
        }
        Some(Price::from(*cum_val / *cum_qty))
    }
}

impl BrokerLog {
    pub fn new() -> Self {
        BrokerLog { log: Vec::new() }
    }
}

impl Default for BrokerLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::BrokerLog;

    use crate::broker::{Trade, TradeType};

    fn setup() -> BrokerLog {
        let mut rec = BrokerLog::new();

        let t1 = Trade::new("ABC", 100.0, 10.00, 100, TradeType::Buy);
        let t2 = Trade::new("ABC", 500.0, 90.00, 101, TradeType::Buy);
        let t3 = Trade::new("BCD", 100.0, 100.0, 102, TradeType::Buy);
        let t4 = Trade::new("BCD", 500.0, 100.00, 103, TradeType::Sell);
        let t5 = Trade::new("BCD", 50.0, 50.00, 104, TradeType::Buy);

        rec.record(t1);
        rec.record(t2);
        rec.record(t3);
        rec.record(t4);
        rec.record(t5);
        rec
    }

    #[test]
    fn test_that_log_filters_trades_between_dates() {
        let log = setup();
        let between = log.trades_between(&102.into(), &104.into());
        assert!(between.len() == 3);
    }

    #[test]
    fn test_that_log_calculates_the_cost_basis() {
        let log = setup();
        let abc_cost = log.cost_basis("ABC").unwrap();
        let bcd_cost = log.cost_basis("BCD").unwrap();

        assert_eq!(*abc_cost, 6.0);
        assert_eq!(*bcd_cost, 1.0);
    }
}
