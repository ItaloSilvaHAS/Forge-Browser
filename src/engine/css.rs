/ src/engine/css.rs
// Motor CSS do WebEngine — parse de propriedades, seletores e cascade
 
use eframe::egui;
 
/// Todas as propriedades CSS que suportamos
#[derive(Clone, Debug, Default)]
pub struct CssProperties {
    pub color: Option<egui::Color32>,
    pub background_color: Option<egui::Color32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u32>,
    pub font_style: Option<String>,
    pub text_align: Option<String>,
    pub text_decoration: Option<String>,
    pub display: Option<String>,
    pub visibility: Option<String>,
    pub margin_top: Option<f32>,
    pub margin_bottom: Option<f32>,
    pub margin_left: Option<f32>,
    pub margin_right: Option<f32>,
    pub padding_top: Option<f32>,
    pub padding_bottom: Option<f32>,
    pub padding_left: Option<f32>,
    pub padding_right: Option<f32>,
    pub border_radius: Option<f32>,
    pub border_color: Option<egui::Color32>,
    pub border_width: Option<f32>,
    pub opacity: Option<f32>,
    pub gap: Option<f32>,
}
 
/// Um seletor CSS
#[derive(Clone, Debug)]
pub enum CssSelector {
    Universal,
    Type(String),
    Class(String),
    Id(String),
    TypeClass(String, String),
    TypeId(String, String),
}
 
/// Uma regra CSS
pub struct CssRule {
    pub selectors: Vec<CssSelector>,
    pub properties: CssProperties,
    pub specificity: u32,
}
 
/// Stylesheet completa parseada
pub struct StyleSheet {
    pub rules: Vec<CssRule>,
}
 
impl StyleSheet {
    pub fn new() -> Self {
        StyleSheet { rules: Vec::new() }
    }
 
    pub fn parse(css: &str) -> Self {
        let mut sheet = StyleSheet::new();
        let css = remove_comments(css);
        let mut pos = 0;
        let bytes = css.as_bytes();
 
        while pos < css.len() {
            // Pula whitespace
            while pos < css.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos >= css.len() { break; }
 
            // Pula @rules
            if pos < css.len() && bytes[pos] == b'@' {
                if let Some(end) = skip_at_rule(&css[pos..]) {
                    pos += end;
                } else {
                    break;
                }
                continue;
            }
 
            if let Some(brace) = css[pos..].find('{') {
                let sel_str = css[pos..pos + brace].trim();
                let rest = &css[pos + brace + 1..];
                if let Some(end) = rest.find('}') {
                    let decls = &rest[..end];
                    let selectors = parse_selectors(sel_str);
                    let properties = parse_declarations(decls);
                    if !selectors.is_empty() {
                        let specificity = calc_specificity(&selectors[0]);
                        sheet.rules.push(CssRule { selectors, properties, specificity });
                    }
                    pos += brace + 1 + end + 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
 
        sheet
    }
 
    /// Computa as propriedades para um elemento, com cascade correto
    pub fn compute(&self, tag: &str, classes: &[String], id: Option<&str>, inline: Option<&str>) -> CssProperties {
        let mut result = CssProperties::default();
 
        // 1. User-agent stylesheet
        ua_styles(&mut result, tag);
 
        // 2. Regras da página, ordenadas por especificidade
        let mut matching: Vec<(&CssRule, u32)> = self
            .rules
            .iter()
            .filter_map(|r| {
                r.selectors
                    .iter()
                    .filter(|s| selector_matches(s, tag, classes, id))
                    .map(|s| calc_specificity(s))
                    .max()
                    .map(|spec| (r, spec))
            })
            .collect();
        matching.sort_by_key(|(_, s)| *s);
        for (rule, _) in matching {
            merge(&mut result, &rule.properties);
        }
 
        // 3. Estilo inline tem prioridade máxima
        if let Some(s) = inline {
            if !s.is_empty() {
                let inline_props = parse_declarations(s);
                merge(&mut result, &inline_props);
            }
        }
 
        result
    }
}
 
// ── User-Agent Stylesheet ─────────────────────────────────────────────────────
 
fn ua_styles(p: &mut CssProperties, tag: &str) {
    match tag {
        "h1" => { p.font_size = Some(32.0); p.font_weight = Some(700); p.margin_top = Some(20.0); p.margin_bottom = Some(20.0); p.display = Some("block".into()); }
        "h2" => { p.font_size = Some(24.0); p.font_weight = Some(700); p.margin_top = Some(18.0); p.margin_bottom = Some(18.0); p.display = Some("block".into()); }
        "h3" => { p.font_size = Some(19.0); p.font_weight = Some(700); p.margin_top = Some(16.0); p.margin_bottom = Some(16.0); p.display = Some("block".into()); }
        "h4" | "h5" | "h6" => { p.font_size = Some(16.0); p.font_weight = Some(700); p.margin_top = Some(14.0); p.margin_bottom = Some(14.0); p.display = Some("block".into()); }
        "p" => { p.margin_top = Some(14.0); p.margin_bottom = Some(14.0); p.display = Some("block".into()); }
        "a" => { p.color = Some(egui::Color32::from_rgb(80, 145, 245)); p.text_decoration = Some("underline".into()); p.display = Some("inline".into()); }
        "strong" | "b" => { p.font_weight = Some(700); }
        "em" | "i" => { p.font_style = Some("italic".into()); }
        "small" => { p.font_size = Some(12.0); }
        "code" | "kbd" | "samp" => { p.font_size = Some(13.0); p.background_color = Some(egui::Color32::from_rgb(38, 38, 48)); }
        "pre" => {
            p.display = Some("block".into()); p.font_size = Some(13.0);
            p.background_color = Some(egui::Color32::from_rgb(28, 28, 36));
            p.padding_top = Some(12.0); p.padding_bottom = Some(12.0);
            p.padding_left = Some(16.0); p.padding_right = Some(16.0);
            p.margin_top = Some(14.0); p.margin_bottom = Some(14.0);
            p.border_radius = Some(6.0);
        }
        "blockquote" => {
            p.display = Some("block".into()); p.margin_left = Some(24.0);
            p.margin_top = Some(14.0); p.margin_bottom = Some(14.0);
            p.padding_left = Some(14.0);
            p.border_color = Some(egui::Color32::from_rgb(90, 70, 160));
            p.border_width = Some(4.0);
            p.color = Some(egui::Color32::from_rgb(180, 180, 190));
        }
        "ul" | "ol" => { p.display = Some("block".into()); p.margin_top = Some(12.0); p.margin_bottom = Some(12.0); p.padding_left = Some(32.0); }
        "li" => { p.display = Some("list-item".into()); p.margin_bottom = Some(4.0); }
        "hr" => { p.display = Some("block".into()); p.margin_top = Some(12.0); p.margin_bottom = Some(12.0); }
        "table" => { p.display = Some("table".into()); p.margin_top = Some(8.0); p.margin_bottom = Some(8.0); }
        "th" => { p.font_weight = Some(700); p.padding_top = Some(6.0); p.padding_bottom = Some(6.0); p.padding_left = Some(8.0); p.padding_right = Some(8.0); }
        "td" => { p.padding_top = Some(6.0); p.padding_bottom = Some(6.0); p.padding_left = Some(8.0); p.padding_right = Some(8.0); }
        "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside" | "form" | "fieldset" | "figure" | "figcaption" => {
            p.display = Some("block".into());
        }
        "span" | "label" | "abbr" | "cite" | "q" => { p.display = Some("inline".into()); }
        "button" | "input" | "select" | "textarea" => { p.display = Some("inline-block".into()); }
        _ => {}
    }
}
 
// ── Seletores ─────────────────────────────────────────────────────────────────
 
fn parse_selectors(input: &str) -> Vec<CssSelector> {
    input.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(parse_one_selector)
        .collect()
}
 
fn parse_one_selector(s: &str) -> Option<CssSelector> {
    let s = s.trim();
    if s == "*" { return Some(CssSelector::Universal); }
    if s.starts_with('#') { return Some(CssSelector::Id(s[1..].to_string())); }
    if s.starts_with('.') { return Some(CssSelector::Class(s[1..].to_string())); }
 
    // Pega só o primeiro componente (ignora descendentes)
    let first = s.split_whitespace().next().unwrap_or(s);
    let first_part = first.split('>').next().unwrap_or(first).split('+').next().unwrap_or(first);
 
    if first_part.contains('#') {
        let parts: Vec<&str> = first_part.splitn(2, '#').collect();
        return Some(CssSelector::TypeId(parts[0].to_lowercase(), parts[1].to_string()));
    }
    if first_part.contains('.') {
        let parts: Vec<&str> = first_part.splitn(2, '.').collect();
        return Some(CssSelector::TypeClass(parts[0].to_lowercase(), parts[1].to_string()));
    }
 
    if first_part.is_empty() { return None; }
    Some(CssSelector::Type(first_part.to_lowercase()))
}
 
fn selector_matches(sel: &CssSelector, tag: &str, classes: &[String], id: Option<&str>) -> bool {
    match sel {
        CssSelector::Universal => true,
        CssSelector::Type(t) => t == tag,
        CssSelector::Class(c) => classes.iter().any(|cl| cl == c),
        CssSelector::Id(i) => id == Some(i.as_str()),
        CssSelector::TypeClass(t, c) => (t.is_empty() || t == tag) && classes.iter().any(|cl| cl == c),
        CssSelector::TypeId(t, i) => (t.is_empty() || t == tag) && id == Some(i.as_str()),
    }
}
 
fn calc_specificity(sel: &CssSelector) -> u32 {
    match sel {
        CssSelector::Universal => 0,
        CssSelector::Type(_) => 1,
        CssSelector::Class(_) | CssSelector::TypeClass(_, _) => 10,
        CssSelector::Id(_) | CssSelector::TypeId(_, _) => 100,
    }
}
 
// ── Declarações ───────────────────────────────────────────────────────────────
 
pub fn parse_declarations(input: &str) -> CssProperties {
    let mut p = CssProperties::default();
    for decl in input.split(';') {
        let decl = decl.trim().trim_end_matches("!important").trim();
        if decl.is_empty() { continue; }
        if let Some(colon) = decl.find(':') {
            let name = decl[..colon].trim().to_lowercase();
            let val = decl[colon + 1..].trim().to_string();
            apply_prop(&mut p, &name, &val);
        }
    }
    p
}
 
fn apply_prop(p: &mut CssProperties, name: &str, val: &str) {
    let val_lo = val.to_lowercase();
    let v = val_lo.as_str();
    match name {
        "color" => p.color = parse_color(v),
        "background-color" => p.background_color = parse_color(v),
        "background" => { if let Some(c) = parse_color(v) { p.background_color = Some(c); } }
        "font-size" => p.font_size = parse_px(v),
        "font-weight" => {
            p.font_weight = match v {
                "bold" | "bolder" => Some(700),
                "normal" | "lighter" => Some(400),
                n => n.parse().ok(),
            };
        }
        "font-style" => p.font_style = Some(v.to_string()),
        "text-align" => p.text_align = Some(v.to_string()),
        "text-decoration" | "text-decoration-line" => p.text_decoration = Some(v.to_string()),
        "display" => p.display = Some(v.to_string()),
        "visibility" => p.visibility = Some(v.to_string()),
        "opacity" => p.opacity = v.parse().ok(),
        "margin" => apply_shorthand4(v, &mut p.margin_top, &mut p.margin_right, &mut p.margin_bottom, &mut p.margin_left),
        "margin-top" => p.margin_top = parse_px(v),
        "margin-bottom" => p.margin_bottom = parse_px(v),
        "margin-left" => p.margin_left = parse_px(v),
        "margin-right" => p.margin_right = parse_px(v),
        "padding" => apply_shorthand4(v, &mut p.padding_top, &mut p.padding_right, &mut p.padding_bottom, &mut p.padding_left),
        "padding-top" => p.padding_top = parse_px(v),
        "padding-bottom" => p.padding_bottom = parse_px(v),
        "padding-left" => p.padding_left = parse_px(v),
        "padding-right" => p.padding_right = parse_px(v),
        "border-radius" => p.border_radius = parse_px(v),
        "border-color" => p.border_color = parse_color(v),
        "border-width" | "border-left-width" => p.border_width = parse_px(v),
        "border" | "border-left" => {
            for part in v.split_whitespace() {
                if let Some(c) = parse_color(part) { p.border_color = Some(c); }
                else if let Some(w) = parse_px(part) { p.border_width = Some(w); }
            }
        }
        "gap" | "grid-gap" => p.gap = parse_px(v),
        _ => {}
    }
}
 
fn apply_shorthand4(v: &str, top: &mut Option<f32>, right: &mut Option<f32>, bottom: &mut Option<f32>, left: &mut Option<f32>) {
    let parts: Vec<f32> = v.split_whitespace().filter_map(parse_px).collect();
    match parts.len() {
        1 => { *top = Some(parts[0]); *right = Some(parts[0]); *bottom = Some(parts[0]); *left = Some(parts[0]); }
        2 => { *top = Some(parts[0]); *bottom = Some(parts[0]); *right = Some(parts[1]); *left = Some(parts[1]); }
        3 => { *top = Some(parts[0]); *right = Some(parts[1]); *left = Some(parts[1]); *bottom = Some(parts[2]); }
        4 => { *top = Some(parts[0]); *right = Some(parts[1]); *bottom = Some(parts[2]); *left = Some(parts[3]); }
        _ => {}
    }
}
 
fn merge(target: &mut CssProperties, src: &CssProperties) {
    if let Some(v) = src.color { target.color = Some(v); }
    if let Some(v) = src.background_color { target.background_color = Some(v); }
    if let Some(v) = src.font_size { target.font_size = Some(v); }
    if let Some(v) = src.font_weight { target.font_weight = Some(v); }
    if let Some(ref v) = src.font_style { target.font_style = Some(v.clone()); }
    if let Some(ref v) = src.text_align { target.text_align = Some(v.clone()); }
    if let Some(ref v) = src.text_decoration { target.text_decoration = Some(v.clone()); }
    if let Some(ref v) = src.display { target.display = Some(v.clone()); }
    if let Some(ref v) = src.visibility { target.visibility = Some(v.clone()); }
    if let Some(v) = src.opacity { target.opacity = Some(v); }
    if let Some(v) = src.margin_top { target.margin_top = Some(v); }
    if let Some(v) = src.margin_bottom { target.margin_bottom = Some(v); }
    if let Some(v) = src.margin_left { target.margin_left = Some(v); }
    if let Some(v) = src.margin_right { target.margin_right = Some(v); }
    if let Some(v) = src.padding_top { target.padding_top = Some(v); }
    if let Some(v) = src.padding_bottom { target.padding_bottom = Some(v); }
    if let Some(v) = src.padding_left { target.padding_left = Some(v); }
    if let Some(v) = src.padding_right { target.padding_right = Some(v); }
    if let Some(v) = src.border_radius { target.border_radius = Some(v); }
    if let Some(v) = src.border_color { target.border_color = Some(v); }
    if let Some(v) = src.border_width { target.border_width = Some(v); }
    if let Some(v) = src.gap { target.gap = Some(v); }
}
 
// ── Cores ─────────────────────────────────────────────────────────────────────
 
pub fn parse_color(s: &str) -> Option<egui::Color32> {
    let s = s.trim();
    if s == "transparent" { return Some(egui::Color32::TRANSPARENT); }
    if s.starts_with('#') { return parse_hex(&s[1..]); }
    if s.starts_with("rgba(") || s.starts_with("rgb(") { return parse_rgb(s); }
    if s.starts_with("hsla(") || s.starts_with("hsl(") { return parse_hsl(s); }
    named_color(s)
}
 
fn parse_hex(h: &str) -> Option<egui::Color32> {
    match h.len() {
        3 => Some(egui::Color32::from_rgb(
            u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?,
        )),
        6 => Some(egui::Color32::from_rgb(
            u8::from_str_radix(&h[0..2], 16).ok()?,
            u8::from_str_radix(&h[2..4], 16).ok()?,
            u8::from_str_radix(&h[4..6], 16).ok()?,
        )),
        8 => Some(egui::Color32::from_rgba_unmultiplied(
            u8::from_str_radix(&h[0..2], 16).ok()?,
            u8::from_str_radix(&h[2..4], 16).ok()?,
            u8::from_str_radix(&h[4..6], 16).ok()?,
            u8::from_str_radix(&h[6..8], 16).ok()?,
        )),
        _ => None,
    }
}
 
fn parse_rgb(s: &str) -> Option<egui::Color32> {
    let inner = s.trim_start_matches("rgba(").trim_start_matches("rgb(").trim_end_matches(')');
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 { return None; }
    let r = chan(parts[0].trim())?;
    let g = chan(parts[1].trim())?;
    let b = chan(parts[2].trim())?;
    let a = if parts.len() >= 4 {
        let af: f32 = parts[3].trim().parse().ok()?;
        (af * 255.0) as u8
    } else { 255 };
    Some(egui::Color32::from_rgba_unmultiplied(r, g, b, a))
}
 
fn chan(s: &str) -> Option<u8> {
    if s.ends_with('%') {
        let pct: f32 = s[..s.len() - 1].parse().ok()?;
        Some((pct / 100.0 * 255.0).clamp(0.0, 255.0) as u8)
    } else {
        let v: f32 = s.parse().ok()?;
        Some(v.clamp(0.0, 255.0) as u8)
    }
}
 
fn parse_hsl(s: &str) -> Option<egui::Color32> {
    let inner = s.trim_start_matches("hsla(").trim_start_matches("hsl(").trim_end_matches(')');
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 { return None; }
    let h: f32 = parts[0].trim().parse().ok()?;
    let sl: f32 = parts[1].trim().trim_end_matches('%').parse::<f32>().ok()? / 100.0;
    let l: f32 = parts[2].trim().trim_end_matches('%').parse::<f32>().ok()? / 100.0;
    let a: f32 = if parts.len() >= 4 { parts[3].trim().parse().unwrap_or(1.0) } else { 1.0 };
    let (r, g, b) = hsl_to_rgb(h / 360.0, sl, l);
    Some(egui::Color32::from_rgba_unmultiplied(r, g, b, (a * 255.0) as u8))
}
 
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s == 0.0 { let v = (l * 255.0) as u8; return (v, v, v); }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let r = hue2rgb(p, q, h + 1.0 / 3.0);
    let g = hue2rgb(p, q, h);
    let b = hue2rgb(p, q, h - 1.0 / 3.0);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
 
fn hue2rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 0.5 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}
 
pub fn parse_px(v: &str) -> Option<f32> {
    let v = v.trim();
    if v == "0" { return Some(0.0); }
    if v == "auto" || v == "none" { return None; }
    if v.ends_with("px") { return v[..v.len() - 2].trim().parse().ok(); }
    if v.ends_with("em") { return v[..v.len() - 2].trim().parse::<f32>().ok().map(|x| x * 16.0); }
    if v.ends_with("rem") { return v[..v.len() - 3].trim().parse::<f32>().ok().map(|x| x * 16.0); }
    if v.ends_with("pt") { return v[..v.len() - 2].trim().parse::<f32>().ok().map(|x| x * 1.333); }
    v.parse().ok()
}
 
// ── Helpers internos ──────────────────────────────────────────────────────────
 
fn remove_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let b = css.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') { i += 1; }
            i += 2;
        } else {
            if i < b.len() { out.push(b[i] as char); }
            i += 1;
        }
    }
    out
}
 
fn skip_at_rule(css: &str) -> Option<usize> {
    let b = css.as_bytes();
    let mut depth = 0usize;
    for (i, &c) in b.iter().enumerate() {
        if c == b'{' { depth += 1; }
        else if c == b'}' {
            if depth > 0 { depth -= 1; if depth == 0 { return Some(i + 1); } }
        } else if c == b';' && depth == 0 { return Some(i + 1); }
    }
    None
}
 
fn named_color(name: &str) -> Option<egui::Color32> {
    let (r, g, b) = match name {
        "black" => (0, 0, 0), "white" => (255, 255, 255), "red" => (255, 0, 0),
        "green" => (0, 128, 0), "blue" => (0, 0, 255), "yellow" => (255, 255, 0),
        "orange" => (255, 165, 0), "purple" => (128, 0, 128), "pink" => (255, 192, 203),
        "gray" | "grey" => (128, 128, 128), "lightgray" | "lightgrey" => (211, 211, 211),
        "darkgray" | "darkgrey" => (64, 64, 64), "navy" => (0, 0, 128), "teal" => (0, 128, 128),
        "silver" => (192, 192, 192), "maroon" => (128, 0, 0), "olive" => (128, 128, 0),
        "lime" => (0, 255, 0), "aqua" | "cyan" => (0, 255, 255), "fuchsia" | "magenta" => (255, 0, 255),
        "coral" => (255, 127, 80), "salmon" => (250, 128, 114), "crimson" => (220, 20, 60),
        "gold" => (255, 215, 0), "violet" => (238, 130, 238), "indigo" => (75, 0, 130),
        "turquoise" => (64, 224, 208), "chocolate" => (210, 105, 30), "tomato" => (255, 99, 71),
        "steelblue" => (70, 130, 180), "royalblue" => (65, 105, 225), "dodgerblue" => (30, 144, 255),
        "skyblue" => (135, 206, 235), "deepskyblue" => (0, 191, 255), "hotpink" => (255, 105, 180),
        "deeppink" => (255, 20, 147), "plum" => (221, 160, 221), "orchid" => (218, 112, 214),
        "darkblue" => (0, 0, 139), "darkred" => (139, 0, 0), "darkgreen" => (0, 100, 0),
        "darkorange" => (255, 140, 0), "darkviolet" => (148, 0, 211), "dimgray" => (105, 105, 105),
        "slateblue" => (106, 90, 205), "mediumblue" => (0, 0, 205), "limegreen" => (50, 205, 50),
        "seagreen" => (46, 139, 87), "springgreen" => (0, 255, 127), "beige" => (245, 245, 220),
        "ivory" => (255, 255, 240), "lavender" => (230, 230, 250), "wheat" => (245, 222, 179),
        "tan" => (210, 180, 140), "khaki" => (240, 230, 140), "peru" => (205, 133, 63),
        "firebrick" => (178, 34, 34), "sienna" => (160, 82, 45), "brown" => (165, 42, 42),
        "rebeccapurple" => (102, 51, 153), _ => return None,
    };
    Some(egui::Color32::from_rgb(r, g, b))
}