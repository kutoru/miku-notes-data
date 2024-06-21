INSERT INTO users (email, password) VALUES
('asdfjk', '34'),
('kuromix', '35'),
('nexochan', '32');

INSERT INTO notes (user_id, title, text) VALUES
(1, 'slkdfjasldjf', 'as ldfjaw o3efj3w ijfas'),
(2, 'osu!', 'is bad i guess'),
(2, 'gacha', 'is also bad i guess'),
(3, 'gaming', 'i cant stop playing ds games help me please');

INSERT INTO tags (user_id, name) VALUES
(2, 'truth'),
(2, 'lies');

INSERT INTO note_tags (note_id, tag_id) VALUES
(2, 2);

INSERT INTO files (user_id, hash, ext, name, size) VALUES
(2, 'filjs3af8was3fj83', '.jpg', 'ss.jpg', 389432),
(2, 'fiasl3j8f32992asd', '.txt', 'rem.txt', 923);

INSERT INTO note_files (note_id, file_id) VALUES
(2, 1),
(3, 2);
