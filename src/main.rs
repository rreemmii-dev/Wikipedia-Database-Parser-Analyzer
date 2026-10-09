use Wikipedia::database::generate_database::generate_databases;
use Wikipedia::database::load_database::{load_graph, load_name_from_id_and_id_from_name};
use Wikipedia::graph_utils::map_graph_utils::{MapGraph, export_as_csv, filtered_map_of_vec};
use Wikipedia::graph_utils::vec_graph_utils::{VecGraph, bfs, count_parents};
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

const WIKI_PATH: &str = "../wikipedia_database/enwiki-20250601-pages-articles-multistream.xml";
const NAME_FROM_ID_PATH: &str = "data/name_from_id.txt";
const GRAPH_PATH: &str = "data/graph.txt";
const FILTERED_CSV_PATH: &str = "data/filtered.csv";

fn page_rank_iteration(graph: &VecGraph, prev_res: &Vec<f64>) -> Vec<f64> {
    let mut res = vec![0.; prev_res.len()];
    for (id, parents) in graph.iter().enumerate() {
        res[id] += 0.5 * prev_res[id];
        let nb_parents = parents.len() as f64;
        for &parent in parents {
            res[parent] += 0.5 * prev_res[id] / nb_parents;
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
) -> MapGraph {
    let mut sorted_score = scores.iter().enumerate().collect::<Vec<_>>();
    sorted_score.sort_by(|a, b| f64::partial_cmp(b.1, a.1).unwrap()); // Reversed sorting
    let kept_vertexes = sorted_score[0..number_vertexes]
        .iter()
        .map(|x| x.0)
        .collect::<HashSet<_>>();
    filtered_map_of_vec(graph, &kept_vertexes)
}

#[allow(unused)] // Another example function that could be used...
fn filtered_graph_by_connections_number(
    graph: &VecGraph,
    reversed: &VecGraph,
    min_nb_parents: u32,
    min_nb_children: u32,
) -> MapGraph {
    let parents = count_parents(graph);
    let children = count_parents(reversed);
    let kept_vertexes = (0..graph.len())
        .filter(|&v| parents[v] > min_nb_parents || children[v] > min_nb_children)
        .collect::<HashSet<_>>();
    filtered_map_of_vec(graph, &kept_vertexes)
}

fn main() {
    let t0 = Instant::now();
    if !(Path::new(GRAPH_PATH).exists() && Path::new(NAME_FROM_ID_PATH).exists()) {
        generate_databases(WIKI_PATH, GRAPH_PATH, NAME_FROM_ID_PATH);
        println!("Databases generated in {:?}", t0.elapsed());
    }

    let t0 = Instant::now();
    println!("Start loading databases...");
    let (name_from_id, id_from_name) = load_name_from_id_and_id_from_name(NAME_FROM_ID_PATH);
    let graph = load_graph(GRAPH_PATH);
    assert_eq!(graph.len(), name_from_id.len());
    assert_eq!(graph.len(), id_from_name.len());
    let nb_edges = graph.iter().map(Vec::len).sum::<usize>();
    println!(
        "Graph loaded: {} vertexes, {} edges, loaded in {:?}",
        graph.len(),
        nb_edges,
        t0.elapsed()
    );
    println!();

    let t0 = Instant::now();
    let source = "Wikipedia";
    let destination = "Rust (programming language)";
    println!("BFS from {} to {}...", source, destination);
    let bfs_result = bfs(id_from_name[source], id_from_name[destination], &graph).unwrap();
    println!("{} clicks:", bfs_result.len() - 1);
    for article_id in bfs_result {
        println!("* {}", name_from_id[article_id]);
    }
    println!("Computed in {:?}", t0.elapsed());
    println!();

    let t0 = Instant::now();
    println!("Computing PageRank...");
    let page_rank_result = run_n_page_rank_iterations(10, &graph);
    let mut res = page_rank_result.iter().enumerate().collect::<Vec<_>>();
    res.sort_by(|&a, &b| f64::partial_cmp(b.1, a.1).unwrap()); // Reversed sorting
    println!("Most popular articles:");
    for (rank, (article_id, _score)) in res.into_iter().take(10).enumerate() {
        println!("{}) {}", rank + 1, name_from_id[article_id]);
    }
    println!("Computed in {:?}", t0.elapsed());
    println!();

    let t0 = Instant::now();
    println!("Filtering the graph to keep only the 1000 most popular articles...");
    let filtered = filtered_graph_by_score(&graph, &page_rank_result, 1_000);
    println!("Filtered in {:?}", t0.elapsed());
    println!();

    let t0 = Instant::now();
    println!("Saving filtered graph as CSV...");
    export_as_csv(&filtered, Some(&name_from_id), "#", FILTERED_CSV_PATH);
    println!("Exported as CSV in {:?}", t0.elapsed());
}
