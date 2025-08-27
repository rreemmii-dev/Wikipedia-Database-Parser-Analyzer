use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::Instant;

// `DATABASE_ERROR` comments show errors that shouldn't happen with a sane Wikipedia database

pub const WIKILINKS_SEPARATOR: &str = ",";
const KEEP_FOOTNOTE_SECTIONS: bool = false;
const PAGES_PER_THREAD: usize = 1_000;

fn read_until_line(wanted_line: &str, file: &mut BufReader<File>) -> Option<String> {
    let mut text = String::new();
    for line in file.lines().map(|l| l.unwrap()) {
        let line = line.trim().to_string();
        if line == wanted_line {
            return Some(text);
        }
        assert!(!line.contains(wanted_line), "\"{wanted_line}\" in {line}");
        text += &line;
        text += "\n";
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

fn get_nz_title_and_nz_redirection(page: &str) -> Option<(String, Option<String>)> {
    let nz_title = normalize_title(&get_raw_title(page))?;
    if let Some(redirection) = get_raw_redirection(page) {
        let nz_redirection = normalize_title(&redirection)?;
        Some((nz_title, Some(nz_redirection)))
    } else {
        Some((nz_title, None))
    }
}

fn generate_id_from_name_and_redirections(
    in_path: &str,
) -> (HashMap<String, u32>, HashMap<String, String>) {
    let t0 = Instant::now();
    let mut in_file = BufReader::new(File::open(in_path).unwrap());
    let mut handle = Vec::new();
    let mut i = 0;
    loop {
        println!("{}, {:?}", i, t0.elapsed());
        if let Some(pages) = read_n_pages(PAGES_PER_THREAD, &mut in_file) {
            handle.push(thread::spawn(move || {
                let mut new_id_from_name_entries = Vec::new();
                let mut new_redirections_entries = HashMap::new();
                for page in pages {
                    if let Some((nz_title, nz_redirection_opt)) =
                        get_nz_title_and_nz_redirection(&page)
                    {
                        if let Some(nz_redirection) = nz_redirection_opt {
                            new_redirections_entries.insert(nz_title, nz_redirection);
                        } else {
                            new_id_from_name_entries.push(nz_title);
                        }
                    }
                }
                (new_id_from_name_entries, new_redirections_entries)
            }));
        } else {
            break;
        }
        i += PAGES_PER_THREAD;
    }
    let mut id_from_name = HashMap::new();
    let mut redirections = HashMap::new();
    for t in handle {
        let (new_id_from_name_entries, new_redirections_entries) = t.join().unwrap();
        redirections.extend(new_redirections_entries);
        for nz_title in new_id_from_name_entries {
            if id_from_name.contains_key(&nz_title) {
                // DATABASE_ERROR: Several main articles with the same normalized name
                continue;
            }
            id_from_name.insert(nz_title, id_from_name.len() as u32);
        }
    }
    redirections = redirections
        .into_iter()
        .filter(|(k, v)| id_from_name.contains_key(v) && !id_from_name.contains_key(k))
        // DATABASE_ERROR: Article redirecting to a non-existing main article
        // DATABASE_ERROR: Several articles with the same name, being redirections and main pages
        .collect::<HashMap<_, _>>();
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
    let opening_index = page.find("<text ").unwrap() + "<text ".len();
    let opening_index = opening_index + page[opening_index..].find(">").unwrap() + ">".len();
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

fn normalize_wikilink(raw_wikilink: &str) -> Option<String> {
    if raw_wikilink.contains("{{") || raw_wikilink.contains("}}") {
        // Nested curly brackets
        return None;
    }
    normalize_title(raw_wikilink)
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

fn get_title_id_and_wikilinks_ids(
    page: &str,
    id_from_name: &HashMap<String, u32>,
    redirections: &HashMap<String, String>,
) -> Option<(u32, HashSet<u32>)> {
    let title = normalize_title(&get_raw_title(page))?;
    if !id_from_name.contains_key(&title) {
        return None;
    }
    let title_id = id_from_name[&title];

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
    let id_from_name = Arc::new(id_from_name.clone());
    let redirections = Arc::new(redirections.clone());
    let mut handle = Vec::new();
    let mut i = 0;
    loop {
        println!("{}, {:?}", i, t0.elapsed());
        if let Some(pages) = read_n_pages(PAGES_PER_THREAD, &mut in_file) {
            let id_from_name = id_from_name.clone();
            let redirections = redirections.clone();
            handle.push(thread::spawn(move || {
                let mut new_entries = HashMap::new();
                for page in pages {
                    if let Some((title_id, wikilinks_ids)) =
                        get_title_id_and_wikilinks_ids(&page, &id_from_name, &redirections)
                    {
                        new_entries.insert(title_id, wikilinks_ids);
                    }
                }
                new_entries
            }));
        } else {
            break;
        }
        i += PAGES_PER_THREAD;
    }
    let mut graph = HashMap::new();
    for t in handle {
        let new_entries = t.join().unwrap();
        graph.extend(new_entries);
    }
    graph
}

pub fn generate_databases(in_path: &str, graph_path: &str, name_from_id_path: &str) {
    let t0 = Instant::now();

    let (id_from_name, redirections) = generate_id_from_name_and_redirections(in_path);
    println!("Step 1: {:?}", t0.elapsed());

    let graph = generate_graph(in_path, &id_from_name, &redirections);
    println!("Step 2: {:?}", t0.elapsed());

    let mut name_from_id_items = id_from_name
        .into_iter()
        .map(|(name, id)| (id, name))
        .collect::<Vec<_>>();
    name_from_id_items.sort();
    let mut name_from_id_file = File::create(name_from_id_path).unwrap();
    for (line_index, (id, name)) in name_from_id_items.into_iter().enumerate() {
        assert_eq!(id as usize, line_index);
        let mut content = String::new();
        if line_index != 0 {
            content += "\n";
        }
        content += &name;
        name_from_id_file.write_all(content.as_bytes()).unwrap();
    }
    println!("Step 3: {:?}", t0.elapsed());

    let mut graph_vec = graph.into_iter().collect::<Vec<_>>();
    graph_vec.sort_by(|a, b| u32::cmp(&a.0, &b.0));
    let mut graph_file = File::create(graph_path).unwrap();
    for (line_index, (id, parents)) in graph_vec.into_iter().enumerate() {
        assert_eq!(id as usize, line_index);
        let mut content = String::new();
        if line_index != 0 {
            content += "\n";
        }
        for wikilink in parents {
            content += &wikilink.to_string();
            content += WIKILINKS_SEPARATOR;
        }
        if content.ends_with(WIKILINKS_SEPARATOR) {
            content.pop();
        }
        graph_file.write_all(content.as_bytes()).unwrap();
    }
    println!("Step 4: {:?}", t0.elapsed());
}
