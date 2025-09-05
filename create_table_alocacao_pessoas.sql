CREATE TABLE alocacao_pessoas (
    id SERIAL PRIMARY KEY,
    pessoa_id INTEGER NOT NULL REFERENCES pessoas(id) ON DELETE CASCADE,
    projeto_id INTEGER NOT NULL REFERENCES projetos(id) ON DELETE CASCADE,
    tarefa_id INTEGER REFERENCES tarefas(id) ON DELETE SET NULL,
    horas_alocadas INTEGER DEFAULT 0,
    data_inicio DATE,
    data_fim DATE,
    papel VARCHAR(50)
);