//! Offline drill evidence. Raw turns and recorded cue timing remain authoritative.
use crate::{
    cube::Move,
    io::Case,
    io::{number, value_str},
    recording,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn normalized<'a>(moves: impl Iterator<Item = &'a str>) -> Option<Vec<Move>> {
    let mut out: Vec<Move> = Vec::new();
    for text in moves {
        let mv = Move::parse(text).ok()?;
        if let Some(last) = out.last_mut().filter(|last| last.face == mv.face) {
            last.turns = (last.turns + mv.turns).rem_euclid(4);
            if last.turns == 0 {
                out.pop();
            }
        } else {
            out.push(mv);
        }
    }
    Some(out)
}
pub fn local_performance(records: &[Value], case: &Case) -> Option<Value> {
    let mut groups: BTreeMap<String, (Option<usize>, Vec<Value>)> = BTreeMap::new();
    for r in records.iter().filter(|r| {
        value_str(r, "caseId") == case.id
            && value_str(r, "kind") == "drill"
            && crate::io::timed(r)
            && value_str(r, "source") == "bluetooth"
            && !recording::execution_duration_estimated(r)
    }) {
        let Some(moves) = r["moves"].as_array() else {
            continue;
        };
        if moves.is_empty() {
            continue;
        }
        let Some(raw) = normalized(moves.iter().map(|m| value_str(m, "move"))) else {
            continue;
        };
        let cross = value_str(r, "cross");
        let variant = case.variants.iter().position(|v| {
            v.positions.get(cross).is_some_and(|p| {
                normalized(p.solution.iter().map(String::as_str)) == Some(raw.clone())
            })
        });
        let key = variant
            .map(|n| case.variants[n].id.clone())
            .unwrap_or_else(|| {
                moves
                    .iter()
                    .map(|m| value_str(m, "move"))
                    .collect::<Vec<_>>()
                    .join(" ")
            });
        let recognition = (value_str(r, "recognitionSource") == "cue")
            .then(|| r["recognitionMs"].as_f64())
            .flatten()
            .filter(|n| n.is_finite() && *n >= 0.);
        groups.entry(key).or_insert_with(||(variant,Vec::new())).1.push(json!({"recordId":r["id"],"createdAt":r["createdAt"],"execution":number(r,"duration"),"recognition":recognition,"recognitionKind":if recognition.is_some(){"cue"}else{"none"}}));
    }
    if groups.is_empty() {
        return None;
    }
    let mut count = 0;
    let algorithms=groups.into_iter().map(|(id,(variant,samples))|{
  let mut times=samples.iter().map(|s|number(s,"execution")).collect::<Vec<_>>();times.sort_by(f64::total_cmp);count+=samples.len();
  let rec=samples.iter().filter_map(|s|s["recognition"].as_f64()).collect::<Vec<_>>();
  json!({"id":id,"caseId":case.id,"name":variant.map(|n|format!("Variant {}",n+1)).unwrap_or("Recorded sequence".into()),"variant":variant.map(|n|n+1),"algorithm":variant.map(|n|case.variants[n].text.clone()).unwrap_or(id),"count":samples.len(),"meanExecution":times.iter().sum::<f64>()/times.len() as f64,"medianExecution":(times[(times.len()-1)/2]+times[times.len()/2])/2.,"meanRecognition":(!rec.is_empty()).then(||rec.iter().sum::<f64>()/rec.len() as f64),"recognitionCount":rec.len(),"samples":samples})
 }).collect::<Vec<_>>();
    Some(
        json!({"caseId":case.id,"name":case.name,"stage":case.stage,"count":count,"algorithms":algorithms}),
    )
}

pub fn summary(performance: &Value) -> Value {
    let algorithms = performance["algorithms"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let total = algorithms.iter().map(|a| number(a, "count")).sum::<f64>();
    let recognized = algorithms
        .iter()
        .map(|a| number(a, "recognitionCount"))
        .sum::<f64>();
    let mut samples = algorithms
        .iter()
        .flat_map(|a| a["samples"].as_array().into_iter().flatten().cloned())
        .collect::<Vec<_>>();
    samples.sort_by(|a, b| value_str(a, "createdAt").cmp(value_str(b, "createdAt")));
    json!({"count":total,"meanExecution":(total>0.).then(||algorithms.iter().map(|a|number(a,"meanExecution")*number(a,"count")).sum::<f64>()/total),"meanRecognition":(recognized>0.).then(||algorithms.iter().map(|a|number(a,"meanRecognition")*number(a,"recognitionCount")).sum::<f64>()/recognized),"recognitionCount":recognized,"samples":samples})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_analysis_uses_observed_algorithms_and_only_measured_cue_recognition() {
        let catalog = crate::io::catalog();
        let c = catalog.cases.iter().find(|c| c.id == "f2l-1").unwrap();
        let p = &c.variants[0].positions["D"];
        let moves = p
            .solution
            .iter()
            .enumerate()
            .map(|(i, m)| json!({"move":m,"t":i*200}))
            .collect::<Vec<_>>();
        let r = json!({"id":"a","createdAt":"2026-01-01T00:00:00Z","kind":"drill","source":"bluetooth","status":"complete","penalty":"","cross":"D","caseId":c.id,"selectedAlgorithmId":"unrelated","initialState":p.state,"duration":600.,"moves":moves,"recognitionSource":"cue","recognitionMs":250.,"quality":[]});
        let mut transition = r.clone();
        transition["id"] = json!("b");
        transition["recognitionSource"] = json!("transition");
        transition["recognitionMs"] = json!(9999.);
        let mut dnf = r.clone();
        dnf["penalty"] = json!("DNF");
        let perf = local_performance(&[r, transition, dnf], c).unwrap();
        let sum = summary(&perf);
        assert_eq!(sum["count"], 2.);
        assert_eq!(sum["recognitionCount"], 1.);
        assert_eq!(sum["meanRecognition"], 250.);
        assert_eq!(sum["meanExecution"], 600.);
        assert_eq!(perf["algorithms"][0]["variant"], 1);
    }
}
