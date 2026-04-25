use clap::{Parser, Subcommand};
use inquire::{Select, Text, validator::Validation};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "dandelion")]
#[command(about = "Dandelion - Gerenciador de musicas", long_about = None)]
#[command(version = "0.1.0")]
#[command(author = "CaioSimioni")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lista arquivos de musica na pasta Desconhecido
    Listar {
        /// Caminho da pasta (padrao: /mnt/c/Users/<user>/Music/Desconhecido)
        #[arg(short, long)]
        pasta: Option<String>,
    },

    /// Organiza musicas da pasta Desconhecido para uma pasta de artista/album
    Organizar {
        /// Nome do artista
        #[arg(short, long)]
        artista: Option<String>,

        /// Nome do album
        #[arg(short = 'l', long)]
        album: Option<String>,

        /// Caminho da pasta de musicas (padrao: /mnt/c/Users/<user>/Music)
        #[arg(short, long)]
        pasta: Option<String>,

        /// Modo dry-run (mostra o que seria feito sem executar)
        #[arg(short, long)]
        dry_run: bool,

        /// Mover arquivos em vez de copiar
        #[arg(short, long)]
        mover: bool,

        /// Modo nao-interativo (usa ordem alfabetica)
        #[arg(short, long)]
        no_interactive: bool,
    },

    /// Renomeia arquivos de musica em uma pasta especifica
    Renomear {
        /// Caminho da pasta com as musicas
        #[arg(short, long)]
        pasta: String,

        /// Nome do artista
        #[arg(short, long)]
        artista: String,

        /// Modo dry-run (mostra o que seria feito sem executar)
        #[arg(short, long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Listar { pasta } => {
            let caminho = pasta
                .clone()
                .unwrap_or_else(|| format!("{}/Desconhecido", obter_pasta_musicas()));
            listar_musicas(&caminho);
        }
        Commands::Organizar {
            artista,
            album,
            pasta,
            dry_run,
            mover,
            no_interactive,
        } => {
            let base_path = pasta.clone().unwrap_or_else(obter_pasta_musicas);
            
            // Perguntar artista e album se não foram fornecidos
            let artista = if let Some(a) = artista {
                a.clone()
            } else {
                match Text::new("Nome do artista:")
                    .with_validator(|input: &str| {
                        if input.trim().is_empty() {
                            Ok(Validation::Invalid("Nome do artista nao pode ser vazio".into()))
                        } else {
                            Ok(Validation::Valid)
                        }
                    })
                    .prompt() {
                    Ok(input) => input,
                    Err(_) => {
                        eprintln!("[ERRO] Operacao cancelada");
                        return;
                    }
                }
            };

            let album = if let Some(a) = album {
                a.clone()
            } else {
                match Text::new("Nome do album:")
                    .with_validator(|input: &str| {
                        if input.trim().is_empty() {
                            Ok(Validation::Invalid("Nome do album nao pode ser vazio".into()))
                        } else {
                            Ok(Validation::Valid)
                        }
                    })
                    .prompt() {
                    Ok(input) => input,
                    Err(_) => {
                        eprintln!("[ERRO] Operacao cancelada");
                        return;
                    }
                }
            };

            organizar_album(&base_path, &artista, &album, *dry_run, *mover, *no_interactive);
        }
        Commands::Renomear {
            pasta,
            artista,
            dry_run,
        } => {
            renomear_musicas(pasta, artista, *dry_run);
        }
    }
}

fn obter_pasta_musicas() -> String {
    // Detectar usuario do Windows via WSL
    if let Ok(user) = std::env::var("USER") {
        let caminho = format!("/mnt/c/Users/{}/Music", user);
        if Path::new(&caminho).exists() {
            return caminho;
        }
    }

    // Fallback: tentar detectar pelo diretorio home do WSL
    if let Ok(home) = std::env::var("HOME") {
        let username = home.split('/').last().unwrap_or("user");
        let caminho = format!("/mnt/c/Users/{}/Music", username);
        if Path::new(&caminho).exists() {
            return caminho;
        }
    }

    // Ultimo fallback
    "/mnt/c/Users/Public/Music".to_string()
}

fn listar_musicas(pasta: &str) {
    println!("Dandelion - Listando musicas");
    println!("-------------------------------------");
    println!("Pasta: {}\n", pasta);

    if !Path::new(pasta).exists() {
        eprintln!("[ERRO] Pasta nao encontrada: {}", pasta);
        return;
    }

    let mut arquivos: Vec<PathBuf> = WalkDir::new(pasta)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && eh_arquivo_musica(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    arquivos.sort();

    if arquivos.is_empty() {
        println!("[AVISO] Nenhum arquivo de musica encontrado.");
        return;
    }

    println!("Arquivos encontrados:\n");

    for (idx, arquivo) in arquivos.iter().enumerate() {
        let nome = arquivo.file_name().unwrap().to_str().unwrap();
        let extensao = arquivo
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .to_uppercase();

        // Obter tamanho do arquivo
        let tamanho = match arquivo.metadata() {
            Ok(meta) => {
                let bytes = meta.len();
                if bytes < 1024 * 1024 {
                    format!("{:.1} KB", bytes as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                }
            }
            Err(_) => "?".to_string(),
        };

        println!("  {:<3}. [{}] {} ({})", idx + 1, extensao, nome, tamanho);
    }

    println!("\nTotal: {} arquivo(s)", arquivos.len());
    println!("\nPara organizar, use:");
    println!("  dandelion organizar");
}

fn organizar_album(base_path: &str, artista: &str, album: &str, dry_run: bool, mover: bool, no_interactive: bool) {
    let origem = format!("{}/Desconhecido", base_path);
    let destino = format!("{}/{} - {}", base_path, artista, album);

    println!("\nDandelion - Organizando album");
    println!("-------------------------------------");
    println!("Album: {} - {}", artista, album);
    println!("Origem: {}", origem);
    println!("Destino: {}", destino);
    println!("Acao: {}", if mover { "Mover" } else { "Copiar" });
    println!();

    if !Path::new(&origem).exists() {
        eprintln!("[ERRO] Pasta de origem nao encontrada: {}", origem);
        return;
    }

    // Coletar arquivos de musica
    let mut arquivos: Vec<PathBuf> = WalkDir::new(&origem)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && eh_arquivo_musica(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    arquivos.sort();

    if arquivos.is_empty() {
        println!("[AVISO] Nenhum arquivo de musica encontrado em {}", origem);
        return;
    }

    // Modo interativo: permitir reordenação
    let arquivos_ordenados = if !no_interactive {
        reordenar_arquivos_interativo(arquivos)
    } else {
        arquivos
    };

    // Criar pasta de destino
    if !dry_run {
        if let Err(e) = fs::create_dir_all(&destino) {
            eprintln!("[ERRO] Erro ao criar pasta de destino: {}", e);
            return;
        }
        println!("[OK] Pasta criada: {}", destino);
    } else {
        println!("[DRY-RUN] Criaria pasta: {}", destino);
    }

    println!("\nProcessando arquivos...\n");

    // Processar cada arquivo
    for (idx, arquivo_origem) in arquivos_ordenados.iter().enumerate() {
        let nome_original = arquivo_origem.file_name().unwrap().to_str().unwrap();
        let extensao = arquivo_origem.extension().unwrap().to_str().unwrap();

        // Extrair nome da musica (remover extensao)
        let nome_musica = nome_original
            .trim_end_matches(&format!(".{}", extensao))
            .trim();

        // Criar novo nome no formato: 01 - Artista - Nome da Musica.mp3
        let numero = format!("{:02}", idx + 1);
        let novo_nome = format!("{} - {} - {}.{}", numero, artista, nome_musica, extensao);
        let arquivo_destino = PathBuf::from(&destino).join(&novo_nome);

        if dry_run {
            println!("  [DRY-RUN] {} -> {}", nome_original, novo_nome);
        } else {
            let resultado = if mover {
                fs::rename(arquivo_origem, &arquivo_destino)
            } else {
                fs::copy(arquivo_origem, &arquivo_destino).map(|_| ())
            };

            match resultado {
                Ok(_) => println!("  [OK] {}", novo_nome),
                Err(e) => eprintln!("  [ERRO] Falha ao processar {}: {}", nome_original, e),
            }
        }
    }

    println!("\nProcesso concluido!");
    println!("Total: {} arquivos processados", arquivos_ordenados.len());
}

fn reordenar_arquivos_interativo(mut arquivos: Vec<PathBuf>) -> Vec<PathBuf> {
    println!("Modo interativo - Reordene as musicas");
    println!("-------------------------------------\n");

    let mut arquivos_finais: Vec<PathBuf> = Vec::new();
    let mut disponiveis = arquivos.clone();

    while !disponiveis.is_empty() {
        let posicao_atual = arquivos_finais.len() + 1;
        
        // Criar opções para o menu
        let opcoes: Vec<String> = disponiveis
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();

        println!("\nEscolha a musica #{} (ou Ctrl+C para cancelar):", posicao_atual);
        
        let escolha = Select::new("", opcoes.clone())
            .with_page_size(15)
            .prompt();

        match escolha {
            Ok(selecionado) => {
                // Encontrar o índice do arquivo selecionado
                if let Some(idx) = opcoes.iter().position(|x| x == &selecionado) {
                    let arquivo = disponiveis.remove(idx);
                    arquivos_finais.push(arquivo);
                    println!("[OK] Adicionado: {}", selecionado);
                }
            }
            Err(_) => {
                eprintln!("\n[ERRO] Operacao cancelada pelo usuario");
                std::process::exit(0);
            }
        }
    }

    println!("\n-------------------------------------");
    println!("Ordem final das musicas:\n");
    
    for (idx, arquivo) in arquivos_finais.iter().enumerate() {
        let nome = arquivo.file_name().unwrap().to_str().unwrap();
        println!("  {:02}. {}", idx + 1, nome);
    }
    
    println!("\n-------------------------------------\n");

    arquivos_finais
}

fn renomear_musicas(pasta: &str, artista: &str, dry_run: bool) {
    println!("Dandelion - Renomeando musicas");
    println!("-------------------------------------");
    println!("Pasta: {}", pasta);
    println!("Artista: {}", artista);
    println!();

    if !Path::new(pasta).exists() {
        eprintln!("[ERRO] Pasta nao encontrada: {}", pasta);
        return;
    }

    // Coletar e ordenar arquivos de musica
    let mut arquivos: Vec<PathBuf> = WalkDir::new(pasta)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && eh_arquivo_musica(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    arquivos.sort();

    if arquivos.is_empty() {
        println!("[AVISO] Nenhum arquivo de musica encontrado na pasta.");
        return;
    }

    let regex_numero = Regex::new(r"^(\d+)").unwrap();

    for (idx, arquivo) in arquivos.iter().enumerate() {
        let nome_original = arquivo.file_name().unwrap().to_str().unwrap();
        let extensao = arquivo.extension().unwrap().to_str().unwrap();

        // Tentar extrair numero do nome original
        let numero = if let Some(caps) = regex_numero.captures(nome_original) {
            caps.get(1).unwrap().as_str().to_string()
        } else {
            format!("{:02}", idx + 1)
        };

        // Extrair nome da musica (remover numero e extensao)
        let nome_musica = nome_original
            .trim_start_matches(|c: char| {
                c.is_numeric() || c == '-' || c == '.' || c.is_whitespace()
            })
            .trim_end_matches(&format!(".{}", extensao))
            .trim();

        let novo_nome = format!("{} - {} - {}.{}", numero, artista, nome_musica, extensao);
        let novo_caminho = arquivo.parent().unwrap().join(&novo_nome);

        if dry_run {
            println!("  [DRY-RUN] {} -> {}", nome_original, novo_nome);
        } else {
            match fs::rename(arquivo, &novo_caminho) {
                Ok(_) => println!("  [OK] {}", novo_nome),
                Err(e) => eprintln!("  [ERRO] Erro ao renomear {}: {}", nome_original, e),
            }
        }
    }

    println!("\nProcesso concluido! Total: {} arquivos", arquivos.len());
}

fn eh_arquivo_musica(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().unwrap().to_lowercase();
        matches!(
            ext.as_str(),
            "mp3" | "flac" | "m4a" | "wav" | "ogg" | "opus" | "aac" | "wma"
        )
    } else {
        false
    }
}
