// src/engine/dom.rs
// Árvore DOM do WebEngine — nós, atributos e mutação via JS
 
use std::collections::HashMap;
use crate::engine::css::CssProperties;
 
#[derive(Clone, Debug)]
pub struct DomNode {
    pub node_type: DomNodeType,
    pub children: Vec<DomNode>,
    pub style: CssProperties, // computed após cascade CSS
}
 
#[derive(Clone, Debug)]
pub enum DomNodeType {
    Element {
        tag: String,
        attrs: HashMap<String, String>,
        classes: Vec<String>,
        id: Option<String>,
    },
    Text(String),
}
 
impl DomNode {
    pub fn element(tag: impl Into<String>, attrs: HashMap<String, String>) -> Self {
        let tag = tag.into();
        let classes: Vec<String> = attrs
            .get("class")
            .map(|c| c.split_whitespace().map(String::from).collect())
            .unwrap_or_default();
        let id = attrs.get("id").cloned();
        DomNode {
            node_type: DomNodeType::Element { tag, attrs, classes, id },
            children: Vec::new(),
            style: CssProperties::default(),
        }
    }
 
    pub fn text(t: impl Into<String>) -> Self {
        DomNode {
            node_type: DomNodeType::Text(t.into()),
            children: Vec::new(),
            style: CssProperties::default(),
        }
    }
 
    pub fn tag(&self) -> &str {
        match &self.node_type {
            DomNodeType::Element { tag, .. } => tag.as_str(),
            DomNodeType::Text(_) => "#text",
        }
    }
 
    pub fn id(&self) -> Option<&str> {
        match &self.node_type {
            DomNodeType::Element { id, .. } => id.as_deref(),
            _ => None,
        }
    }
 
    pub fn classes(&self) -> &[String] {
        match &self.node_type {
            DomNodeType::Element { classes, .. } => classes,
            _ => &[],
        }
    }
 
    pub fn get_attr(&self, name: &str) -> Option<&str> {
        match &self.node_type {
            DomNodeType::Element { attrs, .. } => attrs.get(name).map(String::as_str),
            _ => None,
        }
    }
 
    /// Retorna o texto completo (recursivo, sem HTML)
    pub fn inner_text(&self) -> String {
        match &self.node_type {
            DomNodeType::Text(t) => t.clone(),
            DomNodeType::Element { .. } => self
                .children
                .iter()
                .map(|c| c.inner_text())
                .collect::<Vec<_>>()
                .join(""),
        }
    }
 
    /// Elemento é block-level? (considera UA defaults + computed style)
    pub fn is_block(&self) -> bool {
        match self.tag() {
            "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            | "ul" | "ol" | "li" | "blockquote" | "pre" | "hr"
            | "table" | "thead" | "tbody" | "tr"
            | "section" | "article" | "header" | "footer" | "main"
            | "nav" | "aside" | "figure" | "figcaption"
            | "form" | "fieldset" | "address" | "details" | "summary" => true,
            "#text" => false,
            _ => matches!(&self.style.display,
                Some(d) if matches!(d.as_str(), "block" | "table" | "list-item" | "flex" | "grid")
            ),
        }
    }
 
    pub fn is_visible(&self) -> bool {
        if let Some(d) = &self.style.display {
            if d == "none" { return false; }
        }
        if let Some(v) = &self.style.visibility {
            if v == "hidden" { return false; }
        }
        true
    }
 
    // ── Mutações DOM (usadas pelo motor JS) ─────────────────────────────────
 
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut DomNode> {
        if self.id() == Some(id) { return Some(self); }
        for child in &mut self.children {
            if let Some(n) = child.find_by_id_mut(id) { return Some(n); }
        }
        None
    }
 
    pub fn find_by_selector_mut(&mut self, sel: &str) -> Option<&mut DomNode> {
        let sel = sel.trim().trim_matches('"').trim_matches('\'');
        if sel.starts_with('#') {
            return self.find_by_id_mut(&sel[1..]);
        }
        if sel.starts_with('.') {
            return self.find_by_class_mut(&sel[1..]);
        }
        self.find_by_tag_mut(sel)
    }
 
    fn find_by_class_mut(&mut self, class: &str) -> Option<&mut DomNode> {
        if self.classes().iter().any(|c| c == class) { return Some(self); }
        for child in &mut self.children {
            if let Some(n) = child.find_by_class_mut(class) { return Some(n); }
        }
        None
    }
 
    fn find_by_tag_mut(&mut self, tag: &str) -> Option<&mut DomNode> {
        if self.tag() == tag { return Some(self); }
        for child in &mut self.children {
            if let Some(n) = child.find_by_tag_mut(tag) { return Some(n); }
        }
        None
    }
 
    pub fn set_inner_html(&mut self, html: &str) {
        // Converte HTML simples para texto (sem parser completo)
        let text = html
            .replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n")
            .replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
            .replace("&nbsp;", " ").replace("&quot;", "\"");
        // Strip remaining tags
        let mut out = String::new();
        let mut in_tag = false;
        for c in text.chars() {
            match c {
                '<' => { in_tag = true; }
                '>' => { in_tag = false; }
                _ if !in_tag => out.push(c),
                _ => {}
            }
        }
        self.children = vec![DomNode::text(out)];
    }
 
    pub fn set_text_content(&mut self, text: &str) {
        self.children = vec![DomNode::text(text.to_string())];
    }
 
    pub fn add_class(&mut self, class: &str) {
        if let DomNodeType::Element { ref mut classes, .. } = self.node_type {
            if !classes.iter().any(|c| c == class) {
                classes.push(class.to_string());
            }
        }
    }
 
    pub fn remove_class(&mut self, class: &str) {
        if let DomNodeType::Element { ref mut classes, .. } = self.node_type {
            classes.retain(|c| c != class);
        }
    }
}
 