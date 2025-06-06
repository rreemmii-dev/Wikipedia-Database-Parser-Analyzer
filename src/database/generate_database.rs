use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::Instant;

// `DATABASE_ERROR` comments show cases that shouldn't happen with a sane Wikipedia database

pub const TITLE_SEPARATOR: &str = "|";
pub const WIKILINKS_SEPARATOR: &str = ",";
const KEEP_FOOTNOTE_SECTIONS: bool = false;

const THREAD_SIZE: usize = 1_000;

fn read_until_line(line_content: &str, file: &mut BufReader<File>) -> Option<String> {
    let mut text = String::new();
    for line in file.lines().map(|l| l.unwrap()) {
        let line = line.trim().to_string();
        if line == line_content {
            return Some(text);
        }
        assert!(!line.contains(line_content), "\"{line_content}\": {line}");
        text += &line;
    }
    None
}

fn read_one_page(in_file: &mut BufReader<File>) -> Option<String> {
    read_until_line("<page>", in_file);
    read_until_line("</page>", in_file)
}

fn read_n_pages(n: usize, in_file: &mut BufReader<File>) -> Option<Vec<String>> {
    let mut res = Vec::new();
    for _ in 0..n {
        if let Some(page) = read_one_page(in_file) {
            res.push(page);
        } else {
            break;
        }
    }
    if res.is_empty() { None } else { Some(res) }
}

fn normalize_title(raw_title: &str) -> Option<String> {
    let title = raw_title.replace(' ', "_");
    if title.contains(':') && !title.split(':').nth(1).unwrap().starts_with('_') {
        // TODO: Filter only namespaces
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
    assert!(!title.contains(TITLE_SEPARATOR), "{}", title);
    Some(title)
}

fn get_raw_redirection(page: &str) -> Option<String> {
    static REDIRECT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"<redirect title="(.*?)" />"#).unwrap());
    if let Some(capture) = REDIRECT_REGEX.captures(page) {
        Some(capture[1].to_string())
    } else {
        None
    }
}

fn get_raw_title(page: &str) -> String {
    static TITLE_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<title>(.*?)</title>").unwrap());
    let capture = TITLE_REGEX.captures(page).unwrap();
    capture[1].to_string()
}

fn get_nz_title_nz_redirection(page: &str) -> Option<(String, Option<String>)> {
    let nz_title = normalize_title(&get_raw_title(page))?;
    if let Some(redirection) = get_raw_redirection(page) {
        let nz_redirection = normalize_title(&redirection)?;
        if nz_title == nz_redirection {
            // DATABASE_ERROR: Articles redirects to itself
            return None;
        }
        Some((nz_title, Some(nz_redirection)))
    } else {
        Some((nz_title, None))
    }
}

fn generate_id_from_name_redirections(
    in_path: &str,
) -> (HashMap<String, u32>, HashMap<String, String>) {
    let t0 = Instant::now();
    let mut in_file = BufReader::new(File::open(in_path).unwrap());
    let mut id_from_name = HashMap::new();
    let redirections = Arc::new(Mutex::new(HashMap::new()));
    let mut handle = Vec::new();
    let mut i = 0;
    loop {
        println!("{}, {:?}", i, t0.elapsed());
        if let Some(pages) = read_n_pages(THREAD_SIZE, &mut in_file) {
            let redirections = redirections.clone();
            handle.push(thread::spawn(move || {
                let mut new_id_from_name_entries = Vec::new();
                let mut new_redirections_entries = HashMap::new();
                for page in pages {
                    if let Some((nz_title, nz_redirection_opt)) = get_nz_title_nz_redirection(&page)
                    {
                        if let Some(nz_redirection) = nz_redirection_opt {
                            new_redirections_entries.insert(nz_title, nz_redirection);
                        } else {
                            new_id_from_name_entries.push(nz_title);
                        }
                    }
                }
                redirections
                    .lock()
                    .unwrap()
                    .extend(new_redirections_entries);
                new_id_from_name_entries
            }));
        } else {
            break;
        }
        i += THREAD_SIZE;
    }
    for t in handle {
        let new_id_from_name_entries = t.join().unwrap();
        for nz_title in new_id_from_name_entries {
            if id_from_name.contains_key(&nz_title) {
                // DATABASE_ERROR: Several articles with the same normalized name
                continue;
            }
            id_from_name.insert(nz_title, id_from_name.len() as u32);
        }
    }
    let mut redirections = Arc::try_unwrap(redirections).unwrap().into_inner().unwrap();
    for (k, v) in &redirections.clone() {
        if !id_from_name.contains_key(v) {
            redirections.remove(k);
        }
        if id_from_name.contains_key(k) {
            // DATABASE_ERROR: Several articles with the same name, being redirections and main pages
            redirections.remove(k);
        }
    }
    (id_from_name, redirections)
}

fn get_id_from_name(
    name: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> u32 {
    if let Some(redirection) = redirections.get(name) {
        id_from_name[redirection]
    } else {
        id_from_name[name]
    }
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

    content.to_string()
}

fn get_page_content(page: &str) -> String {
    let opening_index = "<text ".len() + page.find("<text ").unwrap();
    let opening_index = opening_index + ">".len() + page[opening_index..].find(">").unwrap();
    let closing_index = opening_index + page[opening_index..].find("</text>").unwrap();
    let content = page[opening_index..closing_index].to_string();

    clear_content(&content)
}

fn wikilink_is_kept(
    name: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> bool {
    id_from_name.contains_key(name) || redirections.contains_key(name)
}

fn get_wikilinks_from_content(
    content: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> HashSet<String> {
    fn normalize_wikilink(raw_wikilink: &str) -> Option<String> {
        if raw_wikilink.contains("{{") || raw_wikilink.contains("}}") {
            // Nested curly brackets
            return None;
        }
        normalize_title(raw_wikilink)
    }

    let mut res = HashSet::new();

    static SB_LINKS_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\[([^|#]+?)(?:[|#].*?)?]]").unwrap());
    for capture in SB_LINKS_REGEX.captures_iter(content) {
        if let Some(wikilink) = normalize_wikilink(&capture[1]) {
            if wikilink_is_kept(&wikilink, id_from_name, redirections) {
                res.insert(wikilink);
            }
        }
    }

    static CB_LINKS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\{\{(?i:section link|slink|multi-section link|mslink)\|([^|#]+?)(?:[|#].*?)?}}",
        )
        .unwrap()
    });
    for capture in CB_LINKS_REGEX.captures_iter(content) {
        if let Some(wikilink) = normalize_wikilink(&capture[1]) {
            if wikilink_is_kept(&wikilink, id_from_name, redirections) {
                res.insert(wikilink);
            }
        }
    }

    res
}

fn get_title_id_wikilinks_ids(
    page: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> Option<(u32, HashSet<u32>)> {
    let title = normalize_title(&get_raw_title(page))?;
    if !id_from_name.contains_key(&title) {
        return None;
    }

    let title_id = get_id_from_name(&title, id_from_name, redirections);

    let content = get_page_content(page);
    let mut wikilinks_ids = get_wikilinks_from_content(&content, id_from_name, redirections)
        .into_iter()
        .map(|wikilink| get_id_from_name(&wikilink, id_from_name, redirections))
        .collect::<HashSet<_>>();
    if wikilinks_ids.contains(&title_id) {
        // DATABASE_ERROR: Article linking to itself
        wikilinks_ids.remove(&title_id);
    }

    Some((title_id, wikilinks_ids))
}

fn generate_graph(
    in_path: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> HashMap<u32, HashSet<u32>> {
    let t0 = Instant::now();
    let mut in_file = BufReader::new(File::open(in_path).unwrap());
    let graph = Arc::new(Mutex::new(HashMap::new()));
    let id_from_name = Arc::new(id_from_name.clone());
    let redirections = Arc::new(redirections.clone());
    let mut handle = Vec::new();
    let mut i = 0;
    loop {
        println!("{}, {:?}", i, t0.elapsed());
        if let Some(pages) = read_n_pages(THREAD_SIZE, &mut in_file) {
            let graph = graph.clone();
            let id_from_name = id_from_name.clone();
            let redirections = redirections.clone();
            handle.push(thread::spawn(move || {
                let mut new_entries = HashMap::new();
                for page in pages {
                    if let Some((title_id, wikilinks_ids)) =
                        get_title_id_wikilinks_ids(&page, &id_from_name, &redirections)
                    {
                        new_entries.insert(title_id, wikilinks_ids);
                    }
                }
                graph.lock().unwrap().extend(new_entries);
            }));
        } else {
            break;
        }
        i += THREAD_SIZE;
    }
    for t in handle {
        t.join().unwrap();
    }
    Arc::try_unwrap(graph).unwrap().into_inner().unwrap()
}

pub fn generate_databases(in_path: &str, graph_path: &str, name_from_id_path: &str) {
    let t0 = Instant::now();

    let (id_from_name, redirections) = generate_id_from_name_redirections(in_path);
    println!("Step 1: {:?}", t0.elapsed());

    let graph = generate_graph(in_path, &id_from_name, &redirections);
    println!("Step 2: {:?}", t0.elapsed());

    let mut name_from_id_items = id_from_name
        .into_iter()
        .map(|(name, id)| (id, name))
        .collect::<Vec<_>>();
    name_from_id_items.sort();
    let mut name_from_id_file = File::create(name_from_id_path).unwrap();
    for (line, (id, name)) in name_from_id_items.into_iter().enumerate() {
        assert_eq!(id, line as u32);
        let mut content = String::new();
        content += &id.to_string();
        content += TITLE_SEPARATOR;
        content += &name;
        content += "\n";
        name_from_id_file.write_all(content.as_bytes()).unwrap();
    }
    println!("Step 3: {:?}", t0.elapsed());

    let mut graph_vec = graph.into_iter().collect::<Vec<_>>();
    graph_vec.sort_by(|a, b| u32::cmp(&a.0, &b.0));
    let mut graph_file = File::create(graph_path).unwrap();
    for (line, (id, neighbors)) in graph_vec.into_iter().enumerate() {
        assert_eq!(id, line as u32);
        let mut content = String::new();
        content += &id.to_string();
        content += TITLE_SEPARATOR;
        for wikilink in neighbors {
            content += &wikilink.to_string();
            content += WIKILINKS_SEPARATOR;
        }
        if content.ends_with(WIKILINKS_SEPARATOR) {
            content.pop();
        }
        content += "\n";
        graph_file.write_all(content.as_bytes()).unwrap();
    }
    println!("Step 4: {:?}", t0.elapsed());
}
