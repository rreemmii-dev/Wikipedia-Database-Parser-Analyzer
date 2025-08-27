use std::collections::VecDeque;

pub type VecGraph = Vec<Vec<usize>>;

pub fn reversed_graph(graph: &VecGraph) -> VecGraph {
    let mut reversed = vec![Vec::new(); graph.len()];
    for (node, parents) in graph.iter().enumerate() {
        for &parent in parents {
            reversed[parent].push(node);
        }
    }
    reversed
}

pub fn count_parents(graph: &VecGraph) -> Vec<u32> {
    graph
        .iter()
        .map(|parents| parents.len() as u32)
        .collect::<Vec<_>>()
}

pub fn bfs(enter: usize, exit: usize, graph: &VecGraph) -> Option<Vec<usize>> {
    let mut previous = vec![None; graph.len()];
    previous[enter] = Some(usize::MAX);
    let mut queue = VecDeque::new();
    queue.push_back(enter);
    while let Some(vertex) = queue.pop_front() {
        if vertex == exit {
            let mut path = Vec::new();
            let mut previous_vertex = Some(exit);
            while let Some(vertex) = previous_vertex {
                path.push(vertex);
                if vertex == enter {
                    break;
                }
                previous_vertex = previous[vertex];
            }
            path.reverse();
            return Some(path);
        }
        for &parent in &graph[vertex] {
            if previous[parent].is_none() {
                previous[parent] = Some(vertex);
                queue.push_back(parent);
            }
        }
    }
    None
}
