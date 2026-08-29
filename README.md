# Wikipedia-Database-Parser-Analyzer


A parser and analyzer for Wikipedia database dumps, written in Rust.

* Generate the graph linking each article to its sources (= parents)
* Find the shortest path between 2 Wikipedia articles using the [BFS algorithm](https://en.wikipedia.org/wiki/Breadth-first_search)
* Keep the most important vertexes using algorithms such as the [PageRank algorithm](https://en.wikipedia.org/wiki/PageRank)
* Render the graph using Gephi

<div align="center">
Cluster dealing with computers in a 10,000 vertexes graph:
<br>
<img src="https://rreemmii-dev.github.io/wikipedia-database-parser-analyzer/computer_cluster.png" alt="Cluster about computers" />
</div>


> [!NOTE]
> You can find the shortest path between any two Wikipedia articles on this website:
> 
> http://wikipedia.remi-deloye.me:10080 (make sure to use `http`, as `https` is not configured)


## Table of Contents

- [Getting started](#getting-started)
- [Provided tools](#provided-tools)
- [Wikipedia documentation](#wikipedia-documentation)
- [License](#license)


## Getting started

### Installation

```bash
# Clone the repository:
git clone https://github.com/rreemmii-dev/Wikipedia-Database-Parser-Analyzer.git

cd Wikipedia-Database-Parser-Analyzer
```

Ensure you have `cargo` installed.

### Wikipedia dump download

1. Download a Wikipedia dump. A list can be found [here](https://meta.wikimedia.org/wiki/Data_dump_torrents#English_Wikipedia), and a download tutorial [here](https://academictorrents.com/docs/downloading.html). The latest dump I used is [English Wikipedia, 2025-06-01](https://academictorrents.com/details/21dc1fcba351758654f37fb904e629d143dc2a9f) (uncompressed size of 103 GB, contains more than 7 million parsed articles).
2. Extract the dump.
3. Set the `WIKI_PATH` constant in [src/main.rs](src/main.rs) to the dump file path, relative to `Cargo.toml`.

### Run

#### Performance / Hardware requirements

At least 8GB of RAM is strongly recommended, as the program can use up to 7GB of RAM.

Databases need to be generated once and for all, which takes about 15 min.
Then, importing them only takes a few seconds.

#### Run the `main.rs` example file:

[src/main.rs](src/main.rs) does the following:
1. If they have not been generated yet, generates the required databases and saves them in files (using `generate_databases`, see [#Provided tools/Generate and load link databases](#generate-and-load-link-databases) for more information).
2. Loads databases from generated files.
3. Executes a BFS to find the shortest path from the `Wikipedia` article to the `Rust (programming language)` article.
4. Compute a PageRank ranking, and display the 10 most popular articles.
5. Filters the graph to keep only the 1000 most popular articles.
6. Exports the filtered graph as a CSV file.

It can be run using:

```bash
cargo run --release
```


## Provided tools

### Generate and load link databases

`generate_databases` in [src/lib/database/generate_database.rs](src/lib/database/generate_database.rs) generates the following databases:
- `graph`: The graph with each article pointing towards its sources, stored as an adjacency list. Each article is represented by an ID.
- `name_from_id`: The article name corresponding to the article ID.

Databases are saved in files, so you can run `generate_databases` once and for all.

`load_graph` and `load_name_from_id_and_id_from_name` in [src/lib/database/load_database.rs](src/lib/database/load_database.rs) load the two previous databases, plus the `id_from_name` database.

### Different graph types with built-in functions

There are 2 graph types, both being adjacency lists:
- `VecGraph` (`Vec<Vec<usize>>`) for computation on the whole graph. See [src/lib/graph_utils/vec_graph_utils.rs](src/lib/graph_utils/vec_graph_utils.rs).
- `HashmapGraph` (`HashMap<u32, HashSet<u32>>`) if some vertexes were to be removed from the graph. See [src/lib/graph_utils/hashmap_graph_utils.rs](src/lib/graph_utils/hashmap_graph_utils.rs).

The reason for having 2 graph types is that access to `Vec` elements is much faster than access to `HashMap` elements, but items cannot be removed from a `Vec` while leaving other indexes unchanged.

### Render graphs using Gephi

To open the graph in Gephi, it has to be exported as a CSV file using `export_as_csv` in [src/lib/graph_utils/hashmap_graph_utils.rs](src/lib/graph_utils/hashmap_graph_utils.rs).

It is then stored as an adjacency list. It is advised to use `#` as the CSV delimiter (Wikipedia article names cannot contain `#`).

It is not recommended to export graph having more than 10,000 vertexes. See https://gephi.org/users/requirements/.


## Wikipedia documentation

Here is some documentation about Wikipedia articles:
- Links parsing:
    - `[[PageName]]` links: https://en.wikipedia.org/wiki/Help:Link#Wikilinks_(internal_links)
    - `{{section link|PageName}}` links: https://en.wikipedia.org/wiki/Help:Link#Section_linking_(anchors) and https://en.wikipedia.org/wiki/Template:Section_link
    - `{{multi-section link|PageName}}` links: https://en.wikipedia.org/wiki/Template:Multi-section_link
    - `<nowiki>` tag: https://en.wikipedia.org/wiki/Help:Wikitext#Nowiki
    - `<syntaxhighlight>` tag: https://en.wikipedia.org/wiki/Template:Syntaxhighlight
- Article names:
    - Namespaces (e.g.: article names such as "Category:XXX", or "Help:XXX"): https://en.wikipedia.org/wiki/Wikipedia:Namespace
    - Name Restrictions: https://en.wikipedia.org/wiki/Wikipedia:Naming_conventions_(technical_restrictions) and https://en.wikipedia.org/wiki/Wikipedia:Page_name#Technical_restrictions_and_limitations
- Appendice and footnote sections: https://en.wikipedia.org/wiki/Wikipedia:Manual_of_Style/Layout#Standard_appendices_and_footers and https://en.wikipedia.org/wiki/Help:Footnotes


## License

Distributed under the MIT License. See [LICENSE.md](LICENSE.md)
