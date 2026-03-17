// src/engine/mod.rs
// Pipeline principal: fetch HTML → DOM tree → CSS → JS → página renderizável
 
pub mod dom;
pub mod css;
pub mod js;
 
use std::collections::HashMap;
use scraper::{Html, ElementRef};
use scraper::node::Node;
use reqwest::blocking::Client;
use std::time::Duration;
use url::Url;
 
pub use dom::DomNode;
pub use css::StyleSheet;
 
/// Resultado completo do parsing de uma página
pub struct ParsedPage {
    pub dom: DomNode,
    pub stylesheet: StyleSheet,
    pub title: String,
    pub url: String,
}
 
/// Busca e parseia uma URL completa
pub fn fetch_page(url: &str) -> ParsedPage {
    let html = fetch_html(url);
    parse_page(&html, url)
}
 
/// Parseia HTML já em memória (útil para testes)
pub fn parse_page(html_str: &str, base_url: &str) -> ParsedPage {
    let mut css_buf = String::new();
    let mut js_buf = String::new();
    let mut title = String::new();
 
    let document = Html::parse_document(html_str);
    let root = document.root_element();
 
    let mut dom = build_element(root, base_url, &mut css_buf, &mut js_buf, &mut title);
 
    // 1. Aplica CSS
    let stylesheet = StyleSheet::parse(&css_buf);
    apply_styles(&mut dom, &stylesheet);
 
    // 2. Executa JS (muta o DOM)
    if !js_buf.trim().is_empty() {
        js::execute_scripts(&mut dom, &js_buf);
        // Re-aplica estilos pois JS pode ter mudado classes
        apply_styles(&mut dom, &stylesheet);
    }
 
    if title.is_empty() {
        title = base_url.to_string();
    }
 
    ParsedPage { dom, stylesheet, title, url: base_url.to_string() }
}
 
/// Aplica CSS cascade a toda a árvore DOM
pub fn apply_styles(node: &mut DomNode, sheet: &StyleSheet) {
    if let dom::DomNodeType::Element { ref tag, ref classes, ref id, .. } = node.node_type.clone() {
        let inline = node.get_attr("style").map(String::from);
        node.style = sheet.compute(tag, classes, id.as_deref(), inline.as_deref());
    }
    for child in &mut node.children {
        apply_styles(child, sheet);
    }
}
 
// ── Builder de DOM a partir do scraper ────────────────────────────────────────
 
fn build_element(
    elem: ElementRef,
    base_url: &str,
    css: &mut String,
    js: &mut String,
    title: &mut String,
) -> DomNode {
    let tag = elem.value().name().to_lowercase();
 
    match tag.as_str() {
        // <style> → coleta CSS, não vira nó
        "style" => {
            css.push_str(&elem.text().collect::<String>());
            css.push('\n');
            return DomNode::text("");
        }
        // <script> inline → coleta JS
        "script" => {
            if elem.value().attr("src").is_none() {
                let src = elem.text().collect::<String>();
                js.push_str(&src);
                js.push('\n');
            }
            return DomNode::text("");
        }
        // <head> → só extrai metadados, não renderiza
        "head" => {
            for child in elem.children() {
                if let Node::Element(_) = child.value() {
                    if let Some(c) = ElementRef::wrap(child) {
                        build_element(c, base_url, css, js, title);
                    }
                }
            }
            return DomNode::text("");
        }
        // <title> → captura texto do título
        "title" => {
            *title = elem.text().collect::<String>().trim().to_string();
            return DomNode::text("");
        }
        _ => {}
    }
 
    // Constrói mapa de atributos
    let mut attrs: HashMap<String, String> = elem
        .value()
        .attrs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
 
    // Resolve URLs relativas
    match tag.as_str() {
        "a" => {
            if let Some(href) = attrs.get("href").cloned() {
                attrs.insert("href".to_string(), resolve_url(&href, base_url));
            }
        }
        "img" => {
            if let Some(src) = attrs.get("src").cloned() {
                attrs.insert("src".to_string(), resolve_url(&src, base_url));
            }
        }
        _ => {}
    }
 
    let mut node = DomNode::element(tag, attrs);
 
    // Percorre filhos
    for child in elem.children() {
        match child.value() {
            Node::Element(_) => {
                if let Some(child_ref) = ElementRef::wrap(child) {
                    let child_node = build_element(child_ref, base_url, css, js, title);
                    // Filtra nós vazios de placeholder
                    match &child_node.node_type {
                        dom::DomNodeType::Text(t) if t.is_empty() => {}
                        _ => node.children.push(child_node),
                    }
                }
            }
            Node::Text(t) => {
                // Normaliza whitespace, descarta nós só de espaço
                let text = t.to_string();
                let text = text.trim();
                if !text.is_empty() {
                    node.children.push(DomNode::text(text.to_string()));
                }
            }
            _ => {}
        }
    }
 
    node
}
 
// ── Utilitários de rede e URL ──────────────────────────────────────────────────
 
pub fn fetch_html(url: &str) -> String {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36")
        .build()
        .unwrap_or_else(|_| Client::new());
 
    match client.get(url).send() {
        Ok(res) if res.status().is_success() => {
            res.text().unwrap_or_else(|_| html_error("Erro ao ler corpo da página"))
        }
        Ok(res) => html_error(&format!("Erro HTTP {}", res.status())),
        Err(e) => html_error(&format!("Falha de conexão: {}", e)),
    }
}
 
fn html_error(msg: &str) -> String {
    format!(
        "<!DOCTYPE html><html><body><div style='padding:40px;color:#ff6060;font-family:sans-serif'>\
        <h2>⚠ Erro de carregamento</h2><p>{}</p></div></body></html>",
        msg
    )
}
 
pub fn resolve_url(href: &str, base: &str) -> String {
    let href = href.trim();
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if href.starts_with("//") {
        return format!("https:{}", href);
    }
    if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") || href.starts_with("mailto:") {
        return href.to_string();
    }
    if let Ok(base_url) = Url::parse(base) {
        if let Ok(resolved) = base_url.join(href) {
            return resolved.to_string();
        }
    }
    href.to_string()
}
 
pub fn resolve_smart_query(input: &str) -> String {
    let input = input.trim();
    if input.starts_with("http://") || input.starts_with("https://") {
        return input.to_string();
    }
    let is_domain = (input.contains('.') && !input.contains(' ')) || input.starts_with("localhost");
    if is_domain {
        format!("https://{}", input)
    } else {
        format!("https://duckduckgo.com/html/?q={}", input.replace(' ', "+"))
    }
}
 