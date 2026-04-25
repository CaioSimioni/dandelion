# Dandelion

Um gerenciador de músicas para organizar seus arquivos de forma automática e eficiente.

**@author**: CaioSimioni  
**@last_update**: 25/04/2026

---

## O Problema

Quando você baixa músicas de um álbum, elas ficam todas soltas na pasta `Desconhecido` com apenas o título da música no nome do arquivo.

Para organizá-las você precisa:
1. Ver quais músicas estão na pasta
2. Criar manualmente uma pasta `<Artista> - <Album>`
3. Copiar/mover todos os arquivos
4. **Decidir a ordem correta das músicas**
5. Renomear **um por um** no formato `01 - Artista - Nome da Musica.mp3`

Os passos 4 e 5 são **extremamente tediosos**.

## A Solução

Dandelion faz tudo isso automaticamente com um único comando, **permitindo que você escolha a ordem das músicas de forma interativa**.

---

## Instalação

```bash
git clone https://github.com/CaioSimioni/dandelion.git
cd dandelion
cargo build --release
cp target/release/dandelion ~/.local/bin/
```

