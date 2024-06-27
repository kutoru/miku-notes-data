DROP TABLE IF EXISTS note_tags;
DROP TABLE IF EXISTS note_files;
DROP TABLE IF EXISTS notes;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS files;
DROP TABLE IF EXISTS users;

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email TEXT NOT NULL,
    password TEXT NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL
);

CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    hash VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(256) NOT NULL,
    size BIGINT NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX files_hash ON files(hash);

CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    name VARCHAR(64) NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE notes (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    title VARCHAR(1024) NOT NULL,
    text VARCHAR(65536) NOT NULL,
    created TIMESTAMP DEFAULT NOW() NOT NULL,
    last_edited TIMESTAMP DEFAULT NOW() NOT NULL,
    times_edited INT DEFAULT 0 NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE note_files (
    note_id INT NOT NULL,
    file_id INT NOT NULL,
    FOREIGN KEY (note_id) REFERENCES notes(id),
    FOREIGN KEY (file_id) REFERENCES files(id),
    PRIMARY KEY (note_id, file_id)
);

CREATE TABLE note_tags (
    note_id INT NOT NULL,
    tag_id INT NOT NULL,
    FOREIGN KEY (note_id) REFERENCES notes(id),
    FOREIGN KEY (tag_id) REFERENCES tags(id),
    PRIMARY KEY (note_id, tag_id)
);
