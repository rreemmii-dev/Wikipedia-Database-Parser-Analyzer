use crate::graph_utils::vec_graph_utils::VecGraph;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;

pub type HashmapGraph = HashMap<u32, HashSet<u32>>;

#[must_use]
pub fn hashmap_of_vec(vec: &VecGraph) -> HashmapGraph {
    vec.iter()
        .enumerate()
        .map(|(id, parents)| {
            (
                id as u32,
                parents.iter().map(|&v| v as u32).collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>()
}

#[must_use]
pub fn filtered_hashmap_of_vec(graph: &VecGraph, kept_vertexes: &HashSet<usize>) -> HashmapGraph {
    graph
        .iter()
        .enumerate()
        .filter(|(id, _)| kept_vertexes.contains(id))
        .map(|(id, parents)| {
            (
                id as u32,
                parents
                    .iter()
                    .filter(|&v| kept_vertexes.contains(v))
                    .map(|&v| v as u32)
                    .collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>()
}

pub fn remove_vertex(vertex: u32, graph: &mut HashmapGraph, reversed: &mut HashmapGraph) -> bool {
    if !graph.contains_key(&vertex) {
        return false;
    }
    let parents = graph.remove(&vertex).unwrap();
    let children = reversed.remove(&vertex).unwrap();
    for parent in parents {
        reversed.get_mut(&parent).unwrap().remove(&vertex);
    }
    for child in children {
        graph.get_mut(&child).unwrap().remove(&vertex);
    }
    true
}

pub fn export_as_csv(
    graph: &HashmapGraph,
    name_from_id: Option<&Vec<String>>,
    separator: &str,
    path: &str,
) {
    let write_item = |id: usize, content: &mut String| {
        if let Some(name_from_id) = name_from_id {
            assert!(!name_from_id[id].contains(separator));
            content.push_str(&name_from_id[id]);
        } else {
            content.push_str(&id.to_string());
        }
    };
    let mut file = File::create(path).unwrap();
    let mut is_first_line = true;
    for (&id, parents) in graph {
        let mut content = String::new();
        if is_first_line {
            is_first_line = false;
        } else {
            content += "\n";
        }
        write_item(id as usize, &mut content);
        content += separator;
        for &parent in parents {
            write_item(parent as usize, &mut content);
            content += separator;
        }
        content.pop(); // Removes trailing separator
        file.write_all(content.as_bytes()).unwrap();
    }
}
