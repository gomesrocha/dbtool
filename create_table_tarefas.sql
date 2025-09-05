CREATE TABLE tarefas (
    id SERIAL PRIMARY KEY,
    projeto_id INTEGER NOT NULL REFERENCES projetos(id) ON DELETE CASCADE,
    nome VARCHAR(100) NOT NULL,
    descricao TEXT,
    data_inicio DATE,
    data_fim DATE,
    status VARCHAR(20) DEFAULT 'Pendente',
    prioridade INTEGER DEFAULT 1
);