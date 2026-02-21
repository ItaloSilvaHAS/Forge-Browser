use scraper::{Html, Selector};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Clone)]
pub enum ElementoWeb {
    Titulo(String),
    Texto(String),
}

pub fn baixar_html_bruto(host: &str) -> String {
    let clean_host = host.trim_start_matches("http://").trim_start_matches("https://");
    let host_only = clean_host.split('/').next().unwrap_or(clean_host);
    
    // Tentativa de ToSocketAddrs com fallback manual para IPs comuns se falhar
    let addr = format!("{}:80", host_only);
    let socket_addr = match addr.parse::<std::net::SocketAddr>() {
        Ok(s) => Some(s),
        Err(_) => {
            use std::net::ToSocketAddrs;
            match addr.to_socket_addrs() {
                Ok(mut iter) => iter.next(),
                Err(_) => None,
            }
        }
    };

    let socket_addr = match socket_addr {
        Some(a) => a,
        None => return format!("Erro: Não foi possível resolver o host '{}'. Verifique sua conexão ou o nome do domínio.", host_only),
    };

    match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5)) {
        Ok(mut stream) => {
            let request = format!(
                "GET / HTTP/1.1\r\nHost: {}\r\nUser-Agent: ForgeBrowser/1.0\r\nAccept: text/html\r\nConnection: close\r\n\r\n",
                host_only
            );
            if let Err(e) = stream.write_all(request.as_bytes()) {
                return format!("Erro ao enviar requisição: {}", e);
            }
            
            let mut buffer = Vec::new();
            let _ = stream.set_read_timeout(Some(Duration::from_secs(7)));
            
            match stream.read_to_end(&mut buffer) {
                Ok(_) => {
                    let response = String::from_utf8_lossy(&buffer);
                    // Verifica se é um redirecionamento (comum em sites modernos para HTTPS)
                    if response.contains("HTTP/1.1 301") || response.contains("HTTP/1.1 302") {
                        return "Este site redirecionou para HTTPS. O Forge-Browser (Flux Engine) atualmente suporta apenas conexões HTTP puras na porta 80 para garantir o minimalismo e performance máxima.".to_string();
                    }
                    response.to_string()
                },
                Err(e) => format!("Erro ao ler resposta (Timeout ou Interrupção): {}. Note: Sites pesados ou com redirecionamentos podem falhar.", e),
            }
        }
        Err(e) => format!("Erro de Conexão com {}: {}. (Lembre-se: suporte apenas porta 80/HTTP)", host_only, e),
    }
}

pub fn processar_html_semantico(html: &str) -> Vec<ElementoWeb> {
    let document = Html::parse_document(html);
    let mut elementos = Vec::new();

    let seletor_h = Selector::parse("h1, h2, h3").unwrap();
    let seletor_p = Selector::parse("p").unwrap();

    for h in document.select(&seletor_h) {
        let texto = h.text().collect::<Vec<_>>().join(" ");
        if !texto.trim().is_empty() {
            elementos.push(ElementoWeb::Titulo(texto));
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
    if (input.contains('.') && !input.contains(' ')) || input.starts_with("localhost") {
        input.to_string()
    } else {
        // Integração simples com DuckDuckGo (via HTTP 80 para manter compatibilidade)
        // Como o motor é minimalista, apenas simulamos a query ou apontamos para um host de busca
        format!("duckduckgo.com/?q={}", input.replace(' ', "+"))
    }
}
