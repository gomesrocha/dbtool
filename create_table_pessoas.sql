CREATE TABLE pessoas (
    id SERIAL PRIMARY KEY,
    nome VARCHAR(100) NOT NULL,
    cargo VARCHAR(50),
    email VARCHAR(100) UNIQUE,
    data_contratacao DATE
);