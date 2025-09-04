# dbtool

`dbtool` é uma ferramenta de linha de comando (CLI) escrita em Rust para gerenciamento declarativo de esquemas de banco de dados PostgreSQL. Inspirada em ferramentas como Terraform e Ansible, ela permite definir o estado desejado de bancos e tabelas em playbooks YAML, aplicando mudanças de forma idempotente. Suporta verificação de existência, criação, destruição e validação de recursos, com suporte a rollback para tabelas em caso de erros.

## Funcionalidades

- **Idempotência**: Verifica a existência de bancos e tabelas antes de aplicar mudanças.
- **Playbooks YAML**: Define o estado desejado em arquivos YAML, referenciando scripts SQL.
- **Comandos**:
  - `init`: Cria um playbook YAML básico.
  - `validate`: Valida a sintaxe do playbook e a existência dos arquivos SQL.
  - `test`: Testa a conexão com o banco e verifica a existência de recursos.
  - `plan`: Mostra o que será feito sem aplicar mudanças.
  - `apply`: Aplica as mudanças definidas no playbook, com suporte a rollback para tabelas.
  - `destroy`: Remove bancos e tabelas definidos no playbook (use com cuidado).
- **Rollback**: Reverte mudanças parciais em tabelas em caso de erros (exceto para bancos).
- **Logs Detalhados**: Registra todas as ações, incluindo SQL executado e erros.

## Pré-requisitos

- **Rust**: Versão 1.56 ou superior (para compilar o projeto).
- **PostgreSQL**: Versão 10 ou superior, com um usuário configurado (ex: `postgres`).
- **Git**: Para clonar o repositório.
- **Cargo**: Gerenciador de pacotes do Rust, incluído com a instalação do Rust.

## Instalação

1. **Clone o repositório**:
   ```bash
   git clone https://github.com/gomesrocha/dbtool.git
   cd dbtool
   ```

2. **Compile o projeto**:
   ```bash
   cargo build --release
   ```

   O binário será gerado em `target/release/dbtool`.

3. **Configure o PostgreSQL**:
   - Certifique-se de que o PostgreSQL está rodando:
     ```bash
     sudo systemctl start postgresql
     ```
   - Crie um usuário e senha (ex: usuário `postgres`, senha `postgres`):
     ```bash
     psql -U postgres -c "ALTER USER postgres WITH PASSWORD 'postgres';"
     ```

## Uso

### Estrutura do Playbook
O playbook é um arquivo YAML que define bancos e tabelas desejados. Exemplo (`playbook.yml`):

```yaml
---
databases:
  - name: pesquisa
    if_not_exists: create_db.sql
tables:
  - database: pesquisa
    name: users
    if_not_exists: create_users_table.sql
```

Exemplo de `create_db.sql`:
```sql
CREATE DATABASE pesquisa;
```

Exemplo de `create_users_table.sql`:
```sql
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);
```

### Comandos

1. **Inicializar um Playbook**:
   ```bash
   ./target/release/dbtool init --playbook meu_playbook.yml
   ```
   Cria um playbook básico (`meu_playbook.yml`).

2. **Validar um Playbook**:
   ```bash
   ./target/release/dbtool validate --playbook playbook.yml
   ```
   Verifica a sintaxe do YAML e a existência dos arquivos SQL.

3. **Testar Conexão e Recursos**:
   ```bash
   ./target/release/dbtool test --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
   ```
   Testa a conexão com o banco e verifica a existência de bancos/tabelas.

4. **Planejar Mudanças**:
   ```bash
   ./target/release/dbtool plan --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
   ```
   Mostra o que seria feito sem aplicar mudanças.

5. **Aplicar Mudanças**:
   ```bash
   ./target/release/dbtool apply --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
   ```
   Aplica o estado definido no playbook. Use `--no-rollback true` para desativar rollback.

6. **Destruir Recursos**:
   **Cuidado**: Este comando deleta bancos e tabelas.
   ```bash
   ./target/release/dbtool destroy --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
   ```

### Exemplo Completo

1. Crie um playbook (`playbook.yml`):
   ```yaml
   ---
   databases:
     - name: pesquisa
       if_not_exists: create_db.sql
   tables:
     - database: pesquisa
       name: users
       if_not_exists: create_users_table.sql
   ```

2. Crie os arquivos SQL:
   ```bash
   echo "CREATE DATABASE pesquisa;" > create_db.sql
   echo "CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name VARCHAR(255) NOT NULL);" > create_users_table.sql
   ```

3. Valide o playbook:
   ```bash
   ./target/release/dbtool validate --playbook playbook.yml
   ```

4. Aplique as mudanças:
   ```bash
   ./target/release/dbtool apply --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
   ```

## Contribuindo

1. Fork o repositório.
2. Crie uma branch para sua feature:
   ```bash
   git checkout -b minha-feature
   ```
3. Commit suas alterações:
   ```bash
   git commit -m "Adiciona minha feature"
   ```
4. Faça push para sua branch:
   ```bash
   git push origin minha-feature
   ```
5. Abra um Pull Request no GitHub.

## Licença

Este projeto está licenciado sob a [MIT License](LICENSE).

## Contato

Para dúvidas ou sugestões, abra uma issue em [https://github.com/gomesrocha/dbtool](https://github.com/gomesrocha/dbtool).