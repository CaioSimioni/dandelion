use clap::{Parser, Subcommand};
use inquire::{Select, Text, validator::Validation, Confirm};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "dandelion")]
#[command(about = "Dandelion - Gerenciador de musicas", long_about = None)]
#[command(version = "0.2.0")]
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

        /// Nao editar nomes das musicas
        #[arg(short, long)]
        no_edit: bool,
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

    /// Remove os arquivos de musica da pasta Desconhecido apos organizar
    Limpar {
        /// Caminho da pasta (padrao: /mnt/c/Users/<user>/Music/Desconhecido)
        #[arg(short, long)]
        pasta: Option<String>,

        /// Modo dry-run (mostra o que seria removido sem executar)
        #[arg(short, long)]
        dry_run: bool,
    },
}

#[derive(Clone)]
struct MusicaInfo {
    arquivo_original: PathBuf,
    nome_editado: String,
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
            no_edit,
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

            organizar_album(&base_path, &artista, &album, *dry_run, *mover, *no_interactive, *no_edit);
        }
        Commands::Renomear {
            pasta,
            artista,
            dry_run,
        } => {
            renomear_musicas(pasta, artista, *dry_run);
        }
        Commands::Limpar { pasta, dry_run } => {
            let caminho = pasta
                .clone()
                .unwrap_or_else(|| format!("{}/Desconhecido", obter_pasta_musicas()));
            limpar_desconhecido(&caminho, *dry_run);
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
        let username = home.split('/').next_back().unwrap_or("user");
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

fn organizar_album(base_path: &str, artista: &str, album: &str, dry_run: bool, mover: bool, no_interactive: bool, no_edit: bool) {
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

    // Modo interativo: permitir reordenação e edição
    let musicas_ordenadas = if !no_interactive {
        reordenar_e_editar_arquivos(arquivos, !no_edit)
    } else {
        arquivos.into_iter().map(|a| {
            let nome = a.file_stem().unwrap().to_str().unwrap().to_string();
            MusicaInfo {
                arquivo_original: a,
                nome_editado: nome,
            }
        }).collect()
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
    for (idx, musica) in musicas_ordenadas.iter().enumerate() {
        let nome_original = musica.arquivo_original.file_name().unwrap().to_str().unwrap();
        let extensao = musica.arquivo_original.extension().unwrap().to_str().unwrap();

        // Criar novo nome no formato: 01 - Artista - Nome da Musica.mp3
        let numero = format!("{:02}", idx + 1);
        let novo_nome = format!("{} - {} - {}.{}", numero, artista, musica.nome_editado, extensao);
        let arquivo_destino = PathBuf::from(&destino).join(&novo_nome);

        if dry_run {
            println!("  [DRY-RUN] {} -> {}", nome_original, novo_nome);
        } else {
            let resultado = if mover {
                fs::rename(&musica.arquivo_original, &arquivo_destino)
            } else {
                fs::copy(&musica.arquivo_original, &arquivo_destino).map(|_| ())
            };

            match resultado {
                Ok(_) => println!("  [OK] {}", novo_nome),
                Err(e) => eprintln!("  [ERRO] Falha ao processar {}: {}", nome_original, e),
            }
        }
    }

    println!("\nProcesso concluido!");
    println!("Total: {} arquivos processados", musicas_ordenadas.len());
}

fn reordenar_e_editar_arquivos(arquivos: Vec<PathBuf>, permitir_edicao: bool) -> Vec<MusicaInfo> {
    println!("Modo interativo - Reordene as musicas");
    println!("-------------------------------------\n");

    let mut musicas_finais: Vec<MusicaInfo> = Vec::new();
    let mut disponiveis: Vec<MusicaInfo> = arquivos
        .into_iter()
        .map(|a| {
            let nome = a.file_stem().unwrap().to_str().unwrap().to_string();
            MusicaInfo {
                arquivo_original: a,
                nome_editado: nome,
            }
        })
        .collect();

    while !disponiveis.is_empty() {
        let posicao_atual = musicas_finais.len() + 1;
        
        // Criar opções para o menu
        let opcoes: Vec<String> = disponiveis
            .iter()
            .map(|m| m.nome_editado.clone())
            .collect();

        println!("\nEscolha a musica #{} (ou Ctrl+C para cancelar):", posicao_atual);
        
        let escolha = Select::new("", opcoes.clone())
            .with_page_size(15)
            .prompt();

        match escolha {
            Ok(selecionado) => {
                // Encontrar o índice do arquivo selecionado
                if let Some(idx) = opcoes.iter().position(|x| x == &selecionado) {
                    let mut musica = disponiveis.remove(idx);
                    
                    // Perguntar se quer editar o nome
                    if permitir_edicao
                        && let Ok(true) = Confirm::new(&format!("Editar nome da musica '{}'?", musica.nome_editado))
                            .with_default(false)
                            .prompt()
                        && let Ok(novo_nome) = Text::new("Novo nome:")
                            .with_default(&musica.nome_editado)
                            .prompt()
                        && !novo_nome.trim().is_empty()
                    {
                        musica.nome_editado = novo_nome.trim().to_string();
                        println!("[OK] Nome atualizado para: {}", musica.nome_editado);
                    }
                    
                    println!("[OK] Adicionado: {}", musica.nome_editado);
                    musicas_finais.push(musica);
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
    
    for (idx, musica) in musicas_finais.iter().enumerate() {
        println!("  {:02}. {}", idx + 1, musica.nome_editado);
    }
    
    println!("\n-------------------------------------\n");

    musicas_finais
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

fn limpar_desconhecido(pasta: &str, dry_run: bool) {
    println!("Dandelion - Limpando pasta Desconhecido");
    println!("-------------------------------------");
    println!("Pasta: {}", pasta);
    println!();

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
        println!("[AVISO] Nenhum arquivo de musica encontrado em {}", pasta);
        return;
    }

    println!("Arquivos a remover:");
    for arquivo in &arquivos {
        let nome = arquivo.file_name().unwrap().to_str().unwrap();
        println!("  - {}", nome);
    }
    println!();

    if dry_run {
        println!("[DRY-RUN] {} arquivo(s) seriam removidos.", arquivos.len());
        return;
    }

    match Confirm::new(&format!("Remover {} arquivo(s) da pasta Desconhecido?", arquivos.len()))
        .with_default(false)
        .prompt()
    {
        Ok(true) => {}
        _ => {
            println!("[CANCELADO] Nenhum arquivo foi removido.");
            return;
        }
    }

    let mut removidos = 0;
    for arquivo in &arquivos {
        let nome = arquivo.file_name().unwrap().to_str().unwrap();
        match fs::remove_file(arquivo) {
            Ok(_) => {
                println!("  [OK] Removido: {}", nome);
                removidos += 1;
            }
            Err(e) => eprintln!("  [ERRO] Falha ao remover {}: {}", nome, e),
        }
    }

    println!("\nProcesso concluido! {} arquivo(s) removidos.", removidos);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    // ─── eh_arquivo_musica ────────────────────────────────────────────────────

    #[test]
    fn test_musica_mp3() {
        assert!(eh_arquivo_musica(Path::new("song.mp3")));
    }

    #[test]
    fn test_musica_flac() {
        assert!(eh_arquivo_musica(Path::new("song.flac")));
    }

    #[test]
    fn test_musica_m4a() {
        assert!(eh_arquivo_musica(Path::new("song.m4a")));
    }

    #[test]
    fn test_musica_wav() {
        assert!(eh_arquivo_musica(Path::new("song.wav")));
    }

    #[test]
    fn test_musica_ogg() {
        assert!(eh_arquivo_musica(Path::new("song.ogg")));
    }

    #[test]
    fn test_musica_opus() {
        assert!(eh_arquivo_musica(Path::new("song.opus")));
    }

    #[test]
    fn test_musica_aac() {
        assert!(eh_arquivo_musica(Path::new("song.aac")));
    }

    #[test]
    fn test_musica_wma() {
        assert!(eh_arquivo_musica(Path::new("song.wma")));
    }

    #[test]
    fn test_extensao_maiuscula_valida() {
        assert!(eh_arquivo_musica(Path::new("song.MP3")));
        assert!(eh_arquivo_musica(Path::new("song.FLAC")));
    }

    #[test]
    fn test_nao_musica_txt() {
        assert!(!eh_arquivo_musica(Path::new("arquivo.txt")));
    }

    #[test]
    fn test_nao_musica_jpg() {
        assert!(!eh_arquivo_musica(Path::new("imagem.jpg")));
    }

    #[test]
    fn test_nao_musica_pdf() {
        assert!(!eh_arquivo_musica(Path::new("doc.pdf")));
    }

    #[test]
    fn test_sem_extensao() {
        assert!(!eh_arquivo_musica(Path::new("arquivo_sem_extensao")));
    }

    // ─── listar_musicas ───────────────────────────────────────────────────────

    #[test]
    fn test_listar_pasta_inexistente_nao_panics() {
        // Não deve entrar em panic; apenas imprime erro
        listar_musicas("/caminho/que/nao/existe");
    }

    #[test]
    fn test_listar_pasta_vazia() {
        let dir = tempdir();
        listar_musicas(dir.path().to_str().unwrap());
        cleanup_dir(&dir);
    }

    #[test]
    fn test_listar_com_arquivos_musica() {
        let dir = tempdir();
        criar_arquivo(&dir, "musica1.mp3");
        criar_arquivo(&dir, "musica2.flac");
        criar_arquivo(&dir, "imagem.jpg"); // deve ser ignorado
        listar_musicas(dir.path().to_str().unwrap());
        cleanup_dir(&dir);
    }

    // ─── limpar_desconhecido ──────────────────────────────────────────────────

    #[test]
    fn test_limpar_pasta_inexistente_nao_panics() {
        limpar_desconhecido("/caminho/que/nao/existe", false);
    }

    #[test]
    fn test_limpar_dry_run_nao_remove_arquivos() {
        let dir = tempdir();
        criar_arquivo(&dir, "musica1.mp3");
        criar_arquivo(&dir, "musica2.flac");

        limpar_desconhecido(dir.path().to_str().unwrap(), true);

        // Em dry-run os arquivos devem continuar existindo
        assert!(dir.path().join("musica1.mp3").exists());
        assert!(dir.path().join("musica2.flac").exists());
        cleanup_dir(&dir);
    }

    // ─── organizar_album ──────────────────────────────────────────────────────

    #[test]
    fn test_organizar_copia_arquivos() {
        let base = tempdir();
        let desconhecido = base.path().join("Desconhecido");
        fs::create_dir_all(&desconhecido).unwrap();
        criar_arquivo_em(&desconhecido, "faixa1.mp3");
        criar_arquivo_em(&desconhecido, "faixa2.mp3");

        organizar_album(
            base.path().to_str().unwrap(),
            "Artista Teste",
            "Album Teste",
            false, // dry_run
            false, // mover
            true,  // no_interactive
            true,  // no_edit
        );

        let destino = base.path().join("Artista Teste - Album Teste");
        assert!(destino.exists(), "Pasta de destino deve ser criada");
        let arquivos: Vec<_> = fs::read_dir(&destino)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(arquivos.len(), 2, "Devem existir 2 arquivos copiados");
        cleanup_dir(&base);
    }

    #[test]
    fn test_organizar_dry_run_nao_cria_pasta() {
        let base = tempdir();
        let desconhecido = base.path().join("Desconhecido");
        fs::create_dir_all(&desconhecido).unwrap();
        criar_arquivo_em(&desconhecido, "faixa1.mp3");

        organizar_album(
            base.path().to_str().unwrap(),
            "Artista DryRun",
            "Album DryRun",
            true,  // dry_run
            false,
            true,
            true,
        );

        let destino = base.path().join("Artista DryRun - Album DryRun");
        assert!(!destino.exists(), "Em dry-run a pasta nao deve ser criada");
        cleanup_dir(&base);
    }

    #[test]
    fn test_organizar_move_arquivos() {
        let base = tempdir();
        let desconhecido = base.path().join("Desconhecido");
        fs::create_dir_all(&desconhecido).unwrap();
        criar_arquivo_em(&desconhecido, "faixa1.mp3");

        organizar_album(
            base.path().to_str().unwrap(),
            "Artista Move",
            "Album Move",
            false,
            true, // mover
            true,
            true,
        );

        // Arquivo original deve ter sido removido da origem
        assert!(!desconhecido.join("faixa1.mp3").exists(), "Arquivo deve ter sido movido");
        cleanup_dir(&base);
    }

    // ─── renomear_musicas ─────────────────────────────────────────────────────

    #[test]
    fn test_renomear_pasta_inexistente_nao_panics() {
        renomear_musicas("/caminho/inexistente", "Artista", false);
    }

    #[test]
    fn test_renomear_dry_run_nao_altera_arquivos() {
        let dir = tempdir();
        criar_arquivo(&dir, "01 - musica.mp3");

        renomear_musicas(dir.path().to_str().unwrap(), "Novo Artista", true);

        assert!(dir.path().join("01 - musica.mp3").exists());
        cleanup_dir(&dir);
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn path(&self) -> &Path {
            &self.path
        }
    }

    fn tempdir() -> TempDir {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let path = std::env::temp_dir().join(format!("dandelion_test_{}", ts));
        fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }

    fn criar_arquivo(dir: &TempDir, nome: &str) {
        let mut f = File::create(dir.path().join(nome)).unwrap();
        f.write_all(b"dummy").unwrap();
    }

    fn criar_arquivo_em(dir: &Path, nome: &str) {
        let mut f = File::create(dir.join(nome)).unwrap();
        f.write_all(b"dummy").unwrap();
    }

    fn cleanup_dir(dir: &TempDir) {
        let _ = fs::remove_dir_all(dir.path());
    }
}
