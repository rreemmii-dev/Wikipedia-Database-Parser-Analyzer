mod hashmap_graph_utils;
mod vec_graph_utils;
mod database {
    pub mod generate_database;
    pub mod load_database;
}

use crate::database::generate_database::generate_databases;
use crate::database::load_database::{load_graph_database, load_name_id_databases};
use crate::hashmap_graph_utils::{export_as_csv, filtered_hashmap_of_vec};
use crate::vec_graph_utils::{bfs, count_children, count_sources};
use std::collections::HashSet;
use std::time::Instant;

const WIKI_PATH: &str = "../../wikipedia_database/enwiki-20250501-pages-articles-multistream.xml";
const NAME_FROM_ID_PATH: &str = "data/name_from_id.txt";
const GRAPH_PATH: &str = "data/graph.txt";
const FILTERED_CSV_PATH: &str = "data/filtered.csv";

fn main() {
    let mut t0 = Instant::now();

    generate_databases(WIKI_PATH, GRAPH_PATH, NAME_FROM_ID_PATH);
    println!("Database generation: {:?}\n", t0.elapsed());
    t0 = Instant::now();

    let graph = load_graph_database(GRAPH_PATH);
    println!("Graph len: {}", graph.len());
    println!("Graph load: {:?}\n", t0.elapsed());
    t0 = Instant::now();

    let (name_from_id, id_from_name) = load_name_id_databases(NAME_FROM_ID_PATH);
    println!("Name from id load: {:?}\n", t0.elapsed());
    t0 = Instant::now();

    let path = bfs(
        id_from_name["Wikipedia"],
        id_from_name["Rust_(programming_language)"],
        &graph,
    )
    .unwrap();
    println!(
        "Shortest path from `Wikipedia` to `Rust_(programming_language)`: {} clicks",
        path.len() - 1
    );
    for &id in &path {
        println!("> {}", name_from_id[id]);
    }
    println!("BFS algorithm: {:?}\n", t0.elapsed());
    t0 = Instant::now();

    let sources = count_sources(&graph);
    let children = count_children(&graph);
    let min_nb_sources = 1000;
    let min_nb_children = 1000;
    let kept_vertexes = (0..graph.len())
        .filter(|&v| sources[v] > min_nb_sources || children[v] > min_nb_children)
        .collect::<HashSet<_>>();
    let filtered_graph = filtered_hashmap_of_vec(&graph, &kept_vertexes);
    println!("Filtered graph len: {}", filtered_graph.len(),);
    println!("Filtered graph generation: {:?}\n", t0.elapsed());
    t0 = Instant::now();

    export_as_csv(&filtered_graph, &name_from_id, FILTERED_CSV_PATH);
    println!("CSV export: {:?}", t0.elapsed());
}
