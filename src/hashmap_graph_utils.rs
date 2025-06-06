use crate::vec_graph_utils::VecGraph;
use std::collections::{HashMap, HashSet};
use std::io::Write;

pub type HashmapGraph = HashMap<u32, HashSet<u32>>;

pub fn hashmap_of_vec(vec: &VecGraph) -> HashmapGraph {
    vec.iter()
        .enumerate()
        .map(|(id, neighbors)| {
            (
                id as u32,
                neighbors.iter().map(|&v| v as u32).collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>()
}

pub fn filtered_hashmap_of_vec(graph: &VecGraph, kept_vertexes: &HashSet<usize>) -> HashmapGraph {
    graph
        .iter()
        .enumerate()
        .filter(|(id, _)| kept_vertexes.contains(id))
        .map(|(id, neighbors)| {
            (
                id as u32,
                neighbors
                    .iter()
                    .filter(|&v| kept_vertexes.contains(v))
                    .map(|&v| v as u32)
                    .collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>()
}

pub fn remove_vertex(vertex: u32, graph: &mut HashmapGraph, reversed: &mut HashmapGraph) -> bool {
    if graph.contains_key(&vertex) {
        let sources = graph.remove(&vertex).unwrap();
        let children = reversed.remove(&vertex).unwrap();
        for source in sources {
            reversed.get_mut(&source).unwrap().remove(&vertex);
        }
        for child in children {
            graph.get_mut(&child).unwrap().remove(&vertex);
        }
        true
    } else {
        false
    }
}

pub fn export_as_csv(graph: &HashmapGraph, name_from_id: &Vec<String>, path: &str) {
    let mut file = std::fs::File::create(path).unwrap();
    for (&id, sources) in graph {
        let mut content = String::new();
        assert!(!name_from_id[id as usize].contains('#'));
        content += &name_from_id[id as usize];
        content += "#";
        for &source in sources {
            content += &name_from_id[source as usize];
            content += "#";
        }
        content.pop();
        content += "\n";
        file.write_all(content.as_bytes()).unwrap();
    }
}
