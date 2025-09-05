CREATE TABLE projetos (
    id SERIAL PRIMARY KEY,
    nome VARCHAR(100) NOT NULL,
    descricao TEXT,
    data_inicio DATE NOT NULL,
    data_fim DATE,
    status VARCHAR(20) DEFAULT 'Em andamento'
);