# dbtool

[![Coverage Status](https://img.shields.io/badge/coverage-4%25-red)](https://github.com/gomesrocha/dbtool)

`dbtool` is a command-line interface (CLI) tool written in Rust for declarative management of PostgreSQL database schemas. Inspired by tools like Terraform and Ansible, it allows you to define the desired state of databases and tables in YAML playbooks, applying changes idempotently.

`dbtool` é uma ferramenta de linha de comando (CLI) escrita em Rust para gerenciamento declarativo de esquemas de banco de dados PostgreSQL. Inspirada em ferramentas como Terraform e Ansible, ela permite definir o estado desejado de bancos e tabelas em playbooks YAML, aplicando mudanças de forma idempotente.

---

<details>
<summary><strong>English Documentation</strong></summary>

## Features

- **Idempotency**: Checks for the existence of databases and tables before applying changes.
- **YAML Playbooks**: Define the desired state in YAML files, referencing SQL scripts.
- **Commands**:
  - `init`: Creates a basic YAML playbook.
  - `validate`: Validates the playbook syntax and the existence of SQL files.
  - `test`: Tests the database connection and checks for the existence of resources.
  - `plan`: Shows what would be done without applying any changes.
  - `apply`: Applies the changes defined in the playbook, with rollback support for tables.
  - `destroy`: Removes databases and tables defined in the playbook (use with caution).
- **Rollback**: Reverts partial changes to tables in case of errors (not applicable to databases).
- **Detailed Logs**: Records all actions, including executed SQL and errors.

## Prerequisites

- **Rust**: Version 1.56 or higher (to compile the project).
- **PostgreSQL**: Version 10 or higher, with a configured user (e.g., `postgres`).
- **Git**: To clone the repository.
- **Cargo**: Rust's package manager, included with the Rust installation.

## Installation

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/gomesrocha/dbtool.git
    cd dbtool
    ```

2.  **Compile the project**:
    ```bash
    cargo build --release
    ```
    The binary will be generated at `target/release/dbtool`.

3.  **Configure PostgreSQL**:
    - Ensure PostgreSQL is running:
      ```bash
      sudo systemctl start postgresql
      ```
    - Create a user and password (e.g., user `postgres`, password `postgres`):
      ```bash
      psql -U postgres -c "ALTER USER postgres WITH PASSWORD 'postgres';"
      ```

## Usage

### Playbook Structure
The playbook is a YAML file that defines the desired databases and tables. Example (`playbook.yml`):

```yaml
---
databases:
  - name: research
    if_not_exists: create_db.sql
tables:
  - database: research
    name: users
    if_not_exists: create_users_table.sql
```

Example `create_db.sql`:
```sql
CREATE DATABASE research;
```

Example `create_users_table.sql`:
```sql
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);
```

### Commands

1.  **Initialize a Playbook**:
    ```bash
    ./target/release/dbtool init --playbook my_playbook.yml
    ```

2.  **Validate a Playbook**:
    ```bash
    ./target/release/dbtool validate --playbook playbook.yml
    ```

3.  **Test Connection and Resources**:
    ```bash
    ./target/release/dbtool test --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

4.  **Plan Changes**:
    ```bash
    ./target/release/dbtool plan --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

5.  **Apply Changes**:
    ```bash
    ./target/release/dbtool apply --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

6.  **Destroy Resources**:
    **Warning**: This command deletes databases and tables.
    ```bash
    ./target/release/dbtool destroy --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

## Development & Testing

This project includes a suite of integration tests. To run the tests, you'll need a running PostgreSQL instance.

### Running Tests

Run all tests using the following command:
```bash
cargo test
```
**Note**: The tests that interact with the database require a running PostgreSQL instance accessible at `postgres://postgres:postgres@127.0.0.1:5432/postgres`. If the database is not available, these tests will fail.

### Test Coverage

To generate a test coverage report, you'll need `cargo-tarpaulin`.

1.  **Install `cargo-tarpaulin`**:
    ```bash
    cargo install cargo-tarpaulin
    ```

2.  **Generate the report**:
    ```bash
    cargo tarpaulin --out Lcov --output-dir . --ignore-tests
    ```
    This will generate an `lcov.info` file. The coverage badge in this README reflects the baseline coverage from tests that do not require a database. The actual coverage will be higher when the full test suite is run against a live database.

## Contributing

1.  Fork the repository.
2.  Create a branch for your feature (`git checkout -b my-feature`).
3.  Commit your changes (`git commit -m "Add my feature"`).
4.  Push to your branch (`git push origin my-feature`).
5.  Open a Pull Request on GitHub.

## License

This project is licensed under the [MIT License](LICENSE).

## Contact

For questions or suggestions, please open an issue at [https://github.com/gomesrocha/dbtool](https://github.com/gomesrocha/dbtool).

</details>

<details>
<summary><strong>Documentação em Português</strong></summary>

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

1.  **Clone o repositório**:
    ```bash
    git clone https://github.com/gomesrocha/dbtool.git
    cd dbtool
    ```

2.  **Compile o projeto**:
    ```bash
    cargo build --release
    ```
    O binário será gerado em `target/release/dbtool`.

3.  **Configure o PostgreSQL**:
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

1.  **Inicializar um Playbook**:
    ```bash
    ./target/release/dbtool init --playbook meu_playbook.yml
    ```

2.  **Validar um Playbook**:
    ```bash
    ./target/release/dbtool validate --playbook playbook.yml
    ```

3.  **Testar Conexão e Recursos**:
    ```bash
    ./target/release/dbtool test --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

4.  **Planejar Mudanças**:
    ```bash
    ./target/release/dbtool plan --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

5.  **Aplicar Mudanças**:
    ```bash
    ./target/release/dbtool apply --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

6.  **Destruir Recursos**:
    **Cuidado**: Este comando deleta bancos e tabelas.
    ```bash
    ./target/release/dbtool destroy --playbook playbook.yml --db-url postgres://postgres:postgres@localhost:5432/postgres
    ```

## Desenvolvimento e Testes

Este projeto inclui uma suíte de testes de integração. Para rodar os testes, você precisará de uma instância do PostgreSQL em execução.

### Rodando os Testes

Rode todos os testes com o seguinte comando:
```bash
cargo test
```
**Nota**: Os testes que interagem com o banco de dados requerem uma instância do PostgreSQL rodando e acessível em `postgres://postgres:postgres@127.0.0.1:5432/postgres`. Se o banco de dados não estiver disponível, esses testes irão falhar.

### Cobertura de Teste

Para gerar um relatório de cobertura de teste, você precisará do `cargo-tarpaulin`.

1.  **Instale o `cargo-tarpaulin`**:
    ```bash
    cargo install cargo-tarpaulin
    ```

2.  **Gere o relatório**:
    ```bash
    cargo tarpaulin --out Lcov --output-dir . --ignore-tests
    ```
    Isso irá gerar um arquivo `lcov.info`. O "badge" de cobertura neste README reflete a cobertura base dos testes que não requerem um banco de dados. A cobertura real será maior quando a suíte de testes completa for executada com um banco de dados ativo.

## Contribuindo

1.  Fork o repositório.
2.  Crie uma branch para sua feature (`git checkout -b minha-feature`).
3.  Commit suas alterações (`git commit -m "Adiciona minha feature"`).
4.  Faça push para sua branch (`git push origin minha-feature`).
5.  Abra um Pull Request no GitHub.

## Licença

Este projeto está licenciado sob a [MIT License](LICENSE).

## Contato

Para dúvidas ou sugestões, abra uma issue em [https://github.com/gomesrocha/dbtool](https://github.com/gomesrocha/dbtool).

</details>
