// src/ui/renderer.rsrenderer.rs
// Renderer egui — percorre a árvore DOM e aplica CSS com layout block/inline
 
use eframe::egui;
use crate::engine::dom::{DomNode, DomNodeType};
 
// ── Estilos herdados (propagados de pai para filho) ───────────────────────────
 
#[derive(Clone)]
pub struct Inherited {
    pub color: egui::Color32,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub monospace: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub link_url: Option<String>,
}
 
impl Default for Inherited {
    fn default() -> Self {
        Self {
            color: egui::Color32::from_rgb(220, 220, 228),
            font_size: 16.0,
            bold: false,
            italic: false,
            monospace: false,
            underline: false,
            strikethrough: false,
            link_url: None,
        }
    }
}
 
/// Ponto de entrada — renderiza o documento inteiro
/// Retorna a URL clicada, se houver
pub fn render(ui: &mut egui::Ui, dom: &DomNode) -> Option<String> {
    render_node(ui, dom, &Inherited::default())
}
 
// ── Renderização de nós ───────────────────────────────────────────────────────
 
fn render_node(ui: &mut egui::Ui, node: &DomNode, inh: &Inherited) -> Option<String> {
    if !node.is_visible() { return None; }
    match &node.node_type {
        DomNodeType::Text(_) => None, // Texto é tratado pelo pai (inline)
        DomNodeType::Element { tag, .. } => render_element(ui, node, tag.as_str(), inh),
    }
}
 
fn render_element(ui: &mut egui::Ui, node: &DomNode, tag: &str, inh: &Inherited) -> Option<String> {
    let inh = compute_inherited(inh, node, tag);
    let props = &node.style;
 
    match tag {
        // Elementos sem renderização visual
        "head" | "meta" | "link" | "noscript" | "script" | "style"
        | "template" | "svg" | "canvas" => return None,
 
        // Raiz do documento
        "html" | "body" => return render_block_children(ui, node, &inh),
 
        // Quebra de linha
        "br" => { ui.add_space(4.0); return None; }
 
        // Divisor horizontal
        "hr" => {
            ui.add_space(props.margin_top.unwrap_or(8.0));
            ui.separator();
            ui.add_space(props.margin_bottom.unwrap_or(8.0));
            return None;
        }
 
        // Imagem — placeholder com texto alt
        "img" => {
            let alt = node.get_attr("alt").unwrap_or("imagem");
            let label = if alt.is_empty() { "[img]".to_string() } else { format!("[{}]", alt) };
            ui.label(egui::RichText::new(label)
                .color(egui::Color32::from_rgb(100, 100, 130))
                .italics()
                .size(13.0));
            return None;
        }
 
        // Bloco de código / pré-formatado
        "pre" => {
            let bg = props.background_color.unwrap_or(egui::Color32::from_rgb(22, 22, 30));
            let pad = props.padding_left.unwrap_or(14.0);
            ui.add_space(props.margin_top.unwrap_or(10.0));
            egui::Frame::none()
                .fill(bg)
                .rounding(props.border_radius.unwrap_or(6.0))
                .inner_margin(egui::Margin::same(pad))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(node.inner_text())
                            .monospace()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(190, 200, 215)),
                    );
                });
            ui.add_space(props.margin_bottom.unwrap_or(10.0));
            return None;
        }
 
        // Listas
        "ul" | "ol" => {
            ui.add_space(props.margin_top.unwrap_or(8.0));
            let pad = props.padding_left.unwrap_or(24.0);
            let mut nav = None;
            let mut counter = 1usize;
            for child in &node.children {
                if child.tag() == "li" {
                    let bullet = if tag == "ul" {
                        "•".to_string()
                    } else {
                        let s = format!("{}.", counter);
                        counter += 1;
                        s
                    };
                    let child_inh = compute_inherited(&inh, child, "li");
                    let inner = ui.horizontal(|ui| {
                        ui.add_space(pad);
                        ui.label(
                            egui::RichText::new(&bullet)
                                .color(child_inh.color)
                                .size(child_inh.font_size),
                        );
                        ui.add_space(6.0);
                        render_inline_content(ui, child, &child_inh)
                    });
                    if nav.is_none() { nav = inner.inner; }
                    ui.add_space(props.gap.unwrap_or(4.0));
                }
            }
            ui.add_space(props.margin_bottom.unwrap_or(8.0));
            return nav;
        }
 
        // Citação com borda esquerda
        "blockquote" => {
            let ml = props.margin_left.unwrap_or(0.0);
            let border_c = props.border_color.unwrap_or(egui::Color32::from_rgb(80, 55, 140));
            let border_w = props.border_width.unwrap_or(4.0);
            ui.add_space(props.margin_top.unwrap_or(10.0));
            let inner = ui.horizontal(|ui| -> Option<String> {
                ui.add_space(ml);
                // Barra vertical colorida
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(border_w, ui.available_height().max(20.0)),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(rect, 0.0, border_c);
                ui.add_space(10.0);
                ui.vertical(|ui| render_block_children(ui, node, &inh)).inner
            });
            ui.add_space(props.margin_bottom.unwrap_or(10.0));
            return inner.inner;
        }
 
        // Tabela
        "table" => {
            ui.add_space(props.margin_top.unwrap_or(6.0));
            let nav = render_table(ui, node, &inh);
            ui.add_space(props.margin_bottom.unwrap_or(6.0));
            return nav;
        }
        "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" => {
            // Tratados por render_table
            return render_block_children(ui, node, &inh);
        }
 
        // Headings
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            ui.add_space(props.margin_top.unwrap_or(18.0));
            let nav = render_inline_content(ui, node, &inh);
            ui.add_space(props.margin_bottom.unwrap_or(8.0));
            return nav;
        }
 
        // Parágrafo
        "p" => {
            ui.add_space(props.margin_top.unwrap_or(8.0));
            let nav = render_inline_content(ui, node, &inh);
            ui.add_space(props.margin_bottom.unwrap_or(8.0));
            return nav;
        }
 
        // Containers (div, section, article, etc.)
        "div" | "section" | "article" | "main" | "header" | "footer"
        | "nav" | "aside" | "figure" | "figcaption" | "form" | "fieldset"
        | "address" | "details" | "summary" => {
            let mt = props.margin_top.unwrap_or(0.0);
            let mb = props.margin_bottom.unwrap_or(0.0);
            if mt > 0.0 { ui.add_space(mt); }
 
            let nav = if let Some(bg) = props.background_color {
                let m = egui::Margin {
                    left: props.padding_left.unwrap_or(0.0),
                    right: props.padding_right.unwrap_or(0.0),
                    top: props.padding_top.unwrap_or(0.0),
                    bottom: props.padding_bottom.unwrap_or(0.0),
                };
                let inner = egui::Frame::none()
                    .fill(bg)
                    .rounding(props.border_radius.unwrap_or(0.0))
                    .inner_margin(m)
                    .show(ui, |ui| render_block_children(ui, node, &inh));
                inner.inner
            } else {
                let pt = props.padding_top.unwrap_or(0.0);
                let pb = props.padding_bottom.unwrap_or(0.0);
                let pl = props.padding_left.unwrap_or(0.0);
                if pt > 0.0 { ui.add_space(pt); }
                let nav = if pl > 0.0 {
                    let inner = ui.horizontal(|ui| -> Option<String> {
                        ui.add_space(pl);
                        ui.vertical(|ui| render_block_children(ui, node, &inh)).inner
                    });
                    inner.inner
                } else {
                    render_block_children(ui, node, &inh)
                };
                if pb > 0.0 { ui.add_space(pb); }
                nav
            };
 
            if mb > 0.0 { ui.add_space(mb); }
            return nav;
        }
 
        // input, button e afins — placeholder simples
        "input" => {
            let tipo = node.get_attr("type").unwrap_or("text");
            let placeholder = node.get_attr("placeholder").unwrap_or("");
            let value = node.get_attr("value").unwrap_or("");
            let display = if !value.is_empty() { value } else { placeholder };
            ui.label(
                egui::RichText::new(format!("[{}{}]", tipo, if display.is_empty() { "".to_string() } else { format!(": {}", display) }))
                    .color(egui::Color32::from_rgb(120, 120, 140))
                    .size(13.0),
            );
            return None;
        }
 
        "button" => {
            let text = node.inner_text();
            if !text.trim().is_empty() {
                ui.add(egui::Button::new(
                    egui::RichText::new(text.trim())
                        .size(inh.font_size)
                        .color(egui::Color32::from_rgb(220, 220, 235)),
                ));
            }
            return None;
        }
 
        // Fallback
        _ => {
            if node.is_block() {
                return render_block_children(ui, node, &inh);
            } else {
                return render_inline_content(ui, node, &inh);
            }
        }
    }
}
 
// ── Layout block: agrupa inline adjacentes e renderiza blocos ─────────────────
 
fn render_block_children(ui: &mut egui::Ui, node: &DomNode, inh: &Inherited) -> Option<String> {
    let mut nav: Option<String> = None;
    let mut inline_buf: Vec<&DomNode> = Vec::new();
 
    let flush = |ui: &mut egui::Ui, buf: &[&DomNode], inh: &Inherited| -> Option<String> {
        let mut segs = Vec::new();
        for n in buf { collect_segs(n, inh, &mut segs); }
        render_segments(ui, &segs)
    };
 
    for child in &node.children {
        if child.is_block() {
            // Descarrega inline acumulado antes do bloco
            if !inline_buf.is_empty() {
                if let Some(url) = flush(ui, &inline_buf, inh) { nav = Some(url); }
                inline_buf.clear();
            }
            if let Some(url) = render_node(ui, child, inh) { nav = Some(url); }
        } else {
            inline_buf.push(child);
        }
    }
    if !inline_buf.is_empty() {
        if let Some(url) = flush(ui, &inline_buf, inh) { nav = Some(url); }
    }
    nav
}
 
// ── Inline: coleta segmentos de texto e renderiza ────────────────────────────
 
/// Segmento de texto com formatação e URL opcional (link)
type Seg = (String, Inherited, Option<String>);
 
fn render_inline_content(ui: &mut egui::Ui, node: &DomNode, inh: &Inherited) -> Option<String> {
    let mut segs: Vec<Seg> = Vec::new();
    collect_segs(node, inh, &mut segs);
    render_segments(ui, &segs)
}
 
/// Renderiza segmentos: com link → ui.link, sem → LayoutJob fluído
fn render_segments(ui: &mut egui::Ui, segs: &[Seg]) -> Option<String> {
    // Remove trailing spaces
    let segs: Vec<&Seg> = segs.iter()
        .filter(|(t, _, _)| !t.trim().is_empty())
        .collect();
    if segs.is_empty() { return None; }
 
    let has_links = segs.iter().any(|(_, _, url)| url.is_some());
    let mut nav: Option<String> = None;
 
    if has_links {
        // Renderiza como sequência de widgets inline
        let inner = ui.horizontal_wrapped(|ui| -> Option<String> {
            let mut nav = None;
            for (text, inh, url) in &segs {
                let rt = make_rich(text, inh);
                if let Some(link_url) = url {
                    if ui.link(rt).clicked() {
                        nav = Some(link_url.clone());
                    }
                } else {
                    ui.label(rt);
                }
            }
            nav
        });
        nav = inner.inner;
    } else {
        // LayoutJob para texto com formatação mista (wrap perfeito)
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = ui.available_width();
        job.wrap.break_anywhere = false;
 
        for (text, inh, _) in &segs {
            let fmt = egui::text::TextFormat {
                font_id: if inh.monospace {
                    egui::FontId::monospace(inh.font_size)
                } else {
                    egui::FontId::proportional(inh.font_size)
                },
                color: inh.color,
                italics: inh.italic,
                underline: if inh.underline {
                    egui::Stroke::new(1.0, inh.color)
                } else {
                    egui::Stroke::NONE
                },
                strikethrough: if inh.strikethrough {
                    egui::Stroke::new(1.0, inh.color)
                } else {
                    egui::Stroke::NONE
                },
                ..Default::default()
            };
            // Adiciona espaço entre segmentos se necessário
            let sep = if job.text.is_empty() { "" } else { " " };
            job.append(&format!("{}{}", sep, text), 0.0, fmt);
        }
 
        if !job.text.is_empty() {
            ui.label(job);
        }
    }
    nav
}
 
/// Coleta segmentos de texto recursivamente percorrendo os filhos inline
fn collect_segs(node: &DomNode, inh: &Inherited, out: &mut Vec<Seg>) {
    match &node.node_type {
        DomNodeType::Text(t) => {
            let t = t.trim();
            if !t.is_empty() {
                out.push((t.to_string(), inh.clone(), inh.link_url.clone()));
            }
        }
        DomNodeType::Element { tag, .. } => {
            if !node.is_visible() { return; }
            if tag == "br" {
                out.push(("\n".to_string(), inh.clone(), None));
                return;
            }
            if matches!(tag.as_str(), "script" | "style" | "head") { return; }
            let new = compute_inherited(inh, node, tag.as_str());
            for child in &node.children {
                collect_segs(child, &new, out);
            }
        }
    }
}
 
// ── Tabela ────────────────────────────────────────────────────────────────────
 
fn render_table(ui: &mut egui::Ui, table: &DomNode, inh: &Inherited) -> Option<String> {
    // Extrai linhas da tabela navegando thead/tbody/tr
    let mut rows: Vec<Vec<&DomNode>> = Vec::new();
    collect_table_rows(table, &mut rows);
    if rows.is_empty() { return None; }
 
    let mut nav: Option<String> = None;
    let id = egui::Id::new(table as *const DomNode);
 
    egui::Grid::new(id)
        .striped(true)
        .spacing(egui::vec2(12.0, 6.0))
        .show(ui, |ui| {
            for row in &rows {
                for cell in row.iter() {
                    let cell_inh = compute_inherited(inh, cell, cell.tag());
                    if let Some(url) = render_inline_content(ui, cell, &cell_inh) {
                        nav = Some(url);
                    }
                }
                ui.end_row();
            }
        });
    nav
}
 
fn collect_table_rows<'a>(node: &'a DomNode, rows: &mut Vec<Vec<&'a DomNode>>) {
    for child in &node.children {
        match child.tag() {
            "thead" | "tbody" | "tfoot" => collect_table_rows(child, rows),
            "tr" => {
                let cells: Vec<&DomNode> = child.children.iter()
                    .filter(|c| matches!(c.tag(), "td" | "th"))
                    .collect();
                if !cells.is_empty() { rows.push(cells); }
            }
            _ => {}
        }
    }
}
 
// ── Herança e formatação de estilos ─────────────────────────────────────────
 
fn compute_inherited(inh: &Inherited, node: &DomNode, tag: &str) -> Inherited {
    let mut new = inh.clone();
    let props = &node.style;
 
    // Aplica props computados (CSS cascade já resolveu)
    if let Some(c) = props.color { new.color = c; }
    if let Some(s) = props.font_size { new.font_size = s; }
    if let Some(w) = props.font_weight { new.bold = w >= 600; }
    if let Some(ref s) = props.font_style {
        new.italic = matches!(s.as_str(), "italic" | "oblique");
    }
    if let Some(ref d) = props.text_decoration {
        new.underline = d.contains("underline");
        new.strikethrough = d.contains("line-through");
    }
 
    // Semântica de tag (UA defaults inline)
    match tag {
        "a" => {
            new.color = props.color.unwrap_or(egui::Color32::from_rgb(80, 145, 245));
            new.underline = true;
            if let Some(href) = node.get_attr("href") {
                if !href.starts_with('#') && !href.starts_with("javascript:") && !href.is_empty() {
                    new.link_url = Some(href.to_string());
                }
            }
        }
        "strong" | "b" => { new.bold = true; }
        "em" | "i" | "cite" | "var" => { new.italic = true; }
        "code" | "kbd" | "samp" | "tt" | "var" => { new.monospace = true; }
        "small" => { new.font_size = (new.font_size * 0.85).max(10.0); }
        "big" => { new.font_size = (new.font_size * 1.15).min(40.0); }
        "sup" | "sub" => { new.font_size = (new.font_size * 0.75).max(9.0); }
        "del" | "s" => { new.strikethrough = true; }
        "ins" | "u" => { new.underline = true; }
        "mark" => { /* highlight — não trivial em egui, ignora por enquanto */ }
        _ => {}
    }
 
    new
}
 
fn make_rich(text: &str, inh: &Inherited) -> egui::RichText {
    let mut rt = egui::RichText::new(text)
        .color(inh.color)
        .size(inh.font_size);
    if inh.bold { rt = rt.strong(); }
    if inh.italic { rt = rt.italics(); }
    if inh.monospace { rt = rt.monospace(); }
    rt
}
 