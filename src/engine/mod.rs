use scraper::{Html, Selector};
use reqwest::blocking::Client;
use std::time::Duration;
use url::Url;

#[derive(Clone, Debug)]
pub enum ElementoWeb {
    Titulo(String),
    Texto(String),
    Link(String, String), // Texto, URL
}

pub fn baixar_html_bruto(url: &str) -> String {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap_or_else(|_| Client::new());

    match client.get(url).send() {
        Ok(res) => {
            if res.status().is_success() {
                res.text().unwrap_or_else(|_| "Erro ao ler o corpo da página.".to_string())
            } else {
                format!("Erro HTTP {}: {}", res.status(), url)
            }
        }
        Err(e) => format!("Erro de conexão: {}. Verifique se a URL está correta.", e),
    }
}

pub fn processar_html_semantico(html: &str, base_url: &str) -> Vec<ElementoWeb> {
    let document = Html::parse_document(html);
    let mut elementos = Vec::new();

    // Tentar extrair o título da página
    let selector_title = Selector::parse("title").unwrap();
    if let Some(title_node) = document.select(&selector_title).next() {
        let title_text = title_node.text().collect::<Vec<_>>().join("");
        if !title_text.trim().is_empty() {
            elementos.push(ElementoWeb::Titulo(title_text));
        }
    }

    let seletor_h = Selector::parse("h1, h2, h3").unwrap();
    let seletor_p = Selector::parse("p").unwrap();
    let seletor_a = Selector::parse("a").unwrap();

    for h in document.select(&seletor_h) {
        let texto = h.text().collect::<Vec<_>>().join(" ");
        if !texto.trim().is_empty() {
            elementos.push(ElementoWeb::Titulo(texto));
        }
    }

    for a in document.select(&seletor_a) {
        let texto = a.text().collect::<Vec<_>>().join(" ");
        let href = a.value().attr("href").unwrap_or("").to_string();
        
        if !texto.trim().is_empty() && !href.is_empty() {
            // Resolver URLs relativas
            let absolute_url = if href.starts_with("http") {
                href
            } else if href.starts_with('/') {
                if let Ok(base) = Url::parse(base_url) {
                    format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or(""), href)
                } else {
                    href
                }
            } else {
                href
            };
            elementos.push(ElementoWeb::Link(texto, absolute_url));
        }
    }

    for p in document.select(&seletor_p) {
        let texto = p.text().collect::<Vec<_>>().join(" ");
        if !texto.trim().is_empty() {
            elementos.push(ElementoWeb::Texto(texto));
        }
    }

    if elementos.is_empty() {
        elementos.push(ElementoWeb::Texto("Conteúdo não encontrado ou site protegido.".to_string()));
    }

    elementos
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
        // Busca via DuckDuckGo (usando a versão HTML sem JS)
        format!("https://duckduckgo.com/html/?q={}", input.replace(' ', "+"))
    }
}
