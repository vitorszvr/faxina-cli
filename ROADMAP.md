# Roadmap e Melhorias - Faxina CLI

Este documento centraliza as sugestões de melhoria e o planejamento para as próximas versões do projeto.

Prioridades: **Curto prazo = v0.3.0** (segurança/robustez), **Médio prazo = v0.4.0** (UX/performance), **Longo prazo** (infra/assinatura).

## 🎯 Próximos Passos (v0.3.0)

Foco em **Segurança** e **Robustez**, especialmente para ambientes Windows.

### 🛡️ 1. Segurança e Validação

Prevenir deleções acidentais em diretórios críticos do sistema.

- [x] Criar lista de `PROTECTED_PATHS` (ex: `/`, `/usr`, `C:\`, `C:\Windows`).
- [x] Implementar verificação `is_safe_to_scan(path)` antes de iniciar qualquer operação.

### 🔄 2. Robustez no Windows

O Windows bloqueia arquivos em uso (antivírus, indexação, terminais abertos), o que pode fazer a limpeza falhar.

- [x] Implementar **Retry Logic** na remoção de diretórios (`remove_with_retry`).
- [x] Adicionar backoff exponencial (esperar um pouco antes de tentar de novo).

### ⚙️ 3. Configuração Persistente

Permitir que o usuário salve suas preferências padrão.

- [x] Suporte a arquivo de configuração global (`~/.faxina-config.toml` ou similar).
- [x] Opções suportadas:
  - `days` (padrão de dias)
  - `auto_confirm` (para não pedir `y/N` sempre)
  - `excluded_dirs` (pastas para nunca escanear)

---

## 🔮 Futuro (v0.4.0+)

Foco em **Experiência do Usuário (UX)** e **Performance**.

### 📊 4. Estatísticas e Relatórios (médio)

- [x] Flag `--stats` para mostrar resumo por linguagem (ex: "Rust: 2GB", "Node: 500MB").
- [ ] Identificar e listar qual é o projeto mais antigo/pesado. (prioridade: média)

### ⚡ 5. Performance Aprimorada (médio)

- [ ] Otimizar o cálculo de tamanho (`dir_size`) para diretórios gigantes (amostragem ou `metadata` mais leve). (prioridade: média)
- [ ] Evitar re-scan de projetos aninhados (detectar se um projeto está dentro de outro já listado). (prioridade: média)

### 🎨 6. UX Polish

- [x] Ícones específicos por linguagem no terminal (🦀 para Rust, 📦 para Node, etc).
- [ ] Modo interativo de seleção (`dialoguer::MultiSelect`): permitir selecionar quais projetos limpar de uma lista. (prioridade: média)

---

## 📦 Infraestrutura e CI/CD (longo prazo)

- [x] **Checksums**: gerar SHA256 dos artefatos de release `.zip` e `.msi` — (implementado no workflow Windows).
- [ ] **Assinatura de Código**: adquirir certificado para assinar binários Windows e remover aviso do SmartScreen nativamente.

Ações recomendadas:

- Priorizar testes automáticos de release (verificar que MSI contém `License.rtf`).
- Criar tickets para os itens marcados como média/prioridade e estimar esforço.
