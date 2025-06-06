use std::collections::VecDeque;

pub type VecGraph = Vec<Vec<usize>>;

pub fn reversed_graph(graph: &VecGraph) -> VecGraph {
    let mut reversed = vec![Vec::new(); graph.len()];
    for (id, sources) in graph.iter().enumerate() {
        for &s in sources {
            reversed[s].push(id);
        }
    }
    reversed
}

pub fn count_sources(graph: &VecGraph) -> Vec<u32> {
    graph
        .iter()
        .map(|source| source.len() as u32)
        .collect::<Vec<_>>()
}

pub fn count_children(reversed: &VecGraph) -> Vec<u32> {
    reversed
        .iter()
        .map(|child| child.len() as u32)
        .collect::<Vec<_>>()
}

pub fn bfs(enter: usize, exit: usize, graph: &VecGraph) -> Option<Vec<usize>> {
    let mut parent = vec![None; graph.len()];
    parent[enter] = Some(None);
    let mut queue = VecDeque::new();
    queue.push_back(enter);
    while let Some(v) = queue.pop_front() {
        if v == exit {
            let mut v_parent = Some(v);
            let mut path = Vec::new();
            while let Some(v) = v_parent {
                path.push(v);
                v_parent = parent[v].unwrap();
            }
            path.reverse();
            return Some(path);
        }
        for &n in &graph[v] {
            if parent[n].is_none() {
                parent[n] = Some(Some(v));
                queue.push_back(n);
            }
        }
    }
    None
}
