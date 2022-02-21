extern crate byte_unit;
extern crate chrono;
extern crate serde;
extern crate serde_json;

use byte_unit::Byte;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Read, Write};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct PathInfo {
    pub path: String,
    pub references: HashSet<String>,
    pub registrationTime: u64,
    pub downloadSize: u64,
    pub url: String,
}

struct ClosureComputationState<'a> {
    todo: HashMap<String, &'a PathInfo>,
    results: HashMap<String, HashSet<String>>,
}

fn compute_closure<'a>(
    mut state: &'a mut ClosureComputationState,
    path: String,
) -> &'a HashSet<String> {
    let pathinfo = match state.todo.remove(&path) {
        Some(info) => info,
        None => {
            eprintln!("Missing path {}", path);
            state.results.insert(path.clone(), HashSet::new());
            return state.results.get(&path).unwrap();
        }
    };
    let mut closure: HashSet<String> = HashSet::new();
    closure.insert(path.clone());
    for reference in pathinfo.references.iter() {
        if *reference == path {
            continue;
        }
        if let Some(reference_closure) = state.results.get(reference) {
            for path in reference_closure {
                closure.insert(path.clone());
            }
        } else {
            for path in compute_closure(&mut state, reference.clone()) {
                closure.insert(path.clone());
            }
        }
    }
    state.results.insert(path.clone(), closure);
    state.results.get(&path).unwrap()
}

fn compute_all_closures<'a>(mut state: &'a mut ClosureComputationState) {
    let mut last_reported_progress: usize = 0;
    while let Some(key) = state.todo.keys().next().cloned() {
        compute_closure(&mut state, key);
        let total_computed = state.results.len();
        if total_computed > last_reported_progress + 20 {
            eprint!(
                "\rComputed {}/{} closures...  ",
                total_computed,
                total_computed + state.todo.len()
            );
            last_reported_progress = total_computed;
        }
    }
}

fn main() -> Result<(), serde_json::Error> {
    let mut json: Vec<u8> = Vec::with_capacity(500 << 20);
    std::io::stdin().read_to_end(&mut json).unwrap();
    let mut deserializer = serde_json::Deserializer::from_slice(json.as_slice());
    let mut pathinfos: Vec<PathInfo> = Vec::deserialize(&mut deserializer)?;
    eprintln!("parsed info for {} paths", pathinfos.len());
    let pathinfos: HashMap<String, PathInfo> = pathinfos
        .drain(..)
        .map(|pathinfo| (pathinfo.path.clone(), pathinfo))
        .collect();
    let nars: HashMap<String, u64> = pathinfos
        .values()
        .map(|pathinfo| (pathinfo.url.clone(), pathinfo.downloadSize))
        .collect();

    let mut closure_computation_state = ClosureComputationState {
        todo: pathinfos.values().map(|pi| (pi.path.clone(), pi)).collect(),
        results: HashMap::new(),
    };
    eprintln!("");
    compute_all_closures(&mut closure_computation_state);
    let closures = closure_computation_state.results;

    let days_to_keep = std::env::args()
        .skip(1)
        .next()
        .as_ref()
        .map(|arg| {
            i64::from_str_radix(arg, 10).unwrap_or_else(|_| {
                eprintln!("Warning: could not parse argument, using default of 90 days");
                90
            })
        })
        .filter(|n| *n > 0)
        .unwrap_or(90);
    let cutoff_date = Utc::now() - Duration::days(days_to_keep);
    let cutoff_timestamp = cutoff_date.timestamp() as u64;
    let roots_to_keep: HashSet<String> = pathinfos
        .values()
        .filter(|pathinfo| pathinfo.registrationTime > cutoff_timestamp)
        .map(|pathinfo| pathinfo.path.clone())
        .collect();

    let mut paths_to_delete: HashSet<String> = pathinfos.keys().cloned().collect();
    let mut nars_to_delete = nars.clone();
    for root in roots_to_keep {
        for path in closures.get(&root).unwrap() {
            paths_to_delete.remove(path);
            nars_to_delete.remove(&pathinfos.get(path).unwrap().url);
        }
    }

    eprintln!(
        "Will delete {}/{} paths and {}/{} nar files, totalling {}.",
        paths_to_delete.len(),
        pathinfos.len(),
        nars_to_delete.len(),
        nars.len(),
        Byte::from_bytes(nars_to_delete.values().sum::<u64>()).get_appropriate_unit(true),
    );

    let output_file = std::io::stdout();
    let mut writer = BufWriter::new(output_file);
    for path in paths_to_delete {
        let mut hash = path
            .clone()
            .strip_prefix("/nix/store/")
            .map(String::from)
            .unwrap_or(path);
        hash.truncate(32);
        hash += ".narinfo\n";
        writer.write(hash.as_bytes()).unwrap();
    }
    for nar in nars_to_delete.keys() {
        writer.write(nar.as_bytes()).unwrap();
        writer.write(&[b'\n']).unwrap();
    }

    Ok(())
}
