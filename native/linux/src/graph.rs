//! Metric-specific chart evidence. Missing sensor metrics retain their solve slots.
use crate::{io, native_stats, recording};
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) enum Metric {
    #[default]
    Time,
    Tps,
    Moves,
}
impl Metric {
    pub fn index(self) -> usize {
        match self {
            Self::Time => 0,
            Self::Tps => 1,
            Self::Moves => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Time => "Time",
            Self::Tps => "TPS",
            Self::Moves => "Moves",
        }
    }
    pub fn unit(self) -> &'static str {
        match self {
            Self::Time => "s",
            Self::Tps => "tps",
            Self::Moves => "moves",
        }
    }
    pub fn value(self, r: &Value) -> Option<f64> {
        let time = io::adjusted(r);
        if !time.is_finite() || time < 0. {
            return None;
        }
        match self {
            Self::Time => Some(time),
            Self::Tps => recording::execution_metrics(r).map(|m| m.tps),
            Self::Moves => recording::execution_moves(r).map(|m| m as f64),
        }
    }
    pub fn format(self, n: f64) -> String {
        match self {
            Self::Time => io::seconds(n),
            Self::Tps => format!("{n:.2}"),
            Self::Moves => {
                if n.fract().abs() < 0.001 {
                    format!("{n:.0}")
                } else {
                    format!("{n:.1}")
                }
            }
        }
    }
    pub fn axis(self, n: f64) -> String {
        match self {
            Self::Time => format!("{:.1}", n / 1000.),
            _ => self.format(n),
        }
    }
    pub fn minimum(self) -> f64 {
        match self {
            Self::Time => 100.,
            Self::Tps => 0.1,
            Self::Moves => 1.,
        }
    }
}
#[derive(Default)]
pub(crate) struct Series {
    pub slots: Vec<Option<f64>>,
    pub mean: Option<f64>,
    pub best: Option<f64>,
    pub total: usize,
    histograms: RefCell<BTreeMap<i32, Arc<native_stats::Distribution>>>,
}
impl Series {
    pub fn new(records: &[Value], metric: Metric) -> Self {
        let slots = records.iter().map(|r| metric.value(r)).collect::<Vec<_>>();
        let valid = slots.iter().flatten().copied().collect::<Vec<_>>();
        let mean = (!valid.is_empty()).then(|| valid.iter().sum::<f64>() / valid.len() as f64);
        let best = valid.into_iter().reduce(|a, b| {
            if metric == Metric::Tps {
                a.max(b)
            } else {
                a.min(b)
            }
        });
        Self {
            slots,
            mean,
            best,
            total: records.len(),
            ..Default::default()
        }
    }
    pub fn histogram(&self, metric: Metric, width: f32) -> Arc<native_stats::Distribution> {
        let key = width.round() as i32;
        let mut cache = self.histograms.borrow_mut();
        if let Some(d) = cache.get(&key) {
            return d.clone();
        }
        let d = Arc::new(native_stats::histogram_values(
            self.slots.iter().flatten().copied().collect(),
            self.total,
            width,
            metric.minimum(),
        ));
        if cache.len() >= 8 {
            cache.clear();
        }
        cache.insert(key, d.clone());
        d
    }
}
#[derive(Default)]
pub(crate) struct Cache {
    pub revision: u64,
    pub session: String,
    pub len: usize,
    pub series: [Arc<Series>; 3],
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn charts_preserve_missing_solve_slots_and_ignore_scoring_penalties_for_tps() {
        let sensor = |duration: f64, penalty: &str| json!({"source":"bluetooth","status":"complete","penalty":penalty,"duration":duration,"quality":[],"moves":[{"move":"R","t":0.},{"move":"U2","t":duration}]});
        let r = vec![
            sensor(1000., ""),
            json!({"source":"manual","status":"complete","duration":1500.}),
            sensor(2000., "+2"),
            sensor(1000., "DNF"),
        ];
        let tps = Series::new(&r, Metric::Tps);
        assert_eq!(tps.slots, vec![Some(2.), None, Some(1.), None]);
        assert_eq!(tps.best, Some(2.));
        assert_eq!(tps.mean, Some(1.5));
        let moves = Series::new(&r, Metric::Moves);
        assert_eq!(moves.slots, vec![Some(2.), None, Some(2.), None]);
        assert_eq!(moves.histogram(Metric::Moves, 300.).step, 1.);
        for metric in [Metric::Time, Metric::Tps, Metric::Moves] {
            let s = Series::new(&r, metric);
            let d = s.histogram(metric, 300.);
            assert_eq!(
                d.bins.iter().sum::<usize>(),
                s.slots.iter().flatten().count()
            );
        }
    }
}
