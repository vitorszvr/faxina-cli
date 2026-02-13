# Histórico de Mudanças

Todas as alterações notáveis neste projeto serão documentadas neste arquivo.

## [0.2.0] - 2026-02-12

### 🚀 Novidades

- **Performance**: Varredura de arquivos em paralelo usando `jwalk` (substituindo `walkdir`), garantindo muito mais velocidade em discos grandes.
- **UX**: Feedback visual aprimorado com contador de arquivos em tempo real durante a varredura.
- **Instalação**: Novo script `install.sh` que detecta automaticamente o sistema operacional e arquitetura para baixar o binário correto.

### 🏗 Arquitetura

- **Extensibilidade**: Novo sistema de plugins baseado em Traits (`src/projects`), facilitando a adição de suporte a novas linguagens e frameworks.
- **Organização**: Código refatorado e dividido em módulos claros: `scanner`, `cleaner`, `display`, `projects` e `types`.

### 🛡 Robustez

- **Tratamento de Erros**: Migração completa para a biblioteca `anyhow`, proporcionando mensagens de erro mais claras e tratamento consistente.
- **Testes**: Implementação de testes de integração nativos em Rust (`tests/cli.rs`) para garantir a qualidade e portabilidade do binário.

### 📦 Dependências

- Novas: `jwalk` (paralelismo), `anyhow` (erros).
- Desenvolvimento: `assert_cmd`, `predicates`, `tempfile` (para testes E2E).
