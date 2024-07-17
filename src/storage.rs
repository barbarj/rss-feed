use crate::Post;
use rusqlite::Connection;

const DB_VERSION: usize = 1;

pub struct Db {
    conn: Connection,
}
impl Db {
    pub fn build(conn: Connection) -> Result<Self, rusqlite::Error> {
        let db = Db { conn };
        // TODO: migrate version
        db.idempotently_create_posts_table()?;
        Ok(db)
    }

    fn idempotently_create_posts_table(&self) -> Result<(), rusqlite::Error> {
        let rows_changed = self.conn.execute(
            "CREATE TABLE IF NOT EXISTS posts( \
                link TEXT PRIMARY KEY, \
                title TEXT, \
                date TEXT, \
                author TEXT \
            );",
            [],
        )?;
        assert_eq!(rows_changed, 0);
        Ok(())
    }

    pub fn upsert_posts(
        &mut self,
        posts: impl Iterator<Item = Post>,
    ) -> Result<usize, rusqlite::Error> {
        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare(
            "INSERT INTO posts(link, title, date, author) \
                            VALUES(:link, :title, :date, :author) \
                            ON CONFLICT(link) DO NOTHING;",
        )?;

        let mut rows_affected = 0;

        for post in posts {
            rows_affected += stmt.execute(&[
                (":link", &post.link),
                (":title", &post.title),
                (":date", &post.date.to_string()),
                (":author", &post.author),
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(rows_affected)
    }

    pub fn fetch_all_posts(&self) -> Result<Vec<Post>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT link, title, date, author FROM posts ORDER BY date DESC;")?;

        let posts = stmt
            .query([])?
            .mapped(|row| {
                let d: String = row.get(2)?;
                let date = Post::parse_stored_date(&d).expect("Parsing stored date failed");
                let item = Post {
                    link: row.get(0)?,
                    title: row.get(1)?,
                    date,
                    author: row.get(3)?,
                };
                Ok(item)
            })
            .flatten()
            .collect();
        Ok(posts)
    }
}

// pub fn get_db_version(conn: &Connection) -> Result<usize, rusqlite::Error> {}
