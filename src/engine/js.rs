// src/engine/js.rs
// Interpretador JS minimalista — cobre ~70% dos scripts reais simples
// Não é um engine completo: parse por pattern-matching linha a linha
 
use crate::engine::dom::DomNode;
use crate::engine::css::parse_declarations;
 
/// Executa scripts inline contra o DOM
pub fn execute_scripts(dom: &mut DomNode, scripts: &str) {
    // Remove comentários de linha
    let cleaned: Vec<String> = scripts
        .lines()
        .map(|l| {
            if let Some(pos) = l.find("//") {
                // Não remover se estiver dentro de string
                let before = &l[..pos];
                let in_string = before.chars().filter(|&c| c == '"' || c == '\'').count() % 2 == 1;
                if in_string { l.to_string() } else { before.to_string() }
            } else {
                l.to_string()
            }
        })
        .collect();
 
    // Processa statement a statement (simplificação: uma linha = um statement)
    let mut i = 0;
    let lines = cleaned.clone();
    while i < lines.len() {
        let line = lines[i].trim();
        if !line.is_empty() {
            exec_line(dom, line);
        }
        i += 1;
    }
}
 
fn exec_line(dom: &mut DomNode, line: &str) {
    let line = line.trim_end_matches(';').trim();
    if line.is_empty() || line.starts_with("//") || line.starts_with("/*") { return; }
 
    // document.getElementById("id").<prop> = <value>
    if let Some(rest) = strip_prefix_ci(line, "document.getelementbyid(") {
        if let Some((id, prop_path, value)) = parse_dom_mutation(rest) {
            apply_mutation(dom.find_by_id_mut(&id), &prop_path, &value);
        }
        return;
    }
 
    // document.querySelector("sel").<prop> = <value>
    if let Some(rest) = strip_prefix_ci(line, "document.queryselector(") {
        if let Some((sel, prop_path, value)) = parse_dom_mutation(rest) {
            let node = dom.find_by_selector_mut(&sel);
            apply_mutation(node, &prop_path, &value);
        }
        return;
    }
 
    // document.title = "..."
    if let Some(rest) = strip_prefix_ci(line, "document.title") {
        if let Some(val) = extract_rhs(rest) {
            if let Some(node) = dom.find_by_selector_mut("title") {
                node.set_text_content(&val);
            }
        }
        return;
    }
 
    // document.body.style.<prop> = "..."
    if let Some(rest) = strip_prefix_ci(line, "document.body.style.") {
        if let Some(eq) = rest.find('=') {
            let prop = rest[..eq].trim().to_string();
            if let Some(val) = extract_rhs(&rest[eq..]) {
                if let Some(body) = dom.find_by_selector_mut("body") {
                    let decl = format!("{}:{}", prop, val);
                    let props = parse_declarations(&decl);
                    merge_css(body, &props);
                }
            }
        }
        return;
    }
 
    // Ignora: var/let/const, function, return, console.log, etc.
}
 
/// Parseia: "id").style.prop = "value" ou "id").innerHTML = "value"
fn parse_dom_mutation(s: &str) -> Option<(String, String, String)> {
    let s = s.trim();
    // Extrai o seletor/id entre aspas
    let (id, rest) = extract_quoted_arg(s)?;
    let rest = rest.trim().strip_prefix(')')?.trim();
    // Pega o caminho da propriedade até '='
    let eq_pos = rest.find('=')?;
    let prop_path = rest[..eq_pos].trim().trim_start_matches('.').to_string();
    let value = extract_rhs(&rest[eq_pos..])?;
    Some((id, prop_path, value))
}
 
fn apply_mutation(node: Option<&mut DomNode>, prop_path: &str, value: &str) {
    let node = match node { Some(n) => n, None => return };
    let prop = prop_path.to_lowercase();
 
    // .style.<property> (ex: style.display, style.color)
    if let Some(css_prop) = prop.strip_prefix("style.") {
        let decl = format!("{}:{}", css_prop.replace('-', "-"), value);
        let props = parse_declarations(&decl);
        merge_css(node, &props);
        return;
    }
 
    match prop.as_str() {
        "innerhtml" => node.set_inner_html(value),
        "textcontent" | "innertext" => node.set_text_content(value),
        "classname" => {
            if let crate::engine::dom::DomNodeType::Element { ref mut classes, .. } = node.node_type {
                *classes = value.split_whitespace().map(String::from).collect();
            }
        }
        "hidden" => {
            if value == "true" || value == "1" {
                node.style.display = Some("none".to_string());
            } else {
                node.style.display = Some("block".to_string());
            }
        }
        // .classList.add / .classList.remove — chegam como "classlist.add" com arg
        p if p.starts_with("classlist.") => {
            let action = &p["classlist.".len()..];
            let cls = value.trim().trim_matches('"').trim_matches('\'').to_string();
            match action {
                "add" => node.add_class(&cls),
                "remove" => node.remove_class(&cls),
                "toggle" => {
                    if node.classes().iter().any(|c| c == &cls) {
                        node.remove_class(&cls);
                    } else {
                        node.add_class(&cls);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}
 
fn merge_css(node: &mut DomNode, props: &crate::engine::css::CssProperties) {
    let s = &mut node.style;
    if let Some(v) = props.color { s.color = Some(v); }
    if let Some(v) = props.background_color { s.background_color = Some(v); }
    if let Some(v) = props.font_size { s.font_size = Some(v); }
    if let Some(v) = props.font_weight { s.font_weight = Some(v); }
    if let Some(ref v) = props.font_style.clone() { s.font_style = Some(v.clone()); }
    if let Some(ref v) = props.display.clone() { s.display = Some(v.clone()); }
    if let Some(ref v) = props.text_decoration.clone() { s.text_decoration = Some(v.clone()); }
    if let Some(v) = props.margin_top { s.margin_top = Some(v); }
    if let Some(v) = props.margin_bottom { s.margin_bottom = Some(v); }
    if let Some(v) = props.margin_left { s.margin_left = Some(v); }
    if let Some(v) = props.margin_right { s.margin_right = Some(v); }
    if let Some(v) = props.padding_top { s.padding_top = Some(v); }
    if let Some(v) = props.padding_bottom { s.padding_bottom = Some(v); }
    if let Some(v) = props.padding_left { s.padding_left = Some(v); }
    if let Some(v) = props.padding_right { s.padding_right = Some(v); }
    if let Some(v) = props.border_radius { s.border_radius = Some(v); }
    if let Some(v) = props.border_color { s.border_color = Some(v); }
    if let Some(v) = props.border_width { s.border_width = Some(v); }
    if let Some(v) = props.opacity { s.opacity = Some(v); }
}
 
// ── Helpers de parsing ────────────────────────────────────────────────────────
 
/// Extrai o primeiro argumento string entre aspas
fn extract_quoted_arg(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    let q = if s.starts_with('"') { '"' } else if s.starts_with('\'') { '\'' } else { return None; };
    let rest = &s[1..];
    let end = rest.find(q)?;
    Some((rest[..end].to_string(), rest[end + 1..].to_string()))
}
 
/// Extrai o valor à direita de um '='
fn extract_rhs(s: &str) -> Option<String> {
    let s = s.trim().strip_prefix('=')?.trim();
    let s = s.trim_end_matches(';').trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        Some(s[1..s.len() - 1].to_string())
    } else {
        Some(s.to_string())
    }
}
 
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() { return None; }
    if s[..prefix.len()].to_lowercase() == prefix.to_lowercase() {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}