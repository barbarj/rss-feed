use crate::Post;
use rusqlite::Connection;

pub struct Db {
    conn: Connection,
}
impl Db {
    pub fn build(conn: Connection) -> Result<Self, rusqlite::Error> {
        let mut db = Db { conn };
        let version = db.get_version().expect("Getting db version failed");
        match version {
            0 => db.migrate_v0_v1().expect("Migrating version 0 to 1 failed"),
            1 => db.migrate_v1_v2().expect("Migrating v1 to v2 failed"),
            2 => (),
            _ => panic!("Unknown db version found"),
        }
        let version = db.get_version().expect("Getting db version failed");

        assert_eq!(version, 2);

        Ok(db)
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
                (":date", &post.date.to_rfc3339()),
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

    // MIGRATIONS
    fn get_version(&self) -> Result<usize, rusqlite::Error> {
        self.conn
            .execute("CREATE TABLE IF NOT EXISTS _metadata(version INTEGER);", [])?;

        let version: Option<usize> = self
            .conn
            .prepare("SELECT version FROM _metadata ORDER BY version DESC LIMIT 1;")?
            .query_map([], |row| {
                let version: usize = row.get(0)?;
                Ok(version)
            })?
            .flatten()
            .next();

        Ok(version.unwrap_or(0))
    }

    fn migrate_v0_v1(&mut self) -> Result<(), rusqlite::Error> {
        let tx = self.conn.transaction()?;
        // create table
        let rows_changed = tx.execute(
            "CREATE TABLE IF NOT EXISTS posts( \
                link TEXT PRIMARY KEY, \
                title TEXT, \
                date TEXT, \
                author TEXT \
            );",
            [],
        )?;
        assert_eq!(rows_changed, 0);

        tx.execute("INSERT INTO _metadata(version) VALUES(1)", [])?;
        tx.commit()?;

        Ok(())
    }

    fn migrate_v1_v2(&mut self) -> Result<(), rusqlite::Error> {
        let tx = self.conn.transaction()?;
        // dedup on title + date + author
        let rows_to_be_deleted: Vec<(String, String)> = tx
            .prepare(
                "SELECT author, title FROM posts \
                WHERE ROWID NOT IN ( \
                    SELECT max(ROWID) \
                    FROM posts \
                    GROUP BY title, DATE(date), author \
                ); ",
            )?
            .query([])?
            .mapped(|r| Ok((r.get(0)?, r.get(1)?)))
            .flatten()
            .collect();
        println!("ROWS TO BE REMOVED:");
        for row in rows_to_be_deleted {
            println!("{} - '{}'", row.0, row.1);
        }

        let rows_changed = tx.execute(
            "DELETE FROM posts WHERE ROWID NOT IN ( \
                SELECT max(ROWID) \
                FROM posts \
                GROUP BY title, DATE(date), author \
            );",
            [],
        )?;
        tx.execute("INSERT INTO _metadata(version) VALUES(2);", [])?;
        tx.commit()?;
        println!("Duplicate rows removed: {rows_changed}");

        Ok(())
    }
}

// pub fn get_db_version(conn: &Connection) -> Result<usize, rusqlite::Error> {}
