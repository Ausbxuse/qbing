//! Explicit, lossless-source import; never reads a browser profile.
use crate::cube::{Cube, sequence};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
pub fn records(input: &Value) -> Result<Vec<Value>> {
    if let Some(records) = input["records"].as_array() {
        for r in records {
            ensure!(
                r["id"].as_str().is_some_and(|s| !s.is_empty())
                    && r["duration"]
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && n >= 0.),
                "Invalid native record"
            );
        }
        return Ok(records.clone());
    }
    let object = input
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected a native or csTimer JSON backup"))?;
    let mut output = vec![];
    for (key, session) in object.iter().filter(|(k, _)| {
        k.strip_prefix("session")
            .is_some_and(|n| n.parse::<usize>().is_ok())
    }) {
        let session = if let Some(s) = session.as_str() {
            serde_json::from_str(s)?
        } else {
            session.clone()
        };
        let rows = session
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid csTimer session"))?;
        for (index, row) in rows.iter().enumerate() {
            let duration = row[0][1]
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.)
                .ok_or_else(|| anyhow::anyhow!("Invalid recorded time"))?;
            let scramble = row[1].as_str().unwrap_or("");
            let initial = if scramble.trim().is_empty() {
                Cube::default()
            } else {
                Cube::default().apply(&sequence(scramble)?)
            };
            let raw = row[4][0].as_str().unwrap_or("");
            let mut moves = vec![];
            for token in raw.split_whitespace() {
                let Some((mv, time)) = token.rsplit_once('@') else {
                    continue;
                };
                let t: f64 = time.parse()?;
                ensure!(t.is_finite() && t >= 0., "Invalid turn timestamp");
                crate::cube::Move::parse(mv)?;
                moves.push(json!({"move":mv,"t":t,"raw":{}}));
            }
            let penalty = match row[0][0].as_i64().unwrap_or(0) {
                -1 => "DNF",
                2000 => "+2",
                _ => "",
            };
            let epoch = row[3].as_i64().unwrap_or(0);
            let created = chrono::DateTime::from_timestamp(epoch, 0)
                .unwrap_or_default()
                .to_rfc3339();
            let source = if moves.is_empty() {
                "manual"
            } else {
                "bluetooth"
            };
            output.push(json!({"id":format!("cstimer-{key}-{index}-{epoch}"),"createdAt":created,"kind":"solve","source":source,"status":"complete","penalty":penalty,"duration":duration,"initialState":initial.to_string(),"cross":"U","session":format!("Imported {}",key),"stage":"solve","scramble":scramble,"moves":moves,"quality":[],"notes":row[2].as_str().unwrap_or(""),"localReview":row[5]["localReview"],"importedSource":row}));
        }
    }
    ensure!(
        !output.is_empty(),
        "No supported 3×3 sessions found in this JSON backup"
    );
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cstimer_import_preserves_source_penalties_tags_and_recorded_turns() {
        let row = json!([[2000,3456],"R","Legacy",1700000000,["R'@3456","333"],{"localReview":{"favorite":true,"color":"blue"}}]);
        let r = records(&json!({"session1":[row.clone()]}))
            .unwrap()
            .remove(0);
        assert_eq!(r["duration"], 3456.);
        assert_eq!(r["penalty"], "+2");
        assert_eq!(r["importedSource"], row);
        assert_eq!(r["localReview"]["color"], "blue");
        assert!(r["moves"][0]["raw"]["cubeTimestamp"].is_null());
        assert_eq!(records(&json!({"records":[r.clone()]})).unwrap()[0], r);
    }
}
