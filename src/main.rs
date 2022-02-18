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

struct ClosureComputationState<'a> {
    todo: HashMap<String, &'a PathInfo>,
    results: HashMap<String, HashSet<String>>,
}

fn strip_name(path: &String) -> String {
    let mut stripped = path.clone();
    stripped.truncate("/nix/store/00000000000000000000000000000000".len());
    stripped
}

fn get_path<'a, T>(map: &'a HashMap<String, T>, path: &String) -> Option<&'a T> {
    map.get(path).or_else(|| map.get(&strip_name(path)))
}

fn compute_closure<'a>(
    mut state: &'a mut ClosureComputationState,
    path: String,
) -> &'a HashSet<String> {
    let pathinfo = match state
        .todo
        .remove(&path)
        .or_else(|| state.todo.remove(&strip_name(&path)))
    {
        Some(info) => info,
        None => panic!("Could not find info for {}", path),
    };
    eprintln!("Computing closure for {}", path);
    let mut closure: HashSet<String> = HashSet::new();
    closure.insert(strip_name(&path));
    for reference in pathinfo.references.iter() {
        if *reference == path {
            continue;
        }
        let reference = strip_name(reference);
        if let Some(reference_closure) = get_path(&state.results, &reference) {
            for path in reference_closure {
                closure.insert(strip_name(path));
            }
        } else {
            for path in compute_closure(&mut state, reference.clone()) {
                closure.insert(strip_name(path));
            }
        }
    }
    state.results.insert(strip_name(&path), closure);
    get_path(&state.results, &path).unwrap()
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
        .map(|pathinfo| strip_name(&pathinfo.path))
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
        for path in get_path(&closures, &root).unwrap() {
            paths_to_delete.remove(path);
        }
    }

    for path in paths_to_delete {
        println!("{}", path);
    }

    Ok(())
}
