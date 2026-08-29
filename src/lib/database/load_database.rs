use crate::database::generate_database::WIKILINKS_SEPARATOR;
use crate::graph_utils::vec_graph_utils::VecGraph;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[must_use]
pub fn load_name_from_id_and_id_from_name(path: &str) -> (Vec<String>, HashMap<String, usize>) {
    let mut name_from_id = Vec::new();
    let mut id_from_name = HashMap::new();
    let file = BufReader::new(File::open(path).unwrap());
    for (id, name) in file.lines().enumerate() {
        let name = name.unwrap();
        name_from_id.push(name.clone());
        id_from_name.insert(name, id);
    }
    (name_from_id, id_from_name)
}

#[must_use]
pub fn load_graph(path: &str) -> VecGraph {
    let mut graph = Vec::new();
    let file = BufReader::new(File::open(path).unwrap());
    for line in file.lines() {
        let line = line.unwrap();
        let parents = if line.is_empty() {
            Vec::new()
        } else {
            line.split(WIKILINKS_SEPARATOR)
                .map(|v| v.parse::<usize>().unwrap())
                .collect::<Vec<_>>()
        };
        graph.push(parents);
    }
    graph
}
