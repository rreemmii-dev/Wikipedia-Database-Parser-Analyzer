# Wikipedia-Database-Parser-Analyzer



A parser and analyzer for Wikipedia database dumps, written in Rust.

Generate the graph linking each article to its sources, keep the most important vertexes using algorithms such as the [PageRank algorithm](https://en.wikipedia.org/wiki/PageRank), and render it using Gephi!

You can also find the shortest path between 2 Wikipedia articles using the [BFS algorithm](https://en.wikipedia.org/wiki/Breadth-first_search).

<div align="center">
Cluster dealing with UK politics in a generated graph:
<br>
<img src="https://rreemmii-dev.github.io/wikipedia-database-parser-analyzer/uk_politics_graph.png" alt="Graph of UK Politics" />
</div>


## Table of content

- [Getting started](#getting-started)
- [Provided tools](#provided-tools)
- [Wikipedia documentation](#wikipedia-documentation)
- [License](#license)


## Getting started

### Installation

```bash
# Clone the repository:
git clone git@github.com:rreemmii-dev/Wikipedia-Database-Parser-Analyzer.git

cd Wikipedia-Database-Parser-Analyzer
```

Ensure you have `cargo` installed.

### Wikipedia dump download

1. Download a Wikipedia dump (a list can be found [here](https://meta.wikimedia.org/wiki/Data_dump_torrents#English_Wikipedia)). The dump I used is [English Wikipedia, 2025-05-01](https://academictorrents.com/details/cd872797612d95384de3a0ab7e6a1f156bf91495) (uncompressed size: 101 GB, contains about 7 million parsed articles).
2. Extract the dump.
3. Set the `WIKI_PATH` constant in both [src/main.rs](src/main.rs) and [src/simple_main.rs](src/simple_main.rs) to the dump file path, relative to `Cargo.toml`.

### Run

#### Run the example file:

[src/simple_main.rs](src/simple_main.rs) does the following:
1. Generates required databases (using `generate_databases`, see [#Provided tools/Generate and load link databases](#generate-and-load-link-databases) for more information).
2. Loads databases.
3. Executes a BFS to find the shortest path from the `Wikipedia` article to the `Rust_(programming_language)` article.
4. Filters the graph to keep only vertexes having more than 1000 children or parents.
5. Exports the filtered graph as a CSV file.

It can be run using:

```bash
cargo run --release --bin Example
```

#### Run the [src/main.rs](src/main.rs) file

```bash
cargo run --release
```


## Provided tools

### Generate and load link databases

`generate_databases` in [src/database/generate_database.rs](src/database/generate_database.rs) generates the following databases:
- `graph`: The graph with each article pointing towards its sources, stored as an adjacency list. Each article is represented by an ID.
- `name_from_id`: The article name corresponding to the article ID.

`load_name_id_databases` and `load_graph_database` in [src/database/load_database.rs](src/database/load_database.rs) load the two previous databases, plus the `id_from_name` database.

### Different graph types with built-in functions

There are 2 graph types:
- `VecGraph` (`Vec<Vec<usize>>`) for computation on the whole graph. See [src/vec_graph_utils.rs](src/vec_graph_utils.rs).
- `HashmapGraph` (`HashMap<u32, HashSet<u32>>`) if some vertexes were to be removed from the graph. See [src/hashmap_graph_utils.rs](src/hashmap_graph_utils.rs).

### Render graphs using Gephi

To open the graph in Gephi, it has to be exported as a CSV file using `export_as_csv` in [src/hashmap_graph_utils.rs](src/hashmap_graph_utils.rs).

It is then stored as an adjacency list using `#` as the CSV delimiter (Wikipedia article names cannot contain `#`).

It is not recommended to export graph having more than 10,000 vertexes. See https://gephi.org/users/requirements/.


## Wikipedia documentation

Here is some documentation about Wikipedia databases:
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
