extern crate chrono;
extern crate serde;
extern crate serde_json;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Debug)]
pub struct PathInfo {
    pub path: String,
    pub references: HashSet<String>,
    pub registrationTime: u64,
}

enum Closure {
    Missing,
    Paths(HashSet<String>),
}

struct ClosureComputationState<'a> {
    todo: HashMap<String, &'a PathInfo>,
    results: HashMap<String, Closure>,
}

fn compute_closure<'a>(mut state: &'a mut ClosureComputationState, path: String) -> &'a Closure {
    let pathinfo = if let Some(info) = state.todo.remove(&path) {
        info
    } else {
        eprintln!("Missing path: {}", path);
        state.results.insert(path.clone(), Closure::Missing);
        return state.results.get(&path).unwrap();
    };
    let mut closure: HashSet<String> = HashSet::new();
    closure.insert(path.clone());
    for reference in pathinfo.references.iter() {
        if *reference == path {
            continue;
        }
        match state.results.get(reference) {
            Some(Closure::Paths(reference_closure)) => {
                for path in reference_closure {
                    closure.insert(path.clone());
                }
            }
            Some(Closure::Missing) => continue,
            None => match compute_closure(&mut state, reference.clone()) {
                Closure::Missing => continue,
                Closure::Paths(paths) => {
                    for path in paths {
                        closure.insert(path.clone());
                    }
                }
            },
        }
    }
    state.results.insert(path.clone(), Closure::Paths(closure));
    state.results.get(&path).unwrap()
}

fn main() -> Result<(), serde_json::Error> {
    let mut json: Vec<u8> = Vec::with_capacity(500 << 20);
    File::open("/scratch/store-info-with-registration-time.json")
        .unwrap()
        .read_to_end(&mut json)
        .unwrap();
    let mut deserializer = serde_json::Deserializer::from_slice(json.as_slice());
    let pathinfos: Vec<PathInfo> = Vec::deserialize(&mut deserializer)?;
    eprintln!("parsed info for {} paths", pathinfos.len());
    let paths: Vec<String> = pathinfos
        .iter()
        .map(|pathinfo| pathinfo.path.clone())
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

    let mut paths_to_delete: HashSet<String> = paths.iter().cloned().collect();
    for root in roots_to_keep {
        match closures.get(&root).unwrap() {
            Closure::Paths(paths) => {
                for path in paths {
                    paths_to_delete.remove(path);
                }
            }
            Closure::Missing => todo!(),
        }
    }

    for path in paths_to_delete {
        println!("{}", path);
    }

    Ok(())
}
