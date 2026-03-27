use reqwest::Client;
use serde_json::Value;
use regex::Regex;
use std::env;
use tokio::time::{sleep, Duration};
use crate::interner::Interner;
use crate::graph::Graph;
use crate::types::{EdgeType, GraphPayload, NodePayload, EdgePayload};

/// Fetch repository structure and code contents from GitHub.
/// Strictly limited to 80 files and 50KB per file.
/// Sequential fetching with 75ms delay to prevent rate limiting.
pub async fn fetch_github_graph<F>(
    owner: &str,
    repo: &str,
    interner: &mut Interner,
    graph: &mut Graph,
    mut on_progress: F,
) -> Result<GraphPayload, String>
where
    F: FnMut(usize, usize),
{
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::USER_AGENT, "ARIS-AI/1.1".parse().unwrap());
    
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        if !token.is_empty() && !token.contains("your_token") {
            // Correct format for GitHub token header is "token <TOKEN>" or "Bearer <TOKEN>"
            // The user suggested "token <token>"
            headers.insert(reqwest::header::AUTHORIZATION, format!("token {}", token).parse().unwrap());
        }
    }

    let client = Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // ── Step A: Fetch repository tree (Recursive) ──────────────────────────
    let url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/HEAD?recursive=1",
        owner, repo
    );

    let res = client.get(&url).send().await.map_err(|e| format!("Network error fetching tree: {}", e))?;
    
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("GitHub API Error (Tree) {}: {}", status, body));
    }

    let json: Value = res.json().await.map_err(|e| format!("Invalid JSON in tree: {}", e))?;
    let tree = json.get("tree").and_then(|t| t.as_array()).ok_or("No 'tree' found in GitHub response")?;
    println!("[DEBUG] GitHub API returned {} total items in tree.", tree.len());

    let code_extensions = [".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".c", ".h", ".cpp", ".java", ".swift"];

    let mut target_files: Vec<(String, u64)> = Vec::new();
    for item in tree {
        if item.get("type").and_then(|t| t.as_str()) == Some("blob") {
            if let Some(path) = item.get("path").and_then(|p| p.as_str()) {
                let path_lower = path.to_lowercase();
                
                // 1. Match valid extensions
                let has_valid_ext = code_extensions.iter().any(|ext| path_lower.ends_with(ext));
                
                // 2. Ignore heavy compiled/package/test folders
                let is_junk = path_lower.contains("node_modules") 
                    || path_lower.contains(".next") 
                    || path_lower.contains("dist") 
                    || path_lower.contains("build")
                    || path_lower.contains("/tests/")
                    || path_lower.contains("/test/");
                
                if has_valid_ext && !is_junk {
                    let size = item.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                    if size > 0 && size <= 51200 { // Max 50KB
                        println!("[DEBUG] Kept valid file: {}", path);
                        target_files.push((path.to_string(), size));
                    }
                }
            }
        }
    }
    
    // 3. PRIORITY SORT: Push "src/" and "app/" files to the top before we truncate
    target_files.sort_by(|a, b| {
        let a_is_core = a.0.contains("src/") || a.0.contains("app/");
        let b_is_core = b.0.contains("src/") || b.0.contains("app/");
        b_is_core.cmp(&a_is_core) // True comes before False
    });

    // 4. Cap at 80 files to prevent 60-second API bottlenecks
    target_files.truncate(80);
    println!("[DEBUG] Files remaining after truncation: {}", target_files.len());
    let files = target_files;
    let total_files = files.len();
    if total_files == 0 {
        return Err("No eligible code files found in repository.".to_string());
    }

    // Pre-populate Nodes
    for (path, _) in &files {
        let id = interner.intern(path);
        graph.add_node(id);
    }

    // Regex for basic import extraction across common languages
    let re_import = Regex::new(r#"(?m)^(?:use|import|from|require)\s+['"]?([\w\./-]+)"#).unwrap();

    // ── Step B: Fetch File Contents Sequentially with Rate Limiting ────────
    let mut file_contents: Vec<(String, String)> = Vec::new();

    for (i, (path, _)) in files.iter().enumerate() {
        on_progress(i + 1, total_files);

        let content_url = format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, path);
        println!("[DEBUG] Fetching content for: {}", path);
        if let Ok(res) = client.get(&content_url).send().await {
            if res.status().is_success() {
                if let Ok(file_json) = res.json::<Value>().await {
                    if let Some(content) = file_json.get("content").and_then(|c| c.as_str()) {
                        let clean_content = content.replace("\n", "").replace("\r", "");
                        use base64::{Engine as _, engine::general_purpose};
                        if let Ok(decoded) = general_purpose::STANDARD.decode(clean_content) {
                            if let Ok(text) = String::from_utf8(decoded) {
                                file_contents.push((path.clone(), text));
                            } else {
                                println!("[WARN] Could not decode UTF-8 for file: {}", path);
                            }
                        } else {
                            println!("[WARN] Could not base64 decode content for file: {}", path);
                        }
                    } else {
                        println!("[WARN] No 'content' field found for file: {}", path);
                    }
                } else {
                    println!("[WARN] Could not parse JSON for file content: {}", path);
                }
            } else {
                println!("[WARN] Failed to fetch content for {}: Status {}", path, res.status());
            }
        } else {
            println!("[WARN] Network error fetching content for: {}", path);
        }
        
        // Rate limit: 75ms sleep per file
        sleep(Duration::from_millis(75)).await;
    }

    // ── Step C: Build Dependency Edges (Imports) ──────────────────────────
    for (path, text) in &file_contents {
        let src_id = interner.intern(path);
        
        for cap in re_import.captures_iter(text) {
            let mut imp = cap[1].to_string();
            // Normalize path (remove ./ ../ and extensions)
            if imp.starts_with("./") { imp = imp[2..].to_string(); }
            if imp.starts_with("../") { imp = imp[3..].to_string(); }
            if let Some(idx) = imp.rfind('.') {
                if idx < imp.len() && code_extensions.contains(&&imp[idx..]) {
                    imp = imp[..idx].to_string();
                }
            }

            // Substring match on normalized paths in the interner
            for (target_path, _) in &files {
                if target_path != path && target_path.contains(&imp) {
                    let dst_id = interner.intern(target_path);
                    graph.add_edge(src_id, dst_id, EdgeType::Imports);
                    println!("[DEBUG] Added edge: {} -> {} (Imports)", path, target_path);
                }
            }
        }
    }

    // --- Step D: Build and return the GraphPayload ---
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for (path, _) in &files {
        let id = interner.intern(path);
        nodes.push(NodePayload { id, label: path.clone() });
    }

    // Correctly build the edges from the graph adjacency list
    for (&source_id, neighbors) in &graph.adj_out {
        for &(target_id, _) in neighbors {
            edges.push(EdgePayload {
                source: source_id,
                target: target_id,
                kind: "Imports".to_string(),
            });
        }
    }

    println!("[DEBUG] Graph built with {} nodes and {} edges.", nodes.len(), edges.len());
    Ok(GraphPayload { nodes, edges })
}
