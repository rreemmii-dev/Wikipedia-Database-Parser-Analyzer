use regex::Regex;
use spmc::channel;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::Instant;

// `DATABASE_ERROR` comments show errors that shouldn't happen with a sane Wikipedia database.
// These are known database errors:
// DATABASE_ERROR: Several articles with the same name
// DATABASE_ERROR: Several main articles with the same name
// DATABASE_ERROR: Article redirecting to a non-existing main article
// DATABASE_ERROR: Several articles with the same name, being redirections and main pages
// DATABASE_ERROR: Article having a wikilinks to itself
//
// Additionally, these (very arbitrary) conventions are chosen to ensure determinism despite database inconsistencies:
// If several redirection articles have the same name, the last one is chosen.
// If several main articles have the same name, the first one is chosen.
// If several articles have the same name, the main article is chosen over the redirection article.

/// The separator between IDs in graph.txt
pub const WIKILINKS_SEPARATOR: &str = ",";
/// Whether to keep links from footnote sections (such as Notes, References, etc.)
const KEEP_FOOTNOTE_SECTIONS: bool = false;
/// The number of threads to use while generating the databases
const NB_THREADS: usize = 8;
/// The number of pages per batch
const PAGES_PER_BATCH: usize = 1_000;

pub fn generate_databases(input_path: &str, graph_path: &str, name_from_id_path: &str) {
    let t0 = Instant::now();

    let (id_from_name, redirections) = generate_id_from_name_and_redirections(input_path);
    println!("Step 1: {:?}", t0.elapsed());

    let graph = generate_graph(input_path, &id_from_name, &redirections);
    println!("Step 2: {:?}", t0.elapsed());

    store_name_from_id(id_from_name, name_from_id_path);
    println!("Step 3: {:?}", t0.elapsed());

    store_graph(graph, graph_path);
    println!("Step 4: {:?}", t0.elapsed());
}

fn generate_id_from_name_and_redirections(
    input_path: &str,
) -> (HashMap<String, u32>, HashMap<String, String>) {
    let t0 = Instant::now();
    let mut input_file = BufReader::new(File::open(input_path).unwrap());
    let mut handle = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));
    let (mut sender, receivers) = channel::<(usize, Vec<String>)>();
    for _ in 0..NB_THREADS {
        let receiver = receivers.clone();
        let results = results.clone();
        handle.push(thread::spawn(move || {
            while let Ok((batch_ordering, pages)) = receiver.recv() {
                let mut new_name_entries = Vec::new();
                let mut new_redirection_entries = HashMap::new();
                for page in pages {
                    if let Some((nz_title, nz_redirection_opt)) =
                        get_nz_title_and_nz_redirection(&page)
                    {
                        if let Some(nz_redirection) = nz_redirection_opt {
                            new_redirection_entries.insert(nz_title, nz_redirection);
                        } else {
                            new_name_entries.push(nz_title);
                        }
                    }
                }
                results.lock().unwrap().push((
                    batch_ordering,
                    new_name_entries,
                    new_redirection_entries,
                ));
            }
        }));
    }
    for batch_ordering in 0.. {
        if let Some(pages) = read_n_pages(PAGES_PER_BATCH, &mut input_file) {
            println!(
                "Generate ID from name and redirections - Batch n°{}, {:?}",
                batch_ordering,
                t0.elapsed()
            );
            sender.send((batch_ordering, pages)).unwrap();
        } else {
            println!("Generate ID from name and redirections - End of file");
            drop(sender); // Close the connection, release threads waiting in `receiver.recv()`
            break;
        }
    }
    for t in handle {
        t.join().unwrap();
    }
    let mut article_names = Vec::new();
    let mut redirections = HashMap::new();
    let mut results = Arc::into_inner(results).unwrap().into_inner().unwrap();
    results.sort_by_key(|x| x.0);
    for (_page_ordering, new_name_entries, new_redirection_entries) in results {
        article_names.extend(new_name_entries);
        redirections.extend(new_redirection_entries);
    }
    let mut id_from_name = HashMap::new();
    for nz_title in article_names {
        if id_from_name.contains_key(&nz_title) {
            // DATABASE_ERROR: Several main articles with the same name
            continue;
        }
        id_from_name.insert(nz_title, id_from_name.len() as u32);
    }
    redirections = redirections
        .into_iter()
        .filter(|(k, v)| id_from_name.contains_key(v) && !id_from_name.contains_key(k))
        // DATABASE_ERROR: Article redirecting to a non-existing main article
        // DATABASE_ERROR: Several articles with the same name, being redirections and main pages
        .collect::<HashMap<_, _>>();
    (id_from_name, redirections)
}

fn generate_graph(
    input_path: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> Vec<Vec<u32>> {
    let t0 = Instant::now();
    let mut input_file = BufReader::new(File::open(input_path).unwrap());
    let id_from_name = Arc::new(id_from_name.clone());
    let redirections = Arc::new(redirections.clone());
    let mut handle = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));
    let (mut sender, receiver) = channel::<(usize, Vec<String>)>();
    for _ in 0..NB_THREADS {
        let receiver = receiver.clone();
        let id_from_name = id_from_name.clone();
        let redirections = redirections.clone();
        let results = results.clone();
        handle.push(thread::spawn(move || {
            while let Ok((batch_ordering, pages)) = receiver.recv() {
                let mut new_entries = Vec::new();
                for page in pages {
                    if let Some((title_id, wikilinks_ids)) =
                        get_title_id_and_wikilinks_ids(&page, &id_from_name, &redirections)
                    {
                        new_entries.push((title_id, wikilinks_ids));
                    }
                }
                results.lock().unwrap().push((batch_ordering, new_entries));
            }
        }));
    }
    for batch_ordering in 0.. {
        if let Some(pages) = read_n_pages(PAGES_PER_BATCH, &mut input_file) {
            println!(
                "Generate graph - Batch n°{}, {:?}",
                batch_ordering,
                t0.elapsed()
            );
            sender.send((batch_ordering, pages)).unwrap();
        } else {
            println!("Generate graph - End of file");
            drop(sender); // Close the connection, release threads waiting in `receiver.recv()`
            break;
        }
    }
    for t in handle {
        t.join().unwrap();
    }
    let mut graph = vec![None; id_from_name.len()];
    let mut results = Arc::into_inner(results).unwrap().into_inner().unwrap();
    results.sort_by_key(|x| x.0);
    for (_page_ordering, new_entries) in results {
        for (title_id, wikilinks_ids) in new_entries {
            let title_id = title_id as usize;
            if graph[title_id].is_some() {
                // DATABASE_ERROR: Several main articles with the same name
                continue;
            }
            graph[title_id] = Some(wikilinks_ids);
        }
    }
    graph.into_iter().map(Option::unwrap).collect::<Vec<_>>()
}

fn get_title_id_and_wikilinks_ids(
    page: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> Option<(u32, Vec<u32>)> {
    let title = normalize_title(&get_raw_title(page))?;
    if !id_from_name.contains_key(&title) {
        return None;
    }
    let title_id = id_from_name[&title];

    let content = get_page_content(page);
    let mut wikilinks_ids = get_wikilinks_from_content(&content, id_from_name, redirections)
        .into_iter()
        .map(|wikilink| get_id_from_name(&wikilink, id_from_name, redirections))
        .collect::<Vec<_>>();
    wikilinks_ids.sort();
    if let Some(i) = wikilinks_ids.iter().position(|&id| id == title_id) {
        // DATABASE_ERROR: Article having a wikilinks to itself
        wikilinks_ids.remove(i);
    }

    Some((title_id, wikilinks_ids))
}

fn get_wikilinks_from_content(
    content: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> HashSet<String> {
    let mut res = HashSet::new();

    static SB_LINKS_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\[([^|#]+?)(?:[|#].*?)?]]").unwrap());
    for capture in SB_LINKS_REGEX.captures_iter(content) {
        if let Some(wikilink) = normalize_wikilink(&capture[1])
            && wikilink_is_kept(&wikilink, id_from_name, redirections)
        {
            res.insert(wikilink);
        }
    }

    static CB_LINKS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\{\{(?i:section link|slink|multi-section link|mslink)\|([^|#]+?)(?:[|#].*?)?}}",
        )
        .unwrap()
    });
    for capture in CB_LINKS_REGEX.captures_iter(content) {
        if let Some(wikilink) = normalize_wikilink(&capture[1])
            && wikilink_is_kept(&wikilink, id_from_name, redirections)
        {
            res.insert(wikilink);
        }
    }

    res
}

fn normalize_wikilink(raw_wikilink: &str) -> Option<String> {
    if raw_wikilink.contains("{{") || raw_wikilink.contains("}}") {
        // Nested curly brackets
        return None;
    }
    normalize_title(raw_wikilink)
}

fn get_nz_title_and_nz_redirection(page: &str) -> Option<(String, Option<String>)> {
    let nz_title = normalize_title(&get_raw_title(page))?;
    if let Some(redirection) = get_raw_redirection(page) {
        let nz_redirection = normalize_title(&redirection)?;
        Some((nz_title, Some(nz_redirection)))
    } else {
        Some((nz_title, None))
    }
}

fn normalize_title(raw_title: &str) -> Option<String> {
    let title = raw_title.replace('_', " ");
    if title.contains(':') && !title.split(':').nth(1).unwrap().starts_with(' ') {
        // Probably a namespace
        return None;
    }
    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    let mut chars = title.chars();
    let hd = chars.next().unwrap();
    let tl = chars;
    let title = if hd == 'ß' {
        // ß is the only allowed starting lowercase letter
        hd.to_string() + tl.as_str()
    } else {
        hd.to_uppercase().to_string() + tl.as_str()
    };
    Some(title)
}

fn get_id_from_name(
    nz_name: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> u32 {
    if let Some(redirection) = redirections.get(nz_name) {
        id_from_name[redirection]
    } else {
        id_from_name[nz_name]
    }
}

fn wikilink_is_kept(
    nz_name: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> bool {
    id_from_name.contains_key(nz_name) || redirections.contains_key(nz_name)
}

fn get_raw_redirection(page: &str) -> Option<String> {
    // TODO: Regex isn't useful here...
    static REDIRECT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"<redirect title="(.*?)" />"#).unwrap());
    if let Some(capture) = REDIRECT_REGEX.captures(page) {
        Some(capture[1].to_string())
    } else {
        None
    }
}

fn get_raw_title(page: &str) -> String {
    // TODO: Regex isn't useful here...
    static TITLE_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<title>(.*?)</title>").unwrap());
    let capture = TITLE_REGEX.captures(page).unwrap();
    capture[1].to_string()
}

fn get_page_content(page: &str) -> String {
    let opening_index = page.find("<text ").unwrap() + "<text ".len();
    let opening_index = opening_index + page[opening_index..].find(">").unwrap() + ">".len();
    let closing_index = opening_index + page[opening_index..].find("</text>").unwrap();
    let content = page[opening_index..closing_index].to_string();

    clear_content(&content)
}

fn clear_content(content: &str) -> String {
    let mut content = html_escape::decode_html_entities(content).to_string();

    static COMMENTS_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
    content = COMMENTS_REGEX.replace_all(&content, "").to_string();

    static NOWIKI_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<nowiki>.*?</nowiki>").unwrap());
    content = NOWIKI_REGEX.replace_all(&content, "").to_string();

    static SYNTAX_HIGHLIGHT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<syntaxhighlight.*?>.*?</syntaxhighlight>").unwrap());
    content = SYNTAX_HIGHLIGHT_REGEX.replace_all(&content, "").to_string();

    if !KEEP_FOOTNOTE_SECTIONS {
        static REFERENCES_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?s)<ref[> ].*?</ref>").unwrap());
        content = REFERENCES_REGEX.replace_all(&content, "").to_string();
        let footnote_sections = [
            // TODO: Use regex? Then \s* for spaces
            "==Notes==",
            "== Notes ==",
            "==References==",
            "== References ==",
            "==Further reading==",
            "== Further reading ==",
            "==External links==",
            "== External links ==",
        ];
        for section in footnote_sections {
            if let Some(index) = content.find(section) {
                content = content[0..index].to_string();
            }
        }
    }
    content
}

fn read_n_pages(n: usize, file: &mut BufReader<File>) -> Option<Vec<String>> {
    let mut res = Vec::new();
    for _ in 0..n {
        if let Some(page) = read_one_page(file) {
            res.push(page);
        } else {
            break;
        }
    }
    if res.is_empty() { None } else { Some(res) }
}

fn read_one_page(file: &mut BufReader<File>) -> Option<String> {
    read_until_line(file, "<page>");
    read_until_line(file, "</page>")
}

fn read_until_line(file: &mut BufReader<File>, wanted_line: &str) -> Option<String> {
    let mut text = String::new();
    for line in file.lines() {
        let line = line.unwrap();
        let line = line.trim();
        if line == wanted_line {
            return Some(text);
        }
        assert!(!line.contains(wanted_line), "\"{wanted_line}\" in {line}");
        text += line;
        text += "\n";
    }
    None
}

fn store_name_from_id(id_from_name: HashMap<String, u32>, name_from_id_path: &str) {
    let mut name_from_id_items = id_from_name
        .into_iter()
        .map(|(name, id)| (id, name))
        .collect::<Vec<_>>();
    name_from_id_items.sort();
    if let Some(folder) = Path::new(name_from_id_path).parent() {
        create_dir_all(folder).unwrap();
    }
    let mut name_from_id_file = File::create(name_from_id_path).unwrap();
    for (line_index, (id, name)) in name_from_id_items.into_iter().enumerate() {
        assert_eq!(id as usize, line_index);
        let mut content = String::new();
        if line_index > 0 {
            content += "\n";
        }
        content += &name;
        name_from_id_file.write_all(content.as_bytes()).unwrap();
    }
}

fn store_graph(graph: Vec<Vec<u32>>, graph_path: &str) {
    if let Some(folder) = Path::new(graph_path).parent() {
        create_dir_all(folder).unwrap();
    }
    let mut graph_file = File::create(graph_path).unwrap();
    for (id, parents) in graph.into_iter().enumerate() {
        let mut content = String::new();
        if id > 0 {
            content += "\n";
        }
        for wikilink in parents {
            content += &wikilink.to_string();
            content += WIKILINKS_SEPARATOR;
        }
        if content.ends_with(WIKILINKS_SEPARATOR) {
            // TODO: Only works because WIKILINKS_SEPARATOR.len() == 1
            content.pop();
        }
        graph_file.write_all(content.as_bytes()).unwrap();
    }
}
