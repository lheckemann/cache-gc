extern crate chrono;
extern crate serde;
extern crate serde_json;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;

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
        None => panic!("Could not find info for {}", path),
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

fn main() -> Result<(), serde_json::Error> {
    let mut json: Vec<u8> = Vec::with_capacity(500 << 20);
    File::open("/scratch/store-info-with-registration-time.json")
        .unwrap()
        .read_to_end(&mut json)
        .unwrap();
    let mut deserializer = serde_json::Deserializer::from_slice(json.as_slice());
    let mut pathinfos: Vec<PathInfo> = Vec::deserialize(&mut deserializer)?;
    eprintln!("parsed info for {} paths", pathinfos.len());
    let paths: HashMap<String, PathInfo> = pathinfos
        .drain(..)
        .map(|pathinfo| (pathinfo.path.clone(), pathinfo))
        .collect();
    let nars: HashMap<String, u64> = pathinfos
        .iter()
        .map(|pathinfo| (pathinfo.url.clone(), pathinfo.downloadSize))
        .collect();

    let mut closure_computation_state = ClosureComputationState {
        todo: pathinfos.iter().map(|pi| (pi.path.clone(), pi)).collect(),
        results: HashMap::new(),
    };
    while !closure_computation_state.todo.is_empty() {
        let key = closure_computation_state
            .todo
            .keys()
            .next()
            .expect("todo is supposed to be non-empty!")
            .clone();
        compute_closure(&mut closure_computation_state, key);
    }
    let closures = closure_computation_state.results;

    let cutoff_date = Utc::now() - Duration::days(10);
    let cutoff_timestamp = cutoff_date.timestamp() as u64;
    let roots_to_keep: HashSet<String> = pathinfos
        .iter()
        .filter(|pathinfo| pathinfo.registrationTime > cutoff_timestamp)
        .map(|pathinfo| pathinfo.path.clone())
        .collect();

    let mut paths_to_delete: HashSet<String> = paths.keys().cloned().collect();
    let mut nars_to_delete: HashMap<String, u64> = nars;
    for root in roots_to_keep {
        for path in closures.get(&root).unwrap() {
            paths_to_delete.remove(path);
            nars_to_delete.remove(&paths.get(path).unwrap().url);
        }
    }

    println!(
        "Will delete {} paths and {} nar files, totalling {} bytes.",
        paths_to_delete.len(),
        nars_to_delete.len(),
        nars_to_delete.values().sum::<u64>()
    );

    Ok(())
}
