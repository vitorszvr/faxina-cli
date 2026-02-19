# 🧹 Faxina CLI — Lixeiro Inteligente de Projetos

CLI em Rust que varre seus diretórios de projetos e remove automaticamente pastas de dependências de projetos inativos, liberando espaço em disco sem tocar no código-fonte.

## O Problema

Desenvolvedores acumulam pastas pesadas de dependências (`node_modules`, `target/`, `venv/`, etc.) em projetos que não tocam há meses. Essas pastas podem ocupar **gigabytes** de espaço, mas são totalmente reconstruíveis com um simples `npm install`, `cargo build` ou `pip install`.

## Como Funciona

1. **Varre** recursivamente um diretório à procura de projetos
2. **Identifica** projetos inativos (sem modificação há N dias)
3. **Remove** apenas as pastas de dependência, preservando todo o código-fonte
4. **Reporta** quanto espaço foi liberado

## Tipos de Projeto Suportados

| Linguagem   | Pasta Detectada     | Identificador                        |
| ----------- | ------------------- | ------------------------------------ |
| Node.js     | `node_modules/`     | `package.json` no diretório pai      |
| Rust        | `target/`           | `Cargo.toml` no diretório pai        |
| Next.js     | `.next/`            | `package.json` ou `next.config.*`    |
| Python      | `venv/` ou `.venv/` | `pyvenv.cfg` ou `bin/python` dentro  |
| Go          | `vendor/`           | `go.mod` no diretório pai            |
| Java/Gradle | `build/`            | `build.gradle` ou `build.gradle.kts` |

## Instalação

### Windows (Comando Único)

Abra o PowerShell (não o CMD) e rode:

```powershell
iwr -useb https://raw.githubusercontent.com/vitorszvr/faxina-cli/master/install.ps1 | iex
```

### Windows (Instalador MSI)

Alternativamente, baixe e instale o arquivo `faxina-cli.msi` na aba [Releases](https://github.com/vitorszvr/faxina-cli/releases).

O script irá:

- Desbloquear o executável (evitando aviso do SmartScreen)
- Instalar o executável em `%LOCALAPPDATA%\faxina-cli`
- Adicionar o diretório ao seu PATH

### Linux / macOS (Script Automático)

```bash
curl -fsSL https://raw.githubusercontent.com/vitorszvr/faxina-cli/master/install.sh | bash
```

### Compilar manualmente

```bash
git clone https://github.com/vitorszvr/faxina-cli.git
cd faxina-cli
cargo build --release

# O binário estará em target/release/faxina-cli
```

## Uso

```bash
# Varrer o diretório atual (projetos inativos há 30+ dias)
faxina-cli

# Varrer um diretório específico
faxina-cli ~/Projetos

# Modo Interativo (Selecione quais projetos limpar)
faxina-cli ~/Projetos --interactive

# Apenas exibir estatísticas (projeto mais pesado, mais antigo)
faxina-cli ~/Projetos --stats

# Ignorar pastas específicas
faxina-cli ~/Projetos --excluded-dirs "lixo,temp,backup"

# Alterar o limite de dias de inatividade
faxina-cli ~/Projetos --days 60

# Simulação (não deleta nada, só mostra o que faria)
faxina-cli ~/Projetos --dry-run

# Pular confirmação interativa
faxina-cli ~/Projetos --yes

# Mostrar caminhos completos durante limpeza
faxina-cli ~/Projetos --verbose

# Saída mínima (só o total liberado — útil para scripts)
faxina-cli ~/Projetos --quiet --yes
```

## Monorepos e Projetos Aninhados

O **Faxina CLI** possui proteção inteligente para monorepos e projetos aninhados:

1. **Proteção de Filhos**: Se um projeto pai (ex: Monorepo) estiver **ativo** (modificado recentemente), todos os seus subprojetos (ex: `packages/*`) serão preservados, mesmo que não tenham sido tocados. Isso evita quebrar o ambiente de desenvolvimento do monorepo.
2. **Proteção de Pais**: Se um subprojeto estiver **ativo**, o projeto pai também será preservado.

Isso garante que dependências compartilhadas ou ferramentas de build no nível da raiz não sejam deletadas enquanto você trabalha em um subprojeto específico.

```

## Flags

| Flag              | Curta | Descrição                                       |
| ----------------- | ----- | ----------------------------------------------- |
| `--days <N>`      | `-d`  | Dias de inatividade (padrão: 30)                |
| `--dry-run`       |       | Simular sem deletar                             |
| `--yes`           | `-y`  | Pular confirmação                               |
| `--interactive`   | `-i`  | Modo interativo (escolher projetos para limpar) |
| `--stats`         |       | Exibir apenas estatísticas (tamanho, idade)     |
| `--excluded-dirs` |       | Lista de pastas a ignorar (ex: `ignored,tmp`)   |
| `--verbose`       | `-v`  | Mostrar caminhos completos                      |
| `--quiet`         | `-q`  | Saída mínima                                    |

## Exemplo de Saída

```

🧹 Faxina CLI — Lixeiro Inteligente de Projetos
─────────────────────────────────────────────

📦 3 projetos inativos encontrados (3 pastas, 15.0 MB)

▸ meu-projeto-rust
📂 /home/user/Projetos/meu-projeto-rust
🕐 Última modificação: 45 dias atrás
🦀 target 10.0 MB

▸ meu-site-next
📂 /home/user/Projetos/meu-site-next
🕐 Última modificação: 60 dias atrás
📦 node_modules 4.8 MB
▲ .next 200.0 KB

🗑️ Deseja remover essas pastas de dependência? (y/N)

```

## Arquitetura

```

src/
├── main.rs → CLI (clap), validação de args, orquestração
├── scanner.rs → Varredura de projetos, detecção de deps, cálculo de mtime
├── cleaner.rs → Deleção de pastas com barra de progresso
└── display.rs → Formatação de output, cores, confirmação

```

## Segurança

- **Nunca** toca em arquivos de código-fonte
- **Bloqueia** varredura em diretórios críticos do sistema (ex: `/`, `C:\`, `/usr`)
- Modo `--dry-run` para simular antes de agir
- Confirmação interativa por padrão
- Não segue symlinks (previne deleção acidental fora do escopo)
- Erros individuais não param o processo — são reportados no final

## Licença

GPL-3.0
```
