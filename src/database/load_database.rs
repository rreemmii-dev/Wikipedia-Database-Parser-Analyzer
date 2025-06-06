use crate::database::generate_database::{TITLE_SEPARATOR, WIKILINKS_SEPARATOR};
use crate::vec_graph_utils::VecGraph;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn load_name_id_databases(path: &str) -> (Vec<String>, HashMap<String, usize>) {
    let mut id_from_name = HashMap::new();
    let mut name_from_id = Vec::new();
    let file = BufReader::new(File::open(path).unwrap());
    for (id, line) in file.lines().map(|l| l.unwrap()).enumerate() {
        let name = line.trim().split(TITLE_SEPARATOR).nth(1).unwrap();
        id_from_name.insert(name.to_string(), id);
        name_from_id.insert(id, name.to_string());
    }
    (name_from_id, id_from_name)
}

pub fn load_graph_database(path: &str) -> VecGraph {
    let mut graph = Vec::new();
    let file = BufReader::new(File::open(path).unwrap());
    for (id, line) in file.lines().map(|l| l.unwrap()).enumerate() {
        let values = line.trim().split(TITLE_SEPARATOR).nth(1).unwrap();
        let val = if values.is_empty() {
            Vec::new()
        } else {
            values
                .split(WIKILINKS_SEPARATOR)
                .map(|v| v.parse::<usize>().unwrap())
                .collect::<Vec<_>>()
        };
        graph.insert(id, val);
    }
    graph
}
