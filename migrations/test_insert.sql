-- INSERT INTO users (email, pass_hash) VALUES
-- ('asdfjk@gmail.com', '$2a$12$pDD1LIsbG./RQaNZjkrKWOOdDM4EzArm0Fu92j4wfB1UcXZM/x7ru'),
-- ('kuromix@mail.ru', '$2a$12$VRv4DVkfPeYYlMM5HQAMQua1uHFjgw3JYiEGSG8gFttwqTarPaZiC'),
-- ('nexochan@mail.ru', '$2a$10$8rYX3btoXa02PxerJnv/LeK3/gyRNJfq3vQVNCuMkzlblILluTf.u');
-- the passes are 1234

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

INSERT INTO files (user_id, hash, name, size) VALUES
(2, 'filjs3af8was3fj83', 'ss.jpg', 389432),
(3, 'aio8fjasofj3woo8a', 'aaaaa.mp3', 29356),
(2, 'fiasl3j8f32992asd', 'rem.txt', 923),
(2, 'asldkfbqawi3fj8l8', 'osu!install.exe', 238472),
(2, 'qwoitueqweoitwoei', 'important.png', 2938523);

INSERT INTO note_files (note_id, file_id) VALUES
(2, 1),
(4, 2),
(3, 3),
(2, 4);

INSERT INTO shelves (user_id, text) VALUES
(1, ''),
(2, ''),
(3, '');

INSERT INTO shelf_files (shelf_id, file_id) VALUES
(2, 5);
