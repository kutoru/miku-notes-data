-- Add up migration script here

CREATE TABLE IF NOT EXISTS files (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    hash VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(250) NOT NULL,
    size BIGINT NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS files_hash ON files(hash);

CREATE TABLE IF NOT EXISTS tags (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    name VARCHAR(50) NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS notes (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    title VARCHAR(250) NOT NULL,
    text VARCHAR(50000) NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL,
    last_edited TIMESTAMP DEFAULT NOW() NOT NULL,
    times_edited INT DEFAULT 0 NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS note_files (
    note_id INT NOT NULL,
    file_id INT NOT NULL,
    FOREIGN KEY (note_id) REFERENCES notes(id),
    FOREIGN KEY (file_id) REFERENCES files(id),
    PRIMARY KEY (note_id, file_id)
);

CREATE TABLE IF NOT EXISTS note_tags (
    note_id INT NOT NULL,
    tag_id INT NOT NULL,
    FOREIGN KEY (note_id) REFERENCES notes(id),
    FOREIGN KEY (tag_id) REFERENCES tags(id),
    PRIMARY KEY (note_id, tag_id)
);

CREATE TABLE IF NOT EXISTS shelves (
    id SERIAL PRIMARY KEY,
    user_id INT UNIQUE NOT NULL,
    text VARCHAR(2500) NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL,
    last_edited TIMESTAMP DEFAULT NOW() NOT NULL,
    times_edited INT DEFAULT 0 NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS shelf_files (
    shelf_id INT NOT NULL,
    file_id INT NOT NULL,
    FOREIGN KEY (shelf_id) REFERENCES shelves(id),
    FOREIGN KEY (file_id) REFERENCES files(id),
    PRIMARY KEY (shelf_id, file_id)
);
