//! Same nice-bin and population-statistics rules as local-review-data.js.
use serde_json::Value;
pub struct Distribution {
    pub start: f64,
    pub step: f64,
    pub bins: Vec<usize>,
    pub count: usize,
    pub median: Option<f64>,
    pub spread: Option<f64>,
    pub p90: Option<f64>,
    pub dnf_rate: Option<f64>,
}
pub fn distribution(records: &[Value]) -> Distribution {
    distribution_target(records, 8)
}
fn distribution_target(records: &[Value], target: usize) -> Distribution {
    let mut values = records
        .iter()
        .map(crate::io::adjusted)
        .filter(|v| v.is_finite() && *v >= 0.)
        .collect::<Vec<_>>();
    distribution_values(values, records.len(), target, 100.)
}
fn distribution_values(
    mut values: Vec<f64>,
    total: usize,
    target: usize,
    min_step: f64,
) -> Distribution {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    let mut out = Distribution {
        start: 0.,
        step: 0.,
        bins: vec![],
        count: n,
        median: None,
        spread: None,
        p90: None,
        dnf_rate: (total > 0).then(|| total.saturating_sub(n) as f64 / total as f64),
    };
    if n == 0 {
        return out;
    }
    let low = values[0];
    let high = values[n - 1];
    let raw = ((high - low) / target.max(1) as f64).max(min_step);
    let power = 10_f64.powf(raw.log10().floor());
    let step = [1., 2., 5., 10.]
        .into_iter()
        .find(|m| m * power >= raw)
        .unwrap()
        * power;
    let start = (low / step).floor() * step;
    out.start = start;
    out.step = step;
    out.bins = vec![0; ((high - start) / step).floor() as usize + 1];
    for v in &values {
        out.bins[((v - start) / step).floor() as usize] += 1;
    }
    out.median = Some((values[(n - 1) / 2] + values[n / 2]) / 2.);
    out.p90 = Some(values[(n as f64 * 0.9).ceil() as usize - 1]);
    if n > 1 {
        let mean = values.iter().sum::<f64>() / n as f64;
        out.spread =
            Some((values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt());
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn nice_bins_match_the_web_reference() {
        let records = (0..20)
            .map(|i| json!({"status":"complete","duration":10500+i*170}))
            .collect::<Vec<_>>();
        let d = distribution(&records);
        assert_eq!(d.step, 500.);
        assert_eq!(d.bins, vec![3, 3, 3, 3, 3, 3, 2]);
        assert_eq!(d.median, Some(12115.));
        assert_eq!(d.p90, Some(13390.));
    }
}

/// Keep numeric intervals honest; coarsen sparse runs instead of deleting zeros.
pub fn histogram(records: &[Value], width: f32) -> Distribution {
    let values = records
        .iter()
        .map(crate::io::adjusted)
        .filter(|v| v.is_finite() && *v >= 0.)
        .collect::<Vec<_>>();
    histogram_values(values, records.len(), width, 100.)
}
pub fn histogram_values(values: Vec<f64>, total: usize, width: f32, min_step: f64) -> Distribution {
    let count = values.len();
    let target = ((width / 44.).floor() as usize)
        .clamp(2, 12)
        .min((count as f64).sqrt().ceil().max(2.) as usize);
    let mut d = distribution_values(values, total, target, min_step);
    loop {
        let mut run = 0;
        let mut longest = 0;
        for n in &d.bins {
            if *n == 0 {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        if d.bins.len() < 3 || longest == 0 || longest as f32 * width / d.bins.len() as f32 <= 32. {
            break;
        }
        let step = d.step * 2.;
        let start = (d.start / step).floor() * step;
        let last = d.start + d.step * (d.bins.len() - 1) as f64;
        let mut bins = vec![0; ((last - start) / step).floor() as usize + 1];
        for (i, n) in d.bins.iter().enumerate() {
            bins[((d.start + d.step * i as f64 - start) / step).floor() as usize] += n;
        }
        d.start = start;
        d.step = step;
        d.bins = bins;
    }
    d
}
#[cfg(test)]
mod adaptive_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn adaptive_bins_keep_every_sample_and_coarsen_sparse_runs() {
        let r = [10000, 10100, 10200, 10500, 11000, 90000]
            .map(|t| json!({"status":"complete","duration":t}));
        for width in [240., 400., 1200.] {
            let d = histogram(&r, width);
            assert_eq!(d.bins.iter().sum::<usize>(), 6);
            assert_eq!(d.median, distribution(&r).median);
            assert!(d.bins.len() <= 12);
            let mut gap = 0;
            for n in d.bins {
                gap = if n == 0 { gap + 1 } else { 0 };
                assert!(gap <= 1);
            }
        }
    }
}

/// Rolling WCA-style ao5 in original session order, independent of view filters.
pub fn rolling_ao5(records: &[Value]) -> Vec<Option<f64>> {
    let mut out = vec![None; records.len()];
    for (i, window) in records.windows(5).enumerate() {
        let mut times = [0.; 5];
        for (slot, record) in times.iter_mut().zip(window) {
            *slot = crate::io::adjusted(record);
        }
        times.sort_by(f64::total_cmp);
        out[i + 4] = Some(times[1..4].iter().sum::<f64>() / 3.);
    }
    out
}
#[cfg(test)]
mod rolling_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rolling_windows_trim_one_extreme_at_each_end_and_preserve_penalties() {
        let mut r = (10..=15)
            .map(|t| json!({"status":"complete","duration":t*1000}))
            .collect::<Vec<_>>();
        assert_eq!(
            rolling_ao5(&r),
            vec![None, None, None, None, Some(12000.), Some(13000.)]
        );
        r[2]["penalty"] = json!("+2");
        assert_eq!(rolling_ao5(&r)[4], Some(38000. / 3.));
        r[2]["penalty"] = json!("DNF");
        assert_eq!(rolling_ao5(&r)[4], Some(38000. / 3.));
        r[3]["penalty"] = json!("DNF");
        assert!(rolling_ao5(&r)[4].unwrap().is_infinite());
    }
}
