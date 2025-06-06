mod hashmap_graph_utils;
mod vec_graph_utils;
mod database {
    pub mod generate_database;
    pub mod load_database;
}

use crate::database::generate_database::generate_databases;
use crate::database::load_database::{load_graph_database, load_name_id_databases};
use crate::hashmap_graph_utils::{HashmapGraph, export_as_csv, filtered_hashmap_of_vec};
use crate::vec_graph_utils::{VecGraph, bfs, count_children, count_sources};
use std::collections::HashSet;
use std::time::Instant;

const WIKI_PATH: &str = "../../wikipedia_database/enwiki-20250501-pages-articles-multistream.xml";
const NAME_FROM_ID_PATH: &str = "data/name_from_id.txt";
const GRAPH_PATH: &str = "data/graph.txt";
const FILTERED_CSV_PATH: &str = "data/filtered.csv";

fn page_rank_iteration(graph: &VecGraph, prev_res: &Vec<f64>) -> Vec<f64> {
    let mut res = vec![0.; prev_res.len()];
    for (id, sources) in graph.iter().enumerate() {
        res[id] += 0.5 * prev_res[id];
        let nb_sources = sources.len() as f64;
        for &source in sources {
            res[source] += 0.5 * prev_res[id] / nb_sources;
        }
    }
    res
}

fn run_n_page_rank_iterations(n: u64, graph: &VecGraph) -> Vec<f64> {
    let mut res = vec![1.0; graph.len()];
    for _ in 0..n {
        res = page_rank_iteration(graph, &res);
    }
    res
}

fn filtered_graph_by_score(
    graph: &VecGraph,
    scores: &Vec<f64>,
    number_vertexes: usize,
) -> HashmapGraph {
    let mut sorted_score = scores.iter().enumerate().collect::<Vec<_>>();
    sorted_score.sort_by(|a, b| f64::partial_cmp(b.1, a.1).unwrap()); // Reversed sorting
    let kept_vertexes = sorted_score[0..number_vertexes]
        .iter()
        .map(|x| x.0)
        .collect::<HashSet<_>>();
    filtered_hashmap_of_vec(graph, &kept_vertexes)
}

fn filtered_graph_by_connections_number(
    graph: &VecGraph,
    min_nb_sources: u32,
    min_nb_children: u32,
) -> HashmapGraph {
    let sources = count_sources(graph);
    let children = count_children(graph);
    let kept_vertexes = (0..graph.len())
        .filter(|&v| sources[v] > min_nb_sources || children[v] > min_nb_children)
        .collect::<HashSet<_>>();
    filtered_hashmap_of_vec(graph, &kept_vertexes)
}

fn main() {
    generate_databases(WIKI_PATH, GRAPH_PATH, NAME_FROM_ID_PATH);

    let t0 = Instant::now();
    let (name_from_id, id_from_name) = load_name_id_databases(NAME_FROM_ID_PATH);
    println!("Name from id: {}, {:?}", name_from_id.len(), t0.elapsed());
    let graph = load_graph_database(GRAPH_PATH);
    println!("Graph: {}, {:?}", graph.len(), t0.elapsed());

    let nb_edges = graph.iter().map(Vec::len).sum::<usize>();
    println!("Nb edges: {nb_edges}, {:?}", t0.elapsed());

    bfs(
        id_from_name["Susanna_Barker"],
        id_from_name["United_Nations_Security_Council_Resolution_2079"],
        &graph,
    )
    .unwrap()
    .into_iter()
    .for_each(|id| println!("{}", name_from_id[id]));
    println!("{:?}", t0.elapsed());

    let page_rank_result = run_n_page_rank_iterations(10, &graph);
    println!("PageRank: {:?}", t0.elapsed());
    let mut res = page_rank_result.iter().enumerate().collect::<Vec<_>>();
    res.sort_by(|&a, &b| f64::partial_cmp(b.1, a.1).unwrap()); // Reversed sorting

    let ranking = res[0..100]
        .iter()
        .map(|(k, v)| (name_from_id[*k].clone(), *v))
        .collect::<Vec<_>>();
    for (rank, (k, v)) in ranking.iter().enumerate() {
        println!("{}) {}: {}", rank + 1, k, v);
    }

    let filtered = filtered_graph_by_score(&graph, &page_rank_result, 1_000);
    println!("Filtered: {:?}", t0.elapsed());

    export_as_csv(&filtered, &name_from_id, FILTERED_CSV_PATH);
    println!("Export as CSV: {:?}", t0.elapsed());
}
